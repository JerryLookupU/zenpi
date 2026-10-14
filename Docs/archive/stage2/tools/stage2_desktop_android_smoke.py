#!/usr/bin/env python3
"""Real desktop PTY QR + shared owner transcript update observed by Android.

Defaults to adb reverse; --lan-ip uses a private host interface without forwarding.
macOS Vision independently decodes the production terminal QR. No invite is saved.
An emulator on a virtual network does not prove camera or physical LAN peer support.
"""
from __future__ import annotations
import argparse
import base64
import fcntl
import hashlib
import ipaddress
import json
import os
import pty
import re
import shlex
import socket
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit
from stage2_android_pair_smoke import ROOT, seed_session, run_instrumentation, serialized_device_validation, InstrumentationFailure, connect_emulator_wifi
from stage2_android_build import require_current_build
from stage2_host_build import require_current_host
from stage2_instrumentation import require_test_results
from tui_project_workspace_smoke import Terminal, screen_text
from tui_user_shell_smoke import drain

BASE_INSTRUMENTATION_CLASSES = (
    "zenpi.HostPairingTest", "zenpi.NetworkLifecycleTest", "zenpi.PairingScreenTest",
    "zenpi.ResumeStoreTest", "zenpi.ForegroundStoreTest", "zenpi.DeviceRevokeTest", "zenpi.LocalResetTest", "zenpi.OwnerCallbackDeviceTest", "zenpi.PairingCancelTest", "zenpi.LanSocketRoutingTest",
)
WORKSPACE_INSTRUMENTATION_CLASSES = (
    "zenpi.WorkspaceModelTest", "zenpi.WorkspaceScreenTest", "zenpi.SubtabScreenTest", "zenpi.ResumeWorkerTest",
    "zenpi.TabCapacityScreenTest", "zenpi.PromptScreenTest", "zenpi.ApprovalScreenTest",
    "zenpi.WorkspaceFilesScreenTest", "zenpi.TerminalControlScreenTest", "zenpi.TerminalHistoryScreenTest",
    "zenpi.WorkerScreenTest", "zenpi.WorkerStatusPageScreenTest",
    "zenpi.ResourceCatalogScreenTest", "zenpi.ModelStatusScreenTest", "zenpi.DiffScreenTest", "zenpi.TranscriptToolScreenTest",
)
BASE_INSTRUMENTATION_TESTS = 39
WORKSPACE_INSTRUMENTATION_TESTS = 53


def instrumentation_suite(workspace_control=False, doze=False):
    classes = BASE_INSTRUMENTATION_CLASSES + (WORKSPACE_INSTRUMENTATION_CLASSES if workspace_control else ())
    return classes + (("zenpi.DozeLifecycleTest",) if doze else ()), \
        BASE_INSTRUMENTATION_TESTS + (WORKSPACE_INSTRUMENTATION_TESTS if workspace_control else 0) + int(doze)


def private_lan_ip(value):
    try:
        address = ipaddress.ip_address(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("--lan-ip requires a literal private LAN address") from error
    if getattr(address, "scope_id", None) is not None:
        raise argparse.ArgumentTypeError("--lan-ip must not include an IPv6 interface scope")
    private = (address.version == 4 and any(address in ipaddress.ip_network(network) for network in ("10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"))) or (address.version == 6 and address in ipaddress.ip_network("fc00::/7"))
    if not private or address.is_unspecified or address.is_loopback or address.is_link_local or address.is_multicast:
        raise argparse.ArgumentTypeError("--lan-ip must be a private, non-loopback LAN address")
    return address.compressed


def require_no_reverse(adb):
    result = subprocess.run(adb + ["reverse", "--list"], capture_output=True, text=True, check=True, timeout=10)
    if result.stdout.strip():
        raise RuntimeError("Direct IP validation requires an empty adb reverse table; existing mappings are not removed")


def force_stop_live_android(adb):
    live = subprocess.run(adb + ["shell", "pidof", "app.zenpi.android"], capture_output=True, text=True, timeout=10)
    if live.returncode != 0 or not re.fullmatch(r"[0-9]+", live.stdout.strip()):
        raise RuntimeError("Expected exactly one live Android process before force-stop")
    pid = live.stdout.strip()
    subprocess.run(adb + ["shell", "am", "force-stop", "app.zenpi.android"], check=True, timeout=10)
    remaining = subprocess.run(adb + ["shell", "pidof", "app.zenpi.android"], capture_output=True, text=True, timeout=10)
    if remaining.returncode not in (0, 1) or remaining.stdout.strip():
        raise RuntimeError("Android process death could not be proved")
    return pid


def prepare_emulator_host_network(adb):
    require_no_reverse(adb)
    return connect_emulator_wifi(adb)


def process_recovery_evidence(output, metadata_rehydrated=False):
    phases = re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=(network_process_cursor_replay_verified|network_process_tool_metadata_rehydrated_verified)$", output)
    expected = "network_process_tool_metadata_rehydrated_verified" if metadata_rehydrated else "network_process_cursor_replay_verified"
    before = re.findall(r"(?m)^INSTRUMENTATION_STATUS: network_process_seed_cursor=(.*)$", output)
    after = re.findall(r"(?m)^INSTRUMENTATION_STATUS: network_process_recovered_cursor=(.*)$", output)
    if phases != [expected] or len(before) != 1 or len(after) != 1:
        return False
    if any(not re.fullmatch(r"0|[1-9][0-9]{0,19}", value) or int(value) > 18446744073709551615 for value in before + after):
        return False
    return int(after[0]) > int(before[0])


DIFF_PAGE_PHASES = ("worktree_diff_pages_requested", "worktree_diff_pages_verified", "worktree_diff_changed_verified")


def diff_page_workflow_evidence(phases, patch_sha256, total_bytes, output):
    if (phases != list(DIFF_PAGE_PHASES) or not isinstance(patch_sha256, str)
            or not re.fullmatch(r"[0-9a-f]{64}", patch_sha256)
            or type(total_bytes) is not int or not 65536 < total_bytes <= 1048576 or not isinstance(output, str)):
        return False
    return (re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=(worktree_diff_pages_requested|worktree_diff_pages_verified|worktree_diff_changed_verified)$", output) == list(DIFF_PAGE_PHASES)
            and re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_diff_patch_sha256=(.*)$", output) == [patch_sha256]
            and re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_diff_total_bytes=(.*)$", output) == [str(total_bytes)])


def approval_evidence(receipts, events, device_id, project_id, session_id):
    resolved = [event for event in events if event.get("type") == "approval_resolved" and event.get("call_id") in ("approval-allow", "approval-deny")]
    if len(receipts) != 2 or [r.get("decision") for r in receipts] != ["allow", "deny"] or [e.get("decision") for e in resolved] != ["allow", "deny"]:
        return False
    for receipt, event in zip(receipts, resolved):
        binding = receipt.get("binding", {})
        if any(binding.get(field) != value for field, value in (("state", "accepted"), ("device_id", device_id), ("project_id", project_id), ("session_id", session_id), ("target_request_id", event.get("request_id")))):
            return False
        if binding.get("code") is not None or event.get("remember") is not False or event.get("source") != "host_answer":
            return False
        if not re.fullmatch(r"[0-9a-f]{64}", receipt.get("digest", "")) or not re.fullmatch(r"[0-9a-f]{64}", binding.get("fingerprint", "")):
            return False
        if not re.fullmatch(r"[0-9a-f-]{36}", binding.get("request_id", "")):
            return False
        if event.get("remote") != {field: binding.get(field) for field in ("device_id", "request_id", "fingerprint", "project_id", "session_id")} | {"request_digest":receipt.get("digest")}:
            return False
    consumed = [e for e in events if e.get("type") == "approval_consumed" and e.get("call_id") in ("approval-allow", "approval-deny")]
    return len(consumed) == 1 and consumed[0].get("call_id") == "approval-allow" and consumed[0].get("decision") == "allow" and consumed[0].get("request_id") == resolved[0].get("request_id")


def steering_evidence(receipts, events, requests, device_id, project_id, session_id):
    if len(receipts) != 1:
        return False
    receipt = receipts[0]
    if set(receipt) != {"device_id", "request_id", "fingerprint", "project_id", "session_id", "target_request_id", "state", "code"}:
        return False
    if (receipt["device_id"], receipt["project_id"], receipt["session_id"], receipt["state"], receipt["code"]) != (device_id, project_id, session_id, "accepted", None):
        return False
    if not re.fullmatch(r"[0-9a-f]{64}", receipt["fingerprint"]):
        return False
    input_id = hashlib.sha256(f"remote-steer-v1\n{device_id}\n{receipt['request_id']}".encode()).hexdigest()
    enqueued = [event for event in events if event.get("type") == "input_queue" and event.get("change", {}).get("action") == "enqueued"]
    applied = [event for event in events if event.get("type") == "input_queue" and event.get("change", {}).get("action") == "applied"]
    if len(enqueued) != 1 or len(applied) != 1:
        return False
    queued = enqueued[0]["change"]["input"]
    if (queued.get("id"), queued.get("kind"), queued.get("text")) != (input_id, "steer", "STAGE2_ANDROID_PROMPT_STEER"):
        return False
    if applied[0]["change"].get("ids") != [input_id] or applied[0]["change"].get("kind") != "steer":
        return False
    original = [event for event in events if event.get("type") == "remote_prompt_admission_v1"
        and event.get("state") == "accepted" and event.get("request_id") == receipt["target_request_id"]]
    if len(original) != 1 or original[0].get("device_id") != device_id:
        return False
    contexts = [request for request in requests if any("STAGE2_ANDROID_PROMPT_STEER" in item.get("content", "")
        for item in request.get("input", []) if item.get("role") == "user" and isinstance(item.get("content"), str))]
    # Responses API may encode message content as structured content parts.
    if not contexts:
        contexts = [request for request in requests if "STAGE2_ANDROID_PROMPT_STEER" in json.dumps(
            [item for item in request.get("input", []) if item.get("role") == "user"])]
    return bool(contexts) and all("STAGE2_ANDROID_PROMPT_REQUEST" in json.dumps(request.get("input", [])) for request in contexts)


def prompt_binding_evidence(bindings, events, device_id, project_id, session_id, expected_count=3):
    bindings = [binding for binding in bindings if binding.get("operation", "prompt") == "prompt"]
    admissions = [event for event in events if event.get("type") == "remote_prompt_admission_v1" and event.get("state") == "accepted"]
    if len(bindings) != expected_count or len(admissions) != expected_count:
        return False
    identities = set()
    for binding in bindings:
        if set(binding) != {"device_id", "request_id", "fingerprint", "project_id", "session_id"}:
            return False
        if (binding["device_id"], binding["project_id"], binding["session_id"]) != (device_id, project_id, session_id):
            return False
        if not re.fullmatch(r"[0-9a-f]{64}", binding["fingerprint"]):
            return False
        identity = (binding["device_id"], binding["request_id"])
        if identity in identities:
            return False
        identities.add(identity)
        matches = [event for event in admissions if (event.get("device_id"), event.get("request_id")) == identity]
        if len(matches) != 1 or matches[0].get("fingerprint") != binding["fingerprint"]:
            return False
    return True


def terminal_open_evidence(bindings, events, workspace, device, project):
    selected = [binding for binding in bindings if binding.get("operation") in ("terminal_open", "terminal_input")]
    receipts = [event for event in events if event.get("type") == "remote_terminal_admission_v1"]
    operations = ("terminal_open", "terminal_input", "terminal_input", "terminal_open", "terminal_input")
    initial_close = len(selected) == 6
    if initial_close: operations = ("terminal_input", *operations)
    if len(selected) != len(operations) or [event.get("state") for event in receipts] != ["reserved", "succeeded"] * len(operations):
        return False
    identities = [binding.get("request_id") for binding in selected]
    if any(not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", value) for value in identities) or len(set(identities)) != len(operations):
        return False
    terminals = []
    scopes = set()
    for index, (operation, binding) in enumerate(zip(operations, selected)):
        pair = receipts[index * 2:index * 2 + 2]
        if binding.get("operation") != operation or binding.get("device_id") != device or binding.get("project_id") != project:
            return False
        if not isinstance(binding.get("fingerprint"), str) or not re.fullmatch(r"[0-9a-f]{64}", binding["fingerprint"]):
            return False
        if any(any(event.get(field) != binding.get(field) for field in ("operation", "device_id", "request_id", "fingerprint")) for event in pair):
            return False
        value = pair[-1].get("result", {})
        if any(value.get(field) != binding.get(field) for field in ("request_id", "project_id", "session_id")):
            return False
        if not isinstance(value.get("terminal_id"), str) or not re.fullmatch(r"[0-9a-f]{64}", value["terminal_id"]):
            return False
        terminals.append(value["terminal_id"])
        if not isinstance(value.get("subtab_id"), str) or not re.fullmatch(r"[0-9a-f]{64}", value["subtab_id"]) \
            or type(value.get("subtab_revision")) is not int or not 0 <= value["subtab_revision"] <= 18446744073709551615:
            return False
        scopes.add((value["session_id"], value["subtab_id"], value["subtab_revision"]))
        if operation == "terminal_input" and (type(value.get("accepted_bytes")) is not int or not 1 <= value["accepted_bytes"] <= 4096):
            return False
        if index in ({0, 3} if initial_close else {2}) and value.get("accepted_bytes") != len(b"exit\r"):
            return False
    if initial_close:
        if terminals[0] in (terminals[1], terminals[4]): return False
        terminals = terminals[1:]
    if terminals[0] != terminals[1] or terminals[0] != terminals[2] or terminals[3] != terminals[4] or terminals[0] == terminals[3] or len(scopes) != 1:
        return False
    for name, expected in (("android-terminal-owned.txt", b"ANDROID_TERMINAL_OWNED\n"),
                           ("android-terminal-reopened.txt", b"ANDROID_TERMINAL_REOPENED\n")):
        path = workspace / name
        if not path.is_file() or path.is_symlink() or path.stat().st_size != len(expected) or path.read_bytes() != expected:
            return False
    return True


def directory_execution_evidence(records, requests, workspace, project, device):
    workspace = workspace.resolve()
    admissions = [record.get("event", {}) for record in records if record.get("event", {}).get("type") == "remote_prompt_admission_v1"
        and record.get("event", {}).get("state") == "accepted"]
    turns = [record["turn"] for record in records if record.get("kind") == "turn"]
    for name, marker in (("directory-one", "STAGE2_ANDROID_DIRECTORY_ONE"), ("directory-two", "STAGE2_ANDROID_DIRECTORY_TWO")):
        selected = [event for event in admissions if event.get("execution_subtab", {}).get("root") == str(workspace / name)]
        if len(selected) != 1 or selected[0].get("device_id") != device or selected[0].get("result", {}).get("project_id") != project:
            return False
        scope = selected[0]["execution_subtab"]
        if scope.get("kind") != "in_place" or not re.fullmatch(r"[0-9a-f]{64}", scope.get("id", "")):
            return False
        tool = [turn for turn in turns if turn.get("role") == "tool" and turn.get("metadata", {}).get("tool_call_id") == "read-" + marker]
        if len(tool) != 1 or f"must survive tab close: {name}" not in tool[0].get("content", ""):
            return False
        outputs = [item for request in requests for item in request.get("input", [])
            if item.get("type") == "function_call_output" and item.get("call_id") == "read-" + marker]
        if not outputs or not any(f"must survive tab close: {name}" in json.dumps(output) for output in outputs):
            return False
        if not any(turn.get("role") == "assistant" and marker + "_DONE" in turn.get("content", "") for turn in turns):
            return False
    return True


def extension_workspace_evidence(records, extension_events, workspace):
    workspace = workspace.resolve()
    selected = [record.get("event", {}) for record in records
        if record.get("event", {}).get("type") == "extensions_workspace_selected"]
    restored = [record.get("event", {}) for record in records
        if record.get("event", {}).get("type") == "extensions_workspace_restored"]
    if len(selected) != 2 or len(restored) != 2:
        return False
    generations = set()
    for name in ("directory-one", "directory-two"):
        root = str(workspace / name)
        bindings = [event for event in selected if event.get("workspace_root") == root]
        if len(bindings) != 1:
            return False
        binding = bindings[0]
        generation, recovery = binding.get("generation"), binding.get("restore_generation")
        if not isinstance(generation, int) or not isinstance(recovery, int) or generation == recovery or generation in generations or recovery in generations:
            return False
        generations.update((generation, recovery))
        scoped = [event for event in extension_events if event.get("generation") == generation]
        if not scoped or any(event.get("workspace") != root for event in scoped):
            return False
        for hook in ("initialize", "session_start", "agent_start", "before_tool", "tools/call", "after_tool", "agent_end", "session_close"):
            if sum(event.get("hook") == hook for event in scoped) != 1:
                return False
        calls = [event for event in scoped if event.get("hook") == "tools/call"]
        if calls[0].get("contents") != f"must survive tab close: {name}\n":
            return False
        recovery_events = [event for event in extension_events if event.get("generation") == recovery]
        if not recovery_events or any(event.get("workspace") != str(workspace) for event in recovery_events):
            return False
        for hook in ("initialize", "session_start", "session_close"):
            if sum(event.get("hook") == hook for event in recovery_events) != 1:
                return False
        if sum(event.get("generation") == recovery for event in restored) != 1:
            return False
    return True


def worktree_execution_evidence(records, requests, workspace, project, device, tab_id):
    admissions = [record.get("event", {}) for record in records if record.get("event", {}).get("type") == "remote_prompt_admission_v1"
        and record.get("event", {}).get("state") == "accepted" and record.get("event", {}).get("execution_subtab", {}).get("id") == tab_id]
    if len(admissions) != 1 or admissions[0].get("device_id") != device or admissions[0].get("result", {}).get("project_id") != project:
        return False
    scope = admissions[0]["execution_subtab"]
    if scope.get("kind") != "worktree" or scope.get("root") != str(workspace.resolve() / ".zenpi-worktrees/android-feature"):
        return False
    call_id = "read-STAGE2_ANDROID_WORKTREE_READ"
    tools = [record["turn"] for record in records if record.get("kind") == "turn" and record.get("turn", {}).get("role") == "tool"
        and record["turn"].get("metadata", {}).get("tool_call_id") == call_id]
    if len(tools) != 1 or "WORKTREE_ONLY_CONTENT" not in tools[0].get("content", ""):
        return False
    outputs = [item for request in requests for item in request.get("input", []) if item.get("type") == "function_call_output" and item.get("call_id") == call_id]
    return bool(outputs) and any("WORKTREE_ONLY_CONTENT" in json.dumps(item) for item in outputs) and any(
        record.get("kind") == "turn" and record.get("turn", {}).get("role") == "assistant"
        and "STAGE2_ANDROID_WORKTREE_READ_DONE" in record["turn"].get("content", "") for record in records)


def install_workspace_extension(config):
    directory = config / "extensions" / "workspace_fixture"
    directory.mkdir(parents=True)
    hooks = ["session_start", "agent_start", "before_tool", "after_tool", "agent_end", "session_close"]
    (directory / "extension.toml").write_text(
        'name="workspace_fixture"\nversion="1.0.0"\napi_version=2\nexecutable="plugin.py"\n'
        f'hooks={json.dumps(hooks)}\nhook_timeout_ms=1000\n'
        '[permissions]\nworkspace_read=true\n[[tools]]\nname="workspace_fixture_read"\n'
        'description="Read fixture contents in the host-selected workspace"\nside_effect="read_only"\n'
        'input_schema={type="object",properties={path={type="string"}},required=["path"],additionalProperties=false}\n')
    plugin = directory / "plugin.py"
    plugin.write_text(f"#!{sys.executable}\n" + '''import json, os, sys
from pathlib import Path
request = json.loads(sys.stdin.readline())
params = request["params"]
method = request["method"]
workspace = Path(os.environ["ZENPI_WORKSPACE"])
contents = None
if method == "initialize":
    result = {"api_version":2,"hooks":params["hooks"],"tools":[tool["name"] for tool in params["tools"]]}
elif method == "tools/call":
    target = (workspace / params["arguments"]["path"]).resolve()
    target.relative_to(workspace)
    contents = target.read_text()
    result = {"workspace":str(workspace),"contents":contents}
else:
    result = {"action":"continue"}
with open("events.jsonl", "a") as journal:
    journal.write(json.dumps({"hook":params.get("hook", method),"workspace":str(workspace),
        "generation":request["capability"]["generation"],"session_id":request["capability"]["session_id"],
        "pid":os.getpid(),"contents":contents}) + "\\n")
print(json.dumps({"jsonrpc":"2.0","id":request["id"],"capability":request["capability"],"result":result}))
''')
    plugin.chmod(0o700)
    return directory / "events.jsonl"


def directory_stress_evidence(before, after, device_id, project_id):
    prior = before["remote_workspace"]
    current = after["remote_workspace"]
    old = prior["receipts"]
    receipts = current["receipts"]
    if receipts[:len(old)] != old or len(receipts) != len(old) + 12 or current["revision"] != prior["revision"] + 12:
        return False
    previous_tabs = before["subtabs"][project_id]
    tabs = after["subtabs"][project_id]
    if tabs["tabs"] != previous_tabs["tabs"] or tabs["revision"] != previous_tabs["revision"] + 12:
        return False
    identities = {tab["name"]:tab["id"] for tab in tabs["tabs"]}
    additions = receipts[len(old):]
    if len({receipt.get("request_id") for receipt in additions}) != 12:
        return False
    for index, receipt in enumerate(additions):
        result = receipt.get("result", {})
        if receipt.get("device_id") != device_id or not re.fullmatch(r"[0-9a-f]{64}", receipt.get("fingerprint", "")):
            return False
        if result.get("project_id") != project_id or result.get("active_project_id") != project_id or result.get("required_owner") != old[-1]["result"]["required_owner"]:
            return False
        if result.get("workspace_revision") != prior["revision"] + index + 1 or result.get("subtab_revision") != previous_tabs["revision"] + index + 1:
            return False
        if result.get("active_subtab_id") != identities.get("directory-one" if index % 2 == 0 else "directory-two"):
            return False
    return tabs["active_id"] == identities.get("directory-two")


def terminal_pixels(data, width=140, height=100):
    rows = [[(False, False)] * width for _ in range(height)]
    x = y = 0
    fg = bg = False
    text = data.decode(errors="replace")
    index = 0
    while index < len(text):
        char = text[index]
        if char == "\x1b":
            match = re.match(r"\x1b\[([0-?]*)([ -/]*)([@-~])", text[index:])
            if match:
                raw, _, command = match.groups()
                index += len(match.group())
                values = [int(part) if part.isdigit() else 0 for part in raw.lstrip("?").split(";")]
                n = values[0] or 1
                if command in "Hf":
                    y = min(height - 1, max(0, n - 1)); x = min(width - 1, max(0, (values[1] if len(values) > 1 else 1) - 1))
                elif command == "G": x = min(width - 1, n - 1)
                elif command == "d": y = min(height - 1, n - 1)
                elif command == "A": y = max(0, y - n)
                elif command == "B": y = min(height - 1, y + n)
                elif command == "C": x = min(width - 1, x + n)
                elif command == "D": x = max(0, x - n)
                elif command == "J" and values[0] in (2, 3): rows = [[(False, False)] * width for _ in range(height)]
                elif command == "h" and raw == "?1049": rows = [[(False, False)] * width for _ in range(height)]; x = y = 0
                elif command == "m":
                    i = 0
                    while i < len(values):
                        v = values[i]
                        if v == 0: fg = bg = False
                        elif v in (30, 37, 39): fg = v == 30
                        elif v in (40, 47, 49): bg = v == 40
                        elif v in (38, 48) and i + 2 < len(values):
                            if values[i + 1] == 5:
                                black = values[i + 2] in (0, 16); i += 2
                            elif values[i + 1] == 2 and i + 4 < len(values):
                                black = values[i + 2:i + 5] == [0, 0, 0]; i += 4
                            else: raise RuntimeError("unsupported terminal color")
                            if v == 38: fg = black
                            else: bg = black
                        i += 1
                continue
            match = re.match(r"\x1b\][^\a]*(?:\a|\x1b\\)", text[index:])
            if match: index += len(match.group()); continue
            index += 2; continue
        if char == "\r": x = 0
        elif char == "\n": y = min(height - 1, y + 1)
        elif ord(char) >= 32 and x < width:
            rows[y][x] = (fg, bg) if char == "▀" else (False, False)
            x += 1
        index += 1
    scale = 4
    pixels = bytearray()
    for row in rows:
        for half in (0, 1):
            scan = bytes(value for cell in row for value in [0 if cell[half] else 255] * scale)
            pixels.extend(scan * scale)
    return f"P5\n{width * scale} {height * 2 * scale}\n255\n".encode() + pixels


class Desktop(Terminal):
    def __init__(self, binary, workspace, session, identity, env, bind_ip="127.0.0.1", duration=120, advertise=None):
        self.output = bytearray(); self.fd = None; self.pid = None
        self.checkpoint = Path(env["ZENPI_HOME"]) / "project-tabs.json"
        # The desktop fixture must not inherit ambient provider credentials or
        # unrelated ZENPI state from the invoking shell.
        env = {key: value for key, value in env.items()
               if key in {"PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "ZENPI_HOME", "TERM", "NO_PROXY", "no_proxy"}}
        self.child_env = env
        endpoint = f"[{bind_ip}]:0" if ":" in bind_ip else f"{bind_ip}:0"
        self.argv = [str(binary), "host", "pair", "--desktop", "--workspace", str(workspace),
            "--session", str(session), "--bind", endpoint, "--duration-seconds", str(duration), "--identity", str(identity)]
        if advertise is not None:
            self.argv += ["--advertise", private_lan_ip(advertise)]
        pid, fd = pty.fork()
        if pid == 0:
            os.chdir(workspace)
            os.execve(str(binary), self.argv, self.child_env)
        self.pid, self.fd = pid, fd
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 100, 140, 0, 0))
        try:
            self.wait(lambda: b"Safety code:" in screen_text(bytes(self.output), height=100), timeout=20)
            self.wait(lambda: b"Host certificate SHA-256:" in screen_text(bytes(self.output), height=100), timeout=20)
            drain(fd, self.output, .3)
        except Exception:
            self.cleanup()
            raise

    def expect(self, text, timeout=12, start=0):
        self.wait(lambda: text in screen_text(bytes(self.output), height=100), timeout=timeout)


def failed_instrumentation_evidence(error, results, invite):
    retained = [result for result in results if result is not None]
    if isinstance(error, InstrumentationFailure) and all(result is not error.result for result in retained):
        retained.append(error.result)
    def redact(value):
        return value.replace(invite, "<redacted-invite>") if invite else value
    return [{"exit_code":result.returncode, "stdout":redact(result.stdout), "stderr":redact(result.stderr),
             "timed_out":isinstance(error, InstrumentationFailure) and error.result is result and error.timed_out}
            for result in retained]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/zenpi")
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--with-workspace-control", action="store_true")
    parser.add_argument("--with-workspace-extensions", action="store_true")
    parser.add_argument("--with-worktree-workflow", action="store_true")
    route = parser.add_mutually_exclusive_group()
    route.add_argument("--lan-ip", type=private_lan_ip, help="Bind to this local private interface; connect directly without adb reverse")
    route.add_argument("--emulator-host-network", action="store_true", help="Use 10.0.2.2 on an emulator, without adb reverse; not physical LAN")
    parser.add_argument("--expected-api", type=int, help="Refuse evidence from a different Android API")
    parser.add_argument("--with-doze", action="store_true", help="Add controlled emulator deep-idle/foreground recovery validation")
    parser.add_argument("--validation-seconds", type=int, default=120, choices=range(120, 601), help="Bounded initial instrumentation observation budget; invite TTL still caps at 300 seconds")
    args = parser.parse_args()
    build_provenance = require_current_build()
    host_provenance = require_current_host(args.binary)
    if args.with_workspace_extensions and not args.with_workspace_control:
        parser.error("--with-workspace-extensions requires --with-workspace-control")
    if args.with_worktree_workflow and (not args.with_workspace_control or args.with_workspace_extensions):
        parser.error("--with-worktree-workflow requires workspace control and a separate run from extension fixtures")
    if args.with_doze and (not args.with_workspace_control or not args.serial.startswith("emulator-")):
        parser.error("--with-doze requires workspace control on an emulator fixture")
    args.binary = args.binary.resolve()
    adb = ["adb", "-s", args.serial]
    bind_ip = "0.0.0.0" if args.emulator_host_network else args.lan_ip or "127.0.0.1"
    advertise = "10.0.2.2" if args.emulator_host_network else None
    wifi_setup = None
    reverse_created = False
    apk = ROOT / "android/app/build/outputs/apk/debug/app-debug.apk"
    test_apk = ROOT / "android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
    artifacts = {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()
        for p in (args.binary, apk, test_apk)}
    harness_digest = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    test_verifier = ROOT / "tools/stage2_instrumentation.py"
    test_verifier_digest = hashlib.sha256(test_verifier.read_bytes()).hexdigest()
    harness_inputs = {name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in
        ("tools/stage2_desktop_android_smoke.py", "tools/stage2_android_pair_smoke.py", "tools/stage2_instrumentation.py", "tools/decode_terminal_qr.swift")}
    subprocess.run(adb + ["wait-for-device"], check=True, timeout=90)
    until = time.monotonic() + 90
    while subprocess.run(adb + ["shell", "getprop", "sys.boot_completed"], capture_output=True, text=True, timeout=10).stdout.strip() != "1":
        if time.monotonic() > until: raise RuntimeError("Android boot deadline")
        time.sleep(1)
    android_api = int(subprocess.run(adb + ["shell", "getprop", "ro.build.version.sdk"], capture_output=True, text=True, check=True, timeout=10).stdout.strip())
    if args.expected_api is not None and android_api != args.expected_api:
        raise RuntimeError(f"Expected Android API {args.expected_api}, found {android_api}")
    if args.emulator_host_network:
        wifi_setup = prepare_emulator_host_network(adb)
    if args.lan_ip:
        require_no_reverse(adb)
        # Reject a non-local address before seeding a session or opening the listener.
        family = socket.AF_INET6 if ":" in bind_ip else socket.AF_INET
        with socket.socket(family, socket.SOCK_STREAM) as probe:
            probe.bind((bind_ip, 0))
    for artifact in (apk, test_apk): subprocess.run(adb + ["install", "-r", str(artifact)], check=True, timeout=60)
    started = time.time()
    terminal = server = None
    port = None
    with tempfile.TemporaryDirectory(prefix="zenpi-shared-desktop-") as raw:
        root = Path(raw)
        try:
            workspace, session, env, project, answer, seed = seed_session(args.binary.resolve(), root)
            extension_log = install_workspace_extension(root / "config") if args.with_workspace_extensions else None
            if args.with_worktree_workflow:
                (workspace / "keep.txt").write_text("TRACKED_WORKTREE_CONTENT\n")
                (workspace / "diff-pages.txt").write_text("DIFF_BASELINE\n")
                for git_args in (["init", "-q"], ["config", "user.name", "fixture"], ["config", "user.email", "fixture@example.invalid"],
                    ["add", "keep.txt", "diff-pages.txt"], ["commit", "-qm", "fixture baseline"]):
                    subprocess.run(["git", "-C", str(workspace), *git_args], check=True, capture_output=True, timeout=10)
                diff_content = "".join(f"DIFF_PAGE_{index:04d}_UTF8_\u4e2d\n" for index in range(4000))
                (workspace / "diff-pages.txt").write_text(diff_content, encoding="utf-8")
                diff_patch = subprocess.run(["git", "-C", str(workspace), "diff", "HEAD", "--no-ext-diff", "--no-textconv", "--no-color", "--unified=3", "--", "diff-pages.txt"],
                    check=True, capture_output=True, timeout=10).stdout
                diff_patch_sha256 = hashlib.sha256(diff_patch).hexdigest()
                diff_total_bytes = len(diff_patch)
                (workspace / "diff-pages.txt").write_text("DIFF_BASELINE\n")
            child = workspace / "child"
            if args.with_workspace_control: child.mkdir()
            if args.with_workspace_control:
                for name in ("directory-one", "directory-two"):
                    (workspace / name).mkdir()
                    (workspace / name / "keep.txt").write_text(f"must survive tab close: {name}\n")
            child_id = hashlib.sha256(b"zenpi-project-workspace-v1\0" + str(child.resolve()).encode()).hexdigest()
            second = "STAGE2_LIVE_DESKTOP_OWNER_UPDATE"
            third = "STAGE2_DESKTOP_UPDATE_WHILE_ANDROID_STOPPED"
            background = "STAGE2_DESKTOP_UPDATE_READ_BY_WORKMANAGER"
            requests = []
            class Handler(BaseHTTPRequestHandler):
                def log_message(self, *args): pass
                def do_POST(self):
                    body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                    requests.append(body)
                    latest_input = json.dumps([item for item in body.get("input", []) if item.get("role") == "user"][-1:])
                    approval_prompt = "STAGE2_DESKTOP_APPROVAL_" in latest_input
                    approval_decision = "allow" if "STAGE2_DESKTOP_APPROVAL_ALLOW" in latest_input else "deny"
                    remote_prompt = "STAGE2_ANDROID_PROMPT_" in latest_input
                    directory_marker = next((marker for marker in ("STAGE2_ANDROID_DIRECTORY_ONE", "STAGE2_ANDROID_DIRECTORY_TWO", "STAGE2_ANDROID_WORKTREE_READ") if marker in latest_input), None)
                    if directory_marker:
                        self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.end_headers()
                        call_id = "read-" + directory_marker
                        output = [item for item in body.get("input", []) if item.get("type") == "function_call_output" and item.get("call_id") == call_id]
                        if not output:
                            events = [{"type":"response.output_item.done", "item":{"type":"function_call", "call_id":call_id,
                                "name":"workspace_fixture_read" if args.with_workspace_extensions else "read_file", "arguments":json.dumps({"path":"keep.txt"})}},
                                {"type":"response.completed", "response":{"id":call_id,"status":"completed","model":body["model"]}}]
                        else:
                            expected = "directory-one" if directory_marker.endswith("ONE") else "directory-two"
                            content = json.dumps(output)
                            expected_content = "WORKTREE_ONLY_CONTENT" if directory_marker == "STAGE2_ANDROID_WORKTREE_READ" else f"must survive tab close: {expected}"
                            answer = directory_marker + ("_DONE" if expected_content in content else "_WRONG_ROOT")
                            events = [{"type":"response.output_text.delta","delta":answer},
                                {"type":"response.completed","response":{"id":call_id,"status":"completed","model":body["model"]}}]
                        for event in events:
                            self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode()); self.wfile.flush()
                        return
                    if "STAGE2_ANDROID_PROMPT_FAILURE" in latest_input:
                        self.send_response(500); self.send_header("Content-Type", "application/json"); self.end_headers()
                        self.wfile.write(b'{"error":{"message":"controlled provider failure"}}'); self.wfile.flush()
                        return
                    self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.end_headers()
                    if approval_prompt and not any(item.get("type") == "function_call_output" and item.get("call_id") == f"approval-{approval_decision}" for item in body.get("input", [])):
                        events = [{"type":"response.output_item.done", "item":{"type":"function_call", "call_id":f"approval-{approval_decision}",
                            "name":"write_file", "arguments":json.dumps({"path":f"approval-{approval_decision}.txt", "content":f"approved-{approval_decision}"})}},
                            {"type":"response.completed", "response":{"id":f"tool-{approval_decision}", "status":"completed", "model":body["model"]}}]
                        for event in events:
                            self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode()); self.wfile.flush()
                        return
                    if remote_prompt: time.sleep(8 if "STAGE2_ANDROID_PROMPT_REQUEST" in latest_input else (6 if "STAGE2_ANDROID_PROMPT_CANCEL" in latest_input else 3))
                    answers = [second, background, third] if args.with_workspace_control else [second, third]
                    normal_count = sum(not any(marker in json.dumps([value for value in item.get("input", []) if value.get("role") == "user"][-1:]) for marker in ("STAGE2_ANDROID_PROMPT_", "STAGE2_DESKTOP_APPROVAL_", "STAGE2_ANDROID_DIRECTORY_", "STAGE2_ANDROID_WORKTREE_")) for item in requests)
                    answer_text = f"STAGE2_APPROVAL_{approval_decision.upper()}_DONE" if approval_prompt else ("STAGE2_ANDROID_PROMPT_PROVIDER_ANSWER" if remote_prompt else answers[min(normal_count - 1, len(answers) - 1)])
                    for event in [{"type":"response.output_text.delta", "delta":answer_text},
                        {"type":"response.completed", "response":{"id":"desktop", "status":"completed", "model":body["model"]}}]:
                        self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode()); self.wfile.flush()
            server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
            threading.Thread(target=server.serve_forever, daemon=True).start()
            config = root / "config/config.toml"
            config.write_text('backend="openai"\nmodel="gpt-4.1"\n'
                f'base_url="http://127.0.0.1:{server.server_port}/v1"\nwire_api="responses"\nrequires_openai_auth=false\nmax_retries=0\n')
            env["TERM"] = "xterm-256color"
            identity = root / "identity.json"
            terminal = Desktop(args.binary.resolve(), workspace, session, identity, env, bind_ip, min(300, args.validation_seconds + 60), advertise=advertise)
            pgm = root / "terminal.pgm"
            pgm.write_bytes(terminal_pixels(bytes(terminal.output)))
            if pgm.read_bytes().count(b"\x00") == 0:
                codes = re.findall(rb"\x1b\[[0-?]*[ -/]*[@-~]", bytes(terminal.output))
                raise RuntimeError("QR reconstruction blank; style codes=" + repr(sorted(set(c for c in codes if c.endswith(b"m")))) + "; halfblocks=" + str(bytes(terminal.output).count("▀".encode())))
            decoding = subprocess.run(["swift", str(ROOT / "tools/decode_terminal_qr.swift"), str(pgm)],
                capture_output=True, text=True, timeout=60)
            if decoding.returncode:
                raise RuntimeError("terminal QR decode failed: " + decoding.stderr[:1500])
            decoded = decoding.stdout.strip()
            values = parse_qs(urlsplit(decoded).query, strict_parsing=True)
            advertised_ip, advertised_port = values["address"][0].rsplit(":", 1)
            if advertised_ip != (advertise or bind_ip): raise RuntimeError("Production QR did not advertise the selected listener endpoint")
            port = int(advertised_port)
            cert = json.loads(identity.read_text())["certificate"]
            assert hashlib.sha256(base64.urlsafe_b64decode(cert + "=" * (-len(cert) % 4))).hexdigest() == values["fingerprint"][0]
            if not args.lan_ip and not args.emulator_host_network:
                subprocess.run(adb + ["reverse", f"tcp:{port}", f"tcp:{port}"], check=True, timeout=10)
                reverse_created = True

            def devices():
                records = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
                assert len(records) == 1
                return records[0]["id"]
            def grant():
                command = [str(args.binary.resolve()), "host", "pair", "--identity", str(identity),
                    "--grant-device-control" if args.with_workspace_control else "--grant-device-read", devices(), "--project", project]
                if args.with_workspace_control: command += ["--project", child_id]
                subprocess.run(command, check=True, capture_output=True, timeout=10)
                if args.with_workspace_control:
                    terminal.write(b"\x1b"); terminal.expect(b"Prompt")
            workspace_checks = []
            approval_checks = []
            def approval_action(line):
                if "approval_allow_requested" in line:
                    subprocess.run([str(args.binary.resolve()), "host", "pair", "--identity", str(identity), "--grant-device-approval", devices(), "--project", project], check=True, capture_output=True, timeout=10)
                    terminal.command("STAGE2_DESKTOP_APPROVAL_ALLOW")
                elif "approval_deny_requested" in line:
                    assert (workspace / "approval-allow.txt").read_text() == "approved-allow"
                    approval_checks.append("allow")
                    terminal.command("STAGE2_DESKTOP_APPROVAL_DENY")
                elif "approval_decisions_verified" in line:
                    assert not (workspace / "approval-deny.txt").exists()
                    approval_checks.append("deny")
                    subprocess.run([str(args.binary.resolve()), "host", "pair", "--identity", str(identity), "--grant-device-control", devices(), "--project", project, "--project", child_id], check=True, capture_output=True, timeout=10)
            subtab_checkpoint_verified = False
            directory_checkpoint_verified = False
            directory_order_verified = False
            directory_stress_verified = False
            directory_stress_before = None
            worktree_checks = []
            diff_page_phases = []
            diff_read_state = None
            worktree_id = None
            def worktree_changed(line):
                nonlocal worktree_id, diff_read_state
                saved = json.loads((session.parent / "project-workspace.json").read_text())
                tabs = saved["subtabs"][project]
                path = workspace / ".zenpi-worktrees/android-feature"
                if "worktree_created_verified" in line:
                    assert len(tabs["tabs"]) == 2 and tabs["tabs"][1]["kind"] == "worktree"
                    worktree_id = tabs["tabs"][1]["id"]
                    assert tabs["active_id"] == worktree_id
                    assert Path(tabs["tabs"][1]["root"]) == path.resolve()
                    assert (path / "keep.txt").read_text() == "TRACKED_WORKTREE_CONTENT\n"
                    output = subprocess.run(["git", "-C", str(workspace), "worktree", "list", "--porcelain"], capture_output=True, text=True, check=True, timeout=10)
                    assert str(path.resolve()) in output.stdout
                    (path / "keep.txt").write_text("WORKTREE_ONLY_CONTENT\n")
                    worktree_checks.append("create")
                elif "worktree_dirty_refusal_verified" in line:
                    assert len(tabs["tabs"]) == 2 and tabs["active_id"] == worktree_id
                    assert (path / "keep.txt").read_text() == "WORKTREE_ONLY_CONTENT\n"
                    assert (workspace / "keep.txt").read_text() == "TRACKED_WORKTREE_CONTENT\n"
                    assert worktree_execution_evidence([json.loads(line) for line in session.read_text().splitlines()], requests, workspace, project, devices(), worktree_id)
                    # Restore only the tracked fixture file we changed, after
                    # Android's destructive request has been proved refused.
                    (path / "keep.txt").write_text("TRACKED_WORKTREE_CONTENT\n")
                    worktree_checks.append("dirty_refusal")
                elif "worktree_diff_ui_verified" in line:
                    assert tabs["active_id"] == worktree_id and len(tabs["tabs"]) == 2
                    assert (path / "keep.txt").read_text() == "WORKTREE_ONLY_CONTENT\n"
                elif any(phase in line for phase in DIFF_PAGE_PHASES):
                    phase = next(phase for phase in DIFF_PAGE_PHASES if phase in line)
                    assert phase == DIFF_PAGE_PHASES[len(diff_page_phases)]
                    assert tabs["active_id"] == worktree_id and len(tabs["tabs"]) == 2
                    current_read_state = (saved["remote_workspace"]["revision"], tabs, hashlib.sha256(session.read_bytes()).hexdigest())
                    if phase == DIFF_PAGE_PHASES[0]:
                        assert (path / "diff-pages.txt").read_text() == "DIFF_BASELINE\n"
                        (path / "diff-pages.txt").write_text(diff_content, encoding="utf-8")
                        diff_read_state = current_read_state
                    else:
                        assert current_read_state == diff_read_state, "Diff read mutated owner state or journal"
                        expected = diff_content if phase == DIFF_PAGE_PHASES[1] else diff_content + "DIFF_CHANGED\n"
                        assert (path / "diff-pages.txt").read_text(encoding="utf-8") == expected
                        (path / "diff-pages.txt").write_text(diff_content + "DIFF_CHANGED\n" if phase == DIFF_PAGE_PHASES[1] else "DIFF_BASELINE\n", encoding="utf-8")
                    diff_page_phases.append(phase)
                elif "worktree_removed_verified" in line:
                    assert len(tabs["tabs"]) == 1 and not path.exists()
                    subprocess.run(["git", "-C", str(workspace), "show-ref", "--verify", "refs/heads/android-feature"], check=True, capture_output=True, timeout=10)
                    worktree_checks.append("remove")
            def subtab_changed():
                nonlocal subtab_checkpoint_verified
                path = session.parent / "project-workspace.json"
                def matches():
                    if not path.exists(): return False
                    domain = json.loads(path.read_text())["subtabs"][project]
                    return domain["tabs"][0]["name"] == "ui-main" and domain["active_id"] == domain["tabs"][0]["id"]
                terminal.wait(matches)
                subtab_checkpoint_verified = True
            def directories_changed(line):
                nonlocal directory_checkpoint_verified, directory_order_verified, directory_stress_verified, directory_stress_before
                path = session.parent / "project-workspace.json"
                if "directory_selection_stress_start" in line:
                    return
                if "directory_selection_stress_verified" in line:
                    directory_stress_verified = directory_stress_before is not None and directory_stress_evidence(directory_stress_before,
                        json.loads(path.read_text()), devices(), project)
                    return
                if "directory_order_verified" in line:
                    def ordered():
                        state = json.loads(path.read_text())["subtabs"][project]
                        return [tab["name"] for tab in state["tabs"]] == ["ui-main", "directory-two", "directory-one"] and state["active_id"] == state["tabs"][1]["id"]
                    terminal.wait(ordered)
                    directory_order_verified = True
                    directory_stress_before = json.loads(path.read_text())
                    return
                terminal.wait(lambda: len(json.loads(path.read_text())["subtabs"][project]["tabs"]) == 1)
                assert all((workspace / name / "keep.txt").read_text() == f"must survive tab close: {name}\n" for name in ("directory-one", "directory-two"))
                directory_checkpoint_verified = True
            def workspace_changed(line):
                def matches():
                    if not terminal.checkpoint.exists(): return False
                    saved = json.loads(terminal.checkpoint.read_text())
                    if "workspace_open_verified" in line:
                        return child_id in saved["projects"] and saved["projects"][saved["active"]] == child_id
                    return child_id not in saved["projects"] and saved["projects"][saved["active"]] == project
                terminal.wait(matches)
                workspace_checks.append("open" if "workspace_open_verified" in line else "close")
            def update():
                if not args.with_workspace_control:
                    terminal.write(b"\x1b")
                    terminal.expect(b"Prompt")
                terminal.command("Generate a second desktop answer", second.encode())
                terminal.wait(lambda: second in session.read_text())
            def revoke():
                record = json.loads(identity.with_suffix(".devices.json").read_text())["devices"][0]
                if record["revoked"]: return
                subprocess.run([str(args.binary.resolve()), "host", "pair", "--identity", str(identity),
                    "--revoke-device", devices()], check=True, capture_output=True, timeout=10)
            def background_update():
                terminal.command("Generate an answer for background resume", background.encode())
                terminal.wait(lambda: background in session.read_text())
            network_namespace = "network-" + str(time.time_ns())
            instrumentation_classes, expected_tests = instrumentation_suite(args.with_workspace_control, args.with_doze)
            command = ["am", "instrument", "-w", "-r", "-e", "invite", decoded,
                "-e", "class", ",".join(instrumentation_classes),
                *(["-e", "doze_validation", "true"] if args.with_doze else []),
                "-e", "process_namespace", network_namespace,
                "-e", "expected_project", project, "-e", "expected_text", answer, "-e", "desktop_update", second,
                "-e", "background_text", background,
                *(["-e", "workspace_child", child_id] if args.with_workspace_control else []),
                *(["-e", "worktree_workflow", "true"] if args.with_worktree_workflow else []),
                *(["-e", "diff_patch_sha256", diff_patch_sha256, "-e", "diff_total_bytes", str(diff_total_bytes)] if args.with_worktree_workflow else []),
                "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]
            result = run_instrumentation(adb, shlex.join(command), grant, revoke, update, lambda: drain(terminal.fd, terminal.output, .01),
                on_workspace=workspace_changed if args.with_workspace_control else None,
                on_background_update=background_update if args.with_workspace_control else None,
                on_subtab=subtab_changed if args.with_workspace_control else None,
                on_directory=directories_changed if args.with_workspace_control else None,
                on_approval=approval_action if args.with_workspace_control else None,
                on_worktree=worktree_changed if args.with_worktree_workflow else None, timeout=args.validation_seconds)
            print(result.stdout.replace(decoded, "<redacted-invite>"))
            try:
                completed_tests = require_test_results(result.stdout, instrumentation_classes, expected_tests)
            except ValueError:
                completed_tests = []
            passed = result.returncode == 0 and bool(completed_tests) and "zenpi_phase=shared_desktop_verified" in result.stdout and "zenpi_phase=foreground_replay_verified" in result.stdout and "zenpi_phase=encrypted_cache_restore_verified" in result.stdout
            network_result = None
            passed = passed and "zenpi_phase=default_network_loss_resume_verified" in result.stdout
            passed = passed and result.stdout.splitlines().count("INSTRUMENTATION_STATUS: zenpi_phase=stopped_network_observation_fenced_verified") == 1
            passed = passed and "zenpi_phase=local_reset_all_owned_jobs_joined_verified" in result.stdout
            passed = passed and "zenpi_phase=local_reset_retired_jobs_joined_verified" in result.stdout
            passed = passed and "zenpi_phase=first_pair_cancel_saved_host_restored_verified" in result.stdout
            passed = passed and "zenpi_phase=background_pair_saved_host_restored_verified" in result.stdout
            passed = passed and "zenpi_phase=network_pair_saved_host_restored_verified" in result.stdout
            passed = passed and "zenpi_phase=explicit_identity_preconnect_fences_verified" in result.stdout
            passed = passed and "zenpi_phase=resume_storage_wait_cancellation_fenced_verified" in result.stdout
            passed = passed and "zenpi_phase=foreground_storage_wait_cancellation_fenced_verified" in result.stdout
            passed = passed and "zenpi_phase=foreground_storage_final_admission_fenced_verified" in result.stdout
            passed = passed and "zenpi_phase=revocation_identity_removal_cancellation_fenced_verified" in result.stdout
            passed = passed and result.stdout.splitlines().count("INSTRUMENTATION_STATUS: zenpi_phase=device_confirmation_scope_verified") == 1
            if args.with_doze: passed = passed and "zenpi_phase=doze_foreground_resume_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=workmanager_read_resume_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_subtab_control_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=production_subtab_buttons_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=confirmed_write_read_failure_verified" in result.stdout
            if args.with_workspace_control: passed = passed and subtab_checkpoint_verified
            if args.with_workspace_control: passed = passed and directory_checkpoint_verified and directory_order_verified
            if args.with_workspace_control: passed = passed and "zenpi_phase=directory_offline_cache_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=directory_selection_stress_verified" in result.stdout
            if args.with_workspace_control: passed = passed and directory_stress_verified
            if args.with_workspace_control: passed = passed and "zenpi_phase=directory_execution_verified" in result.stdout
            if args.with_worktree_workflow: passed = passed and worktree_checks == ["create", "dirty_refusal", "remove"]
            if args.with_worktree_workflow: passed = passed and "zenpi_phase=worktree_diff_ui_verified" in result.stdout
            diff_pages_verified = args.with_worktree_workflow and diff_page_workflow_evidence(diff_page_phases, diff_patch_sha256, diff_total_bytes, result.stdout)
            if args.with_worktree_workflow: passed = passed and diff_pages_verified
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_prompt_shared_runner_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_prompt_terminal_status_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_desktop_terminal_open_input_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_desktop_terminal_exit_reopen_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_prompt_cancel_verified" in result.stdout
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_steer_durable_receipt_verified" in result.stdout
            if args.with_workspace_control: passed = passed and approval_checks == ["allow", "deny"]
            if args.with_workspace_control: passed = passed and "zenpi_phase=android_approval_preview_verified" in result.stdout
            death_result = None
            killed_pid = None
            revoke_result = None
            killed_revoke_pid = None
            revoke_process_recovery = False
            network_resume = False
            if passed:
                first_pids = re.findall(r"network_process_pid=([0-9]+)", result.stdout)
                if len(first_pids) != 1: raise RuntimeError("Network seed process PID missing")
                def kill_live_process():
                    nonlocal killed_pid
                    killed_pid = force_stop_live_android(adb)
                hold_command = shlex.join(["am", "instrument", "-w", "-r", "-e", "class",
                    "zenpi.NetworkProcessTest#cachedOwnerRemainsAliveUntilHarnessKillsItsProcess",
                    *(["-e", "pending_write", "true"] if args.with_workspace_control else []),
                    "-e", "process_namespace", network_namespace, "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"])
                death_result = run_instrumentation(adb, hold_command, grant, revoke,
                    tick=lambda: drain(terminal.fd, terminal.output, .01), on_process_ready=kill_live_process)
                held_pids = re.findall(r"network_process_pid=([0-9]+)", death_result.stdout)
                if held_pids != [killed_pid] or "zenpi_phase=network_process_live_ready" not in death_result.stdout:
                    raise RuntimeError("Live process identity was not proved before death")
                terminal.command("Generate an answer while Android is stopped", third.encode())
                terminal.wait(lambda: third in session.read_text())
                command = shlex.join(["am", "instrument", "-w", "-r", "-e", "class", "zenpi.NetworkProcessTest#newProcessAuthenticatesAndReplaysFromItsEncryptedCursor",
                    *(["-e", "pending_write", "true"] if args.with_workspace_control else []),
                    "-e", "revoke_process_death", "true",
                    "-e", "process_namespace", network_namespace, "-e", "previous_pid", killed_pid,
                    "-e", "expected_text", third, "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"])
                def kill_revoked_process():
                    nonlocal killed_revoke_pid
                    record = json.loads(identity.with_suffix(".devices.json").read_text())["devices"][0]
                    if not record["revoked"] or not record.get("revoke_request_id"):
                        raise RuntimeError("Host revocation was not durable before process death")
                    killed_revoke_pid = force_stop_live_android(adb)
                network_result = run_instrumentation(adb, command, grant, revoke,
                    tick=lambda: drain(terminal.fd, terminal.output, .01), on_process_ready=kill_revoked_process)
                print(network_result.stdout)
                restored_pids = re.findall(r"network_process_pid=([0-9]+)", network_result.stdout)
                revoke_pids = re.findall(r"revoke_process_pid=([0-9]+)", network_result.stdout)
                revoke_ids = re.findall(r"revoke_request_id=([0-9a-f-]{36})", network_result.stdout)
                record = json.loads(identity.with_suffix(".devices.json").read_text())["devices"][0]
                network_resume = "FAILURES" not in network_result.stdout and "zenpi_phase=network_process_resume_verified" in network_result.stdout and len(restored_pids) == 1 and restored_pids[0] != killed_pid
                network_resume = network_resume and process_recovery_evidence(network_result.stdout, args.with_workspace_control)
                network_resume = network_resume and revoke_pids == [killed_revoke_pid] and len(revoke_ids) == 1 and record.get("revoke_request_id") == revoke_ids[0]
                if args.with_workspace_control:
                    network_resume = network_resume and "zenpi_phase=pending_write_process_recovery_verified" in network_result.stdout
                passed = passed and network_resume
                if network_resume:
                    command = shlex.join(["am", "instrument", "-w", "-r", "-e", "class",
                        "zenpi.NetworkProcessTest#newProcessResolvesRevocationReceiptAndWipesCredentials",
                        "-e", "process_namespace", network_namespace, "-e", "previous_pid", killed_revoke_pid,
                        "-e", "revoke_request_id", revoke_ids[0], "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"])
                    revoke_result = run_instrumentation(adb, command, grant, revoke, tick=lambda: drain(terminal.fd, terminal.output, .01))
                    print(revoke_result.stdout)
                    recovered_pids = re.findall(r"recovered_revoke_process_pid=([0-9]+)", revoke_result.stdout)
                    revoke_process_recovery = revoke_result.returncode == 0 and re.search(r"(?m)^OK \(1 test\)$", revoke_result.stdout) is not None and "FAILURES" not in revoke_result.stdout and "zenpi_phase=revoke_receipt_process_recovery_verified" in revoke_result.stdout and len(recovered_pids) == 1 and recovered_pids[0] != killed_revoke_pid
                    passed = passed and revoke_process_recovery
            cancel_checkpoint_verified = False
            prompt_binding_verified = False
            steering_verified = False
            directory_execution_verified = False
            terminal_open_verified = False
            provider_update = len(requests) == (17 if args.with_worktree_workflow else (15 if args.with_workspace_control else 2)) and second in session.read_text() and third in session.read_text()
            if args.with_workspace_control:
                prompt_events = [json.loads(line).get("event", {}) for line in session.read_text().splitlines()]
                prompt_receipts = [event for event in prompt_events if event.get("type") == "remote_prompt_admission_v1"]
                provider_update = provider_update and [event.get("state") for event in prompt_receipts] == ["reserved", "accepted"] * (6 if args.with_worktree_workflow else 5)
                progress = [event.get("state") for event in prompt_events if event.get("type") == "remote_prompt_progress_v1"]
                expected_progress = ["running", "succeeded", "running", "failed", "running", "cancelled", "running", "succeeded", "running", "succeeded"] + (["running", "succeeded"] if args.with_worktree_workflow else [])
                provider_update = provider_update and progress == expected_progress
                provider_update = provider_update and "STAGE2_ANDROID_PROMPT_PROVIDER_ANSWER" in session.read_text()
                saved = json.loads((session.parent / "project-workspace.json").read_text())
                terminal_open_verified = terminal_open_evidence(saved["remote_workspace"].get("prompt_bindings", []), prompt_events, workspace, devices(), project)
                provider_update = provider_update and terminal_open_verified
                cancelled = [event for event in prompt_events if event.get("type") == "remote_prompt_progress_v1" and event.get("state") == "cancelled"]
                cancel_receipts = saved["remote_workspace"].get("cancel_receipts", [])
                cancel_checkpoint_verified = len(cancel_receipts) == 1 and len(cancelled) == 1
                if cancel_checkpoint_verified:
                    receipt, terminal_event = cancel_receipts[0], cancelled[0]
                    cancel_checkpoint_verified = receipt["state"] == "accepted" and receipt["code"] is None and receipt["target_request_id"] == terminal_event["request_id"]
                    cancel_checkpoint_verified = cancel_checkpoint_verified and all(receipt[field] == terminal_event[field] for field in ("device_id", "project_id", "session_id"))
                provider_update = provider_update and cancel_checkpoint_verified
                approval_receipts = saved["remote_workspace"].get("approval_receipts", [])
                provider_update = provider_update and len(cancelled) == 1 and approval_evidence(approval_receipts, prompt_events, devices(), project, cancelled[0]["session_id"])
                prompt_binding_verified = len(cancelled) == 1 and prompt_binding_evidence(saved["remote_workspace"].get("prompt_bindings", []), prompt_events, devices(), project, cancelled[0]["session_id"], expected_count=6 if args.with_worktree_workflow else 5)
                provider_update = provider_update and prompt_binding_verified
                steering_verified = len(cancelled) == 1 and steering_evidence(saved["remote_workspace"].get("steer_receipts", []), prompt_events,
                    requests, devices(), project, cancelled[0]["session_id"])
                provider_update = provider_update and steering_verified
                directory_execution_verified = directory_execution_evidence([json.loads(line) for line in session.read_text().splitlines()],
                    requests, workspace, project, devices())
                provider_update = provider_update and directory_execution_verified
                if args.with_worktree_workflow:
                    provider_update = provider_update and worktree_execution_evidence(
                        [json.loads(line) for line in session.read_text().splitlines()], requests, workspace, project, devices(), worktree_id)
                evidence_path = args.receipt.with_suffix(".commands.json")
                evidence_path.parent.mkdir(parents=True, exist_ok=True)
                evidence_path.write_text(json.dumps({"prompt_bindings":saved["remote_workspace"].get("prompt_bindings", []),
                    "prompt_admissions":prompt_receipts,"progress":progress,
                    "terminal_admissions":[event for event in prompt_events if event.get("type") == "remote_terminal_admission_v1"],
                    "terminal_file_sha256":{name:hashlib.sha256((workspace / name).read_bytes()).hexdigest()
                        for name in ("android-terminal-owned.txt", "android-terminal-reopened.txt") if (workspace / name).is_file()},
                    "terminal_exit_reopen_verified":terminal_open_verified,
                    "worktree_checks":worktree_checks,"worktree_id":worktree_id,
                    "directory_stress_verified":directory_stress_verified,"directory_execution_verified":directory_execution_verified,
                    "prompt_binding_verified":prompt_binding_verified}, indent=2) + "\n")
            if not terminal.checkpoint.exists():
                terminal.write(b"\x1b"); terminal.expect(b"Prompt")
                terminal.wait(lambda: terminal.checkpoint.exists())
            terminal.close()
            extension_verified = False
            extension_artifact = None
            if extension_log is not None:
                extension_events = [json.loads(line) for line in extension_log.read_text().splitlines()]
                extension_verified = extension_workspace_evidence(
                    [json.loads(line) for line in session.read_text().splitlines()], extension_events, workspace)
                extension_path = args.receipt.with_suffix(".extensions.json")
                extension_path.parent.mkdir(parents=True, exist_ok=True)
                extension_path.write_text(json.dumps(extension_events, indent=2) + "\n")
                extension_artifact = {"path":str(extension_path),"sha256":hashlib.sha256(extension_path.read_bytes()).hexdigest()}
                provider_update = provider_update and extension_verified
            family = socket.AF_INET6 if ":" in bind_ip else socket.AF_INET
            with socket.socket(family, socket.SOCK_STREAM) as probe:
                probe.settimeout(1)
                listener_closed = probe.connect_ex(("127.0.0.1" if args.emulator_host_network else bind_ip, port)) != 0
            passed = passed and listener_closed and provider_update
            workspace_verified = args.with_workspace_control and workspace_checks == ["open", "close", "open", "close"]
            if args.with_workspace_control: passed = passed and workspace_verified
            process_cache = False
            cache_outputs = []
            if passed:
                namespace = "process-" + str(time.time_ns())
                previous_pid = None
                revoke_intent = None
                for phase, test_class in [(phase, test_class) for test_class in ("zenpi.CacheProcessTest", "zenpi.RevokeProcessTest") for phase in ("seed", "restore")]:
                    command = ["am", "instrument", "-w", "-r", "-e", "class", test_class,
                        "-e", "cache_phase", phase, "-e", "cache_namespace", namespace]
                    if previous_pid: command += ["-e", "previous_pid", previous_pid]
                    if test_class == "zenpi.RevokeProcessTest" and phase == "restore": command += ["-e", "revoke_intent", revoke_intent]
                    command += ["app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]
                    cache_run = subprocess.run(adb + ["shell", shlex.join(command)], capture_output=True, text=True, timeout=45)
                    cache_outputs.append(cache_run.stdout)
                    if cache_run.returncode != 0 or re.search(r"(?m)^OK \(1 test\)$", cache_run.stdout) is None or "FAILURES" in cache_run.stdout:
                        raise RuntimeError("Independent-process cache validation failed: " + cache_run.stdout)
                    pids = re.findall(r"cache_process_pid=([0-9]+)", cache_run.stdout)
                    if len(pids) != 1: raise RuntimeError("Cache process PID evidence missing")
                    if phase == "seed":
                        if test_class == "zenpi.RevokeProcessTest":
                            intents = re.findall(r"revoke_intent=([0-9a-f-]{36})", cache_run.stdout)
                            if len(intents) != 1: raise RuntimeError("Revocation intent identity evidence missing")
                            revoke_intent = intents[0]
                        previous_pid = pids[0]
                        subprocess.run(adb + ["shell", "am", "force-stop", "app.zenpi.android"], check=True, timeout=10)
                    else: process_cache = pids[0] != previous_pid
            passed = passed and process_cache
            if network_result is not None:
                if revoke_result is None or "zenpi_phase=android_self_revoke_verified" not in revoke_result.stdout:
                    raise RuntimeError("Android self-revocation confirmation missing")
                record = json.loads(identity.with_suffix(".devices.json").read_text())["devices"][0]
                if not record["revoked"]: raise RuntimeError("Android self-revocation did not persist on host")
            screenshot = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/host-projection.png"], capture_output=True, check=True, timeout=10).stdout
            if not screenshot.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing Android screenshot")
            screenshot_path = args.receipt.with_suffix(".png")
            screenshot_path.parent.mkdir(parents=True, exist_ok=True)
            screenshot_path.write_bytes(screenshot)
            if args.lan_ip or args.emulator_host_network: require_no_reverse(adb)
            if harness_inputs != {name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in harness_inputs}:
                raise RuntimeError("Harness dependencies changed during validation")
            receipt = {"passed":passed, "started_at_unix":started, "finished_at_unix":time.time(),
                "serial":args.serial,
                "initial_instrumentation_observation_seconds":args.validation_seconds,
                "invite_ttl_seconds":min(300, args.validation_seconds + 60),
                "android_api":android_api, "expected_android_api":args.expected_api,
                "controlled_emulator_doze_foreground_resume_verified":passed and args.with_doze,
                "instrumentation_outputs": [value.stdout.replace(decoded, "<redacted-invite>")
                    for value in (result, death_result, network_result, revoke_result) if value is not None],
                "scope":"production desktop PTY / same owner observed by Android foreground durable transcript replay",
                "terminal_qr_decoded_verified":True, "camera_scanning_verified":False, "lan_reachability_verified":False,
                "transport_route":"emulator_host_alias" if args.emulator_host_network else "direct_private_interface" if args.lan_ip else "adb_reverse",
                "emulator_host_network_verified":passed and args.emulator_host_network,
                "wifi_association_setup":wifi_setup,
                "advertised_endpoint":values["address"][0],
                "harness_inputs":harness_inputs,
                "lan_interface_tls_verified":bool(args.lan_ip) and passed,
                "physical_lan_peer_verified":False,
                "adb_reverse_used":reverse_created,
                "shared_desktop_owner_update_verified":passed, "durable_transcript_replay_verified":passed,
                "android_worktree_diff_ui_verified":passed and args.with_worktree_workflow,
                "android_worktree_diff_pages_verified":passed and diff_pages_verified,
                "diff_page_workflow_phases":diff_page_phases,
                "diff_patch_sha256":diff_patch_sha256 if args.with_worktree_workflow else None,
                "diff_total_bytes":diff_total_bytes if args.with_worktree_workflow else None,
                "foreground_auto_resume_verified":passed,
                "encrypted_cache_recreation_verified":passed, "independent_process_cache_restore_verified":process_cache,
                "actual_process_death_network_resume_verified":network_resume,
                "process_death_cursor_replay_verified":network_resume and not args.with_workspace_control,
                "process_death_tool_metadata_rehydration_verified":network_resume and args.with_workspace_control,
                "pending_write_process_recovery_verified":network_result is not None and "zenpi_phase=pending_write_process_recovery_verified" in network_result.stdout,
                "killed_live_android_pid":killed_pid,
                "process_death_stdout_sha256":hashlib.sha256(death_result.stdout.encode()).hexdigest() if death_result else None,
                "network_process_stdout_sha256":hashlib.sha256(network_result.stdout.encode()).hexdigest() if network_result else None,
                "killed_live_revoke_android_pid":killed_revoke_pid,
                "revoke_process_stdout_sha256":hashlib.sha256(revoke_result.stdout.encode()).hexdigest() if revoke_result else None,
                "actual_process_death_revoke_receipt_verified":revoke_process_recovery,
                "cache_process_outputs_sha256":[hashlib.sha256(value.encode()).hexdigest() for value in cache_outputs],
                "independent_process_revoke_intent_verified":process_cache and len(cache_outputs) == 4,
                "live_event_push_verified":False, "remote_mutation_verified":workspace_verified and passed,
                "workspace_open_close_desktop_checkpoint_verified":workspace_verified,
                "production_workspace_buttons_verified":"zenpi_phase=production_workspace_buttons_verified" in result.stdout,
                "confirmed_write_read_failure_verified":"zenpi_phase=confirmed_write_read_failure_verified" in result.stdout and passed,
                "android_prompt_shared_runner_verified":"zenpi_phase=android_prompt_shared_runner_verified" in result.stdout and provider_update,
                "android_prompt_terminal_status_verified":"zenpi_phase=android_prompt_terminal_status_verified" in result.stdout and provider_update,
                "android_desktop_terminal_open_input_verified":terminal_open_verified and passed,
                "android_desktop_terminal_exit_reopen_verified":terminal_open_verified and passed,
                "android_prompt_cancel_verified":"zenpi_phase=android_prompt_cancel_verified" in result.stdout and provider_update,
                "cancel_checkpoint_receipt_verified":cancel_checkpoint_verified,
                "android_steer_durable_receipt_and_provider_context_verified":steering_verified and "zenpi_phase=android_steer_durable_receipt_verified" in result.stdout,
                "prompt_global_identity_binding_verified":prompt_binding_verified and passed,
                "android_live_tool_approval_verified":approval_checks == ["allow", "deny"] and provider_update and passed,
                "remote_approval_actor_audit_verified":approval_checks == ["allow", "deny"] and provider_update and passed,
                "android_approval_preview_verified":"zenpi_phase=android_approval_preview_verified" in result.stdout and approval_checks == ["allow", "deny"] and provider_update and passed,
                "default_network_loss_resume_verified":"zenpi_phase=default_network_loss_resume_verified" in result.stdout,
                "stopped_network_observation_fenced_verified":result.stdout.splitlines().count("INSTRUMENTATION_STATUS: zenpi_phase=stopped_network_observation_fenced_verified") == 1 and passed,
                "local_reset_all_owned_jobs_joined_verified":"zenpi_phase=local_reset_all_owned_jobs_joined_verified" in result.stdout and passed,
                "local_reset_retired_jobs_joined_verified":"zenpi_phase=local_reset_retired_jobs_joined_verified" in result.stdout and passed,
                "first_pair_cancel_saved_host_restored_verified":"zenpi_phase=first_pair_cancel_saved_host_restored_verified" in result.stdout and passed,
                "background_pair_saved_host_restored_verified":"zenpi_phase=background_pair_saved_host_restored_verified" in result.stdout and passed,
                "network_pair_saved_host_restored_verified":"zenpi_phase=network_pair_saved_host_restored_verified" in result.stdout and passed,
                "explicit_identity_preconnect_fences_verified":"zenpi_phase=explicit_identity_preconnect_fences_verified" in result.stdout and passed,
                "resume_storage_wait_cancellation_fenced_verified":"zenpi_phase=resume_storage_wait_cancellation_fenced_verified" in result.stdout and passed,
                "foreground_storage_wait_cancellation_fenced_verified":"zenpi_phase=foreground_storage_wait_cancellation_fenced_verified" in result.stdout and passed,
                "foreground_storage_final_admission_fenced_verified":"zenpi_phase=foreground_storage_final_admission_fenced_verified" in result.stdout and passed,
                "revocation_identity_removal_cancellation_fenced_verified":"zenpi_phase=revocation_identity_removal_cancellation_fenced_verified" in result.stdout and passed,
                "device_confirmation_scope_verified":result.stdout.splitlines().count("INSTRUMENTATION_STATUS: zenpi_phase=device_confirmation_scope_verified") == 1 and passed,
                "workmanager_read_resume_verified":"zenpi_phase=workmanager_read_resume_verified" in result.stdout,
                "android_subtab_control_verified":"zenpi_phase=android_subtab_control_verified" in result.stdout,
                "production_subtab_buttons_verified":"zenpi_phase=production_subtab_buttons_verified" in result.stdout,
                "subtab_desktop_checkpoint_verified":subtab_checkpoint_verified,
                "directory_tabs_desktop_checkpoint_verified":directory_checkpoint_verified,
                "directory_order_desktop_checkpoint_verified":directory_order_verified,
                "directory_offline_cache_verified":"zenpi_phase=directory_offline_cache_verified" in result.stdout,
                "directory_selection_stress_verified":directory_stress_verified and passed,
                "directory_execution_provider_tool_context_verified":"zenpi_phase=directory_execution_verified" in result.stdout and directory_execution_verified and passed,
                "extension_workspace_lifecycle_and_tool_output_verified":extension_verified and passed,
                "extension_workspace_artifact":extension_artifact,
                "android_git_worktree_create_execute_remove_verified":args.with_worktree_workflow and worktree_checks == ["create", "dirty_refusal", "remove"] and provider_update and passed,
                "listener_closed_with_desktop":listener_closed, "desktop_provider_requests":len(requests),
                "host_revocation_verified": revoke_result is not None and "zenpi_phase=host_state_verified" in revoke_result.stdout and passed,
                "android_self_revoke_verified":revoke_result is not None and "zenpi_phase=android_self_revoke_verified" in revoke_result.stdout and passed,
                "revoke_receipt_recovery_verified":revoke_process_recovery and passed,
                "environment_names": ["ZENPI_HOME", "NO_PROXY", "no_proxy", "TERM", "NO_COLOR"],
                "no_color_qr_verified":env.get("NO_COLOR") == "1",
                "screenshot":{"path":str(screenshot_path),"sha256":hashlib.sha256(screenshot).hexdigest()},
                "host_argv":terminal.argv, "host_cwd":str(workspace), "seed":seed,
                "requirement":json.loads((ROOT / "Docs/execution/stage2_android2unix_active_requirement.json").read_text()),
                "baseline_revision":subprocess.run(["git","rev-parse","HEAD"],cwd=ROOT,capture_output=True,text=True,check=True).stdout.strip(),
                "integration_state":"provisional uncommitted worktree; no item acceptance",
                "instrumentation_exit_code":result.returncode,
                "instrumentation_classes":list(instrumentation_classes),
                "expected_instrumentation_tests":expected_tests,
                "completed_instrumentation_tests":completed_tests,
                "stdout_sha256":hashlib.sha256(result.stdout.encode()).hexdigest(), "stderr_sha256":hashlib.sha256(result.stderr.encode()).hexdigest(),
                "artifacts":artifacts, "android_build_provenance":build_provenance, "host_build_provenance":host_provenance}
            if require_current_build() != build_provenance:
                raise RuntimeError("Android source/build provenance changed during device validation")
            if require_current_host(args.binary) != host_provenance:
                raise RuntimeError("Host source/build provenance changed during device validation")
            if artifacts != {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()
                for p in (args.binary, apk, test_apk)} or harness_digest != hashlib.sha256(Path(__file__).read_bytes()).hexdigest() \
                or test_verifier_digest != hashlib.sha256(test_verifier.read_bytes()).hexdigest():
                raise RuntimeError("Validation artifacts changed during the run; evidence cannot be attributed")
            receipt["harness_sha256"] = harness_digest
            receipt["instrumentation_verifier_sha256"] = test_verifier_digest
            args.receipt.parent.mkdir(parents=True, exist_ok=True)
            args.receipt.write_text(json.dumps(receipt,indent=2)+"\n")
            print(json.dumps(receipt,indent=2))
            if not passed: raise RuntimeError("shared desktop validation failed")
        except Exception as error:
            # Early process/revocation failures must leave a bounded failed receipt too.
            if not args.receipt.exists():
                sensitive_invite = locals().get("decoded", "<no-invite>")
                failure = {"passed":False, "android_api":android_api, "expected_android_api":args.expected_api,
                    "scope":"production desktop/Android regression interrupted before final acceptance",
                    "failure":str(error).replace(sensitive_invite, "<redacted-invite>"), "artifacts":artifacts, "harness_sha256":harness_digest,
                    "started_at_unix":started, "finished_at_unix":time.time(),
                    "integration_state":"provisional uncommitted worktree; no item acceptance"}
                if identity.exists() and identity.with_suffix(".devices.json").exists():
                    failure["device_registry"] = json.loads(identity.with_suffix(".devices.json").read_text())
                sensitive_invite = locals().get("decoded", "<no-invite>")
                observations = failed_instrumentation_evidence(error,
                    (locals().get("result"), locals().get("death_result"), locals().get("network_result"), locals().get("revoke_result")), sensitive_invite)
                failure["instrumentation_observations"] = observations
                failure["instrumentation_outputs"] = [value["stdout"] for value in observations]
                args.receipt.parent.mkdir(parents=True, exist_ok=True)
                args.receipt.write_text(json.dumps(failure, indent=2) + "\n")
            raise
        finally:
            if reverse_created: subprocess.run(adb+["reverse","--remove",f"tcp:{port}"],capture_output=True,timeout=10)
            if terminal is not None: terminal.cleanup()
            if server is not None: server.shutdown(); server.server_close()


if __name__ == "__main__": serialized_device_validation(main)
