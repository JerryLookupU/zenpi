#!/usr/bin/env python3
"""Route admission checks; the production TLS path is verified separately."""
import argparse
import subprocess
import unittest
import copy
import hashlib
import json
import re
import tempfile
from pathlib import Path
from unittest.mock import patch
from stage2_desktop_android_smoke import private_lan_ip, require_no_reverse, force_stop_live_android, approval_evidence, prompt_binding_evidence, directory_stress_evidence, steering_evidence, directory_execution_evidence, extension_workspace_evidence, worktree_execution_evidence
from stage2_desktop_android_smoke import instrumentation_suite, DIFF_PAGE_PHASES, diff_page_workflow_evidence
from stage2_desktop_android_smoke import terminal_open_evidence
from stage2_desktop_android_smoke import failed_instrumentation_evidence
from stage2_desktop_android_smoke import process_recovery_evidence
from stage2_desktop_android_smoke import prepare_emulator_host_network
from stage2_android_pair_smoke import InstrumentationFailure


class EmulatorHostRouteTests(unittest.TestCase):
    def test_existing_reverse_fails_before_any_association(self):
        with patch("stage2_desktop_android_smoke.require_no_reverse", side_effect=RuntimeError("reverse")), \
                patch("stage2_desktop_android_smoke.connect_emulator_wifi") as wifi:
            with self.assertRaises(RuntimeError): prepare_emulator_host_network(["adb"])
            wifi.assert_not_called()

    def test_observed_nonemulator_refuses_and_does_not_change_network(self):
        with patch("stage2_desktop_android_smoke.subprocess.run", side_effect=[
                subprocess.CompletedProcess([],0,"", ""), subprocess.CompletedProcess([],0,"0", "")]) as run:
            with self.assertRaises(ValueError): prepare_emulator_host_network(["adb"])
            self.assertEqual(2, run.call_count)
            self.assertEqual(["adb", "shell", "getprop", "ro.kernel.qemu"], run.call_args.args[0])

    def test_explicit_alias_preparation_uses_empty_reverse_and_owned_wifi_only(self):
        with patch("stage2_desktop_android_smoke.subprocess.run", side_effect=[
                subprocess.CompletedProcess([],0,"", ""), subprocess.CompletedProcess([],0,"1", ""),
                subprocess.CompletedProcess([],0,"", ""),
                subprocess.CompletedProcess([],0,"10.0.2.0/24 dev wlan0 proto kernel scope link src 10.0.2.16\n", "")]) as run:
            self.assertEqual("shell", prepare_emulator_host_network(["adb"]))
            self.assertEqual(["adb", "reverse", "--list"], run.call_args_list[0].args[0])
            self.assertIn(["adb", "shell", "cmd", "wifi", "connect-network", "AndroidWifi", "open"],
                [call.args[0] for call in run.call_args_list])

    def test_alias_preparation_waits_for_wlan_route_after_async_association(self):
        with patch("stage2_desktop_android_smoke.subprocess.run", side_effect=[
                subprocess.CompletedProcess([],0,"", ""), subprocess.CompletedProcess([],0,"1", ""),
                subprocess.CompletedProcess([],0,"", ""),
                subprocess.CompletedProcess([],0,"10.0.0.0/8 dev eth0\n", ""),
                subprocess.CompletedProcess([],0,"10.0.2.0/24 dev wlan0\n", "")]) as run:
            self.assertEqual("shell", prepare_emulator_host_network(["adb"]))
            self.assertEqual(["adb", "shell", "ip", "route"], run.call_args_list[-1].args[0])


class ProcessRecoveryEvidenceTests(unittest.TestCase):
    def test_replay_and_tool_rehydration_are_distinct_and_advance_one_cursor(self):
        for metadata in (False, True):
            phase = "network_process_tool_metadata_rehydrated_verified" if metadata else "network_process_cursor_replay_verified"
            output = f"INSTRUMENTATION_STATUS: zenpi_phase={phase}\nINSTRUMENTATION_STATUS: network_process_seed_cursor=4\nINSTRUMENTATION_STATUS: network_process_recovered_cursor=6\n"
            self.assertTrue(process_recovery_evidence(output, metadata))
            self.assertFalse(process_recovery_evidence(output, not metadata))
            for invalid in (output + output, output.replace("seed_cursor=4", "seed_cursor=06"),
                            output.replace("recovered_cursor=6", "recovered_cursor=4"),
                            output.replace("recovered_cursor=6", "recovered_cursor=3"),
                            output.replace("recovered_cursor=6", "recovered_cursor=18446744073709551616"),
                            output.replace("recovered_cursor=6", "recovered_cursor=invalid"),
                            output.replace(f"zenpi_phase={phase}", "zenpi_phase=network_process_resume_verified")):
                self.assertFalse(process_recovery_evidence(invalid, metadata))


class FailedInstrumentationEvidenceTests(unittest.TestCase):
    def test_timeout_retains_partial_stdout_stderr_and_redacts_invite(self):
        invite = "zenpi://pair?private-token"
        result = subprocess.CompletedProcess([], -9, "phase\n" + invite, "stderr " + invite)
        error = InstrumentationFailure(result, timed_out=True)
        expected = [{"exit_code":-9,"stdout":"phase\n<redacted-invite>","stderr":"stderr <redacted-invite>","timed_out":True}]
        self.assertEqual(expected, failed_instrumentation_evidence(error, (None,), invite))
        self.assertEqual(expected, failed_instrumentation_evidence(error, (result,), invite))

    def test_retains_prior_invocations_without_claiming_timeout(self):
        first = subprocess.CompletedProcess([], 0, "prior", "")
        failed = subprocess.CompletedProcess([], 1, "partial", "failed")
        values = failed_instrumentation_evidence(InstrumentationFailure(failed), (first,None), "")
        self.assertEqual([0,1], [value["exit_code"] for value in values])
        self.assertEqual(["prior","partial"], [value["stdout"] for value in values])
        self.assertFalse(any(value["timed_out"] for value in values))
        self.assertEqual([], failed_instrumentation_evidence(RuntimeError("startup"), (None,), ""))


class TerminalOpenEvidenceTests(unittest.TestCase):
    def test_desktop_file_effect_and_two_durable_receipts_must_match(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "android-terminal-owned.txt"
            path.write_bytes(b"ANDROID_TERMINAL_OWNED\n")
            (root / "android-terminal-reopened.txt").write_bytes(b"ANDROID_TERMINAL_REOPENED\n")
            bindings, events = [], []
            for index, operation in enumerate(("terminal_open", "terminal_input", "terminal_input", "terminal_open", "terminal_input")):
                binding = {"operation": operation, "device_id": "device", "project_id": "project", "session_id": "session",
                    "request_id": str(index), "fingerprint": "a" * 64}
                bindings.append(binding)
                value = {"request_id": str(index), "project_id": "project", "session_id": "session", "terminal_id": ("b" if index < 3 else "d") * 64,
                    "subtab_id": "c" * 64, "subtab_revision": 1}
                if operation == "terminal_input": value["accepted_bytes"] = 5 if index == 2 else 90
                events.extend([{**binding, "type": "remote_terminal_admission_v1", "state": state, "result": value}
                    for state in ("reserved", "succeeded")])
            self.assertTrue(terminal_open_evidence(bindings, events, root, "device", "project"))
            for field in ("operation", "device_id", "request_id", "fingerprint"):
                bad = copy.deepcopy(events); bad[3][field] = "foreign"
                self.assertFalse(terminal_open_evidence(bindings, bad, root, "device", "project"))
            for field in ("terminal_id", "session_id", "project_id", "accepted_bytes", "subtab_id", "subtab_revision"):
                bad = copy.deepcopy(events); bad[3]["result"][field] = "foreign"
                self.assertFalse(terminal_open_evidence(bindings, bad, root, "device", "project"))
            for bad in (events[:-1], events + events[:1], list(reversed(events))):
                self.assertFalse(terminal_open_evidence(bindings, bad, root, "device", "project"))
            bad = copy.deepcopy(events); bad[9]["result"]["terminal_id"] = "b" * 64
            self.assertFalse(terminal_open_evidence(bindings, bad, root, "device", "project"))
            bad = copy.deepcopy(events); bad[5]["result"]["accepted_bytes"] = 90
            self.assertFalse(terminal_open_evidence(bindings, bad, root, "device", "project"))
            bad = copy.deepcopy(bindings); bad[4]["request_id"] = bad[3]["request_id"]
            self.assertFalse(terminal_open_evidence(bad, events, root, "device", "project"))
            close = {"operation":"terminal_input", "device_id":"device", "project_id":"project", "session_id":"session",
                "request_id":"initial-close", "fingerprint":"a" * 64}
            result = {"request_id":"initial-close", "project_id":"project", "session_id":"session", "terminal_id":"e" * 64,
                "subtab_id":"c" * 64, "subtab_revision":1, "accepted_bytes":len(b"exit\r")}
            closing = [{**close, "type":"remote_terminal_admission_v1", "state":state, "result":result}
                for state in ("reserved", "succeeded")]
            self.assertTrue(terminal_open_evidence([close, *bindings], closing + events, root, "device", "project"))
            for field, value in (("terminal_id","b" * 64), ("terminal_id","d" * 64), ("accepted_bytes",90),
                                 ("subtab_revision",2), ("session_id","foreign")):
                bad = copy.deepcopy(closing + events); bad[1]["result"][field] = value
                self.assertFalse(terminal_open_evidence([close, *bindings], bad, root, "device", "project"))
            self.assertFalse(terminal_open_evidence([close, close, *bindings], closing + closing + events, root, "device", "project"))
            self.assertFalse(terminal_open_evidence([close, *bindings], events, root, "device", "project"))
            (root / "android-terminal-reopened.txt").write_bytes(b"wrong\n")
            self.assertFalse(terminal_open_evidence(bindings, events, root, "device", "project"))
            (root / "android-terminal-reopened.txt").write_bytes(b"ANDROID_TERMINAL_REOPENED\n")
            path.write_bytes(b"wrong\n")
            self.assertFalse(terminal_open_evidence(bindings, events, root, "device", "project"))


class DiffPageWorkflowEvidenceTests(unittest.TestCase):
    def test_complete_ordered_workflow_binds_android_content_to_host_patch(self):
        digest, total = "a" * 64, 100000
        phases = list(DIFF_PAGE_PHASES)
        output = "\n".join([f"INSTRUMENTATION_STATUS: zenpi_phase={phase}" for phase in phases]
            + [f"INSTRUMENTATION_STATUS: zenpi_diff_patch_sha256={digest}", f"INSTRUMENTATION_STATUS: zenpi_diff_total_bytes={total}"])
        self.assertTrue(diff_page_workflow_evidence(phases, digest, total, output))
        for invalid in (phases[1:], list(reversed(phases)), phases + [phases[0]]):
            self.assertFalse(diff_page_workflow_evidence(invalid, digest, total, output))
        for invalid in (output.replace(digest, "b" * 64), output.replace(str(total), "1"),
                        output + "\n" + output, output.replace(phases[1], "missing"),
                        output + "\nINSTRUMENTATION_STATUS: zenpi_diff_patch_sha256=" + "b" * 64,
                        output.replace(phases[0], "TEMP").replace(phases[-1], phases[0]).replace("TEMP", phases[-1])):
            self.assertFalse(diff_page_workflow_evidence(phases, digest, total, invalid))
        for total in (True, 65536, 1048577):
            self.assertFalse(diff_page_workflow_evidence(phases, digest, total, output))


class InstrumentationSuiteTests(unittest.TestCase):
    def test_exact_suite_includes_current_boundary_classes_and_matches_source_methods(self):
        root = Path(__file__).resolve().parents[1]
        required = {"zenpi.WorkspaceFilesScreenTest", "zenpi.TerminalControlScreenTest", "zenpi.TerminalHistoryScreenTest",
                    "zenpi.WorkerScreenTest", "zenpi.WorkerStatusPageScreenTest", "zenpi.ResourceCatalogScreenTest",
                    "zenpi.ModelStatusScreenTest", "zenpi.DiffScreenTest", "zenpi.TranscriptToolScreenTest"}
        for workspace in (False, True):
            for doze in (False, True):
                classes, expected = instrumentation_suite(workspace, doze)
                self.assertEqual(len(classes), len(set(classes)))
                self.assertEqual("zenpi.DozeLifecycleTest" in classes, doze)
                self.assertEqual(required.issubset(classes), workspace)
                count = 0
                for name in classes:
                    source = (root / "android/app/src/androidTest/java" / (name.replace(".", "/") + ".kt")).read_text()
                    count += len(re.findall(r"(?m)^\s*@Test\b", source))
                self.assertEqual(expected, count, classes)


class WorktreeExecutionEvidenceTests(unittest.TestCase):
    def test_admission_root_tool_output_provider_and_answer_must_all_agree(self):
        workspace = Path("/fixture/workspace")
        records = [{"event":{"type":"remote_prompt_admission_v1","state":"accepted","device_id":"device",
            "result":{"project_id":"project"},"execution_subtab":{"id":"tab","kind":"worktree","root":str(workspace / ".zenpi-worktrees/android-feature")}}},
            {"kind":"turn","turn":{"role":"tool","metadata":{"tool_call_id":"read-STAGE2_ANDROID_WORKTREE_READ"},"content":"WORKTREE_ONLY_CONTENT"}},
            {"kind":"turn","turn":{"role":"assistant","content":"STAGE2_ANDROID_WORKTREE_READ_DONE"}}]
        requests = [{"input":[{"type":"function_call_output","call_id":"read-STAGE2_ANDROID_WORKTREE_READ","output":"WORKTREE_ONLY_CONTENT"}]}]
        self.assertTrue(worktree_execution_evidence(records, requests, workspace, "project", "device", "tab"))
        for index in range(3):
            self.assertFalse(worktree_execution_evidence(records[:index] + records[index + 1:], requests, workspace, "project", "device", "tab"))
        for field in ("id", "kind", "root"):
            invalid = copy.deepcopy(records); invalid[0]["event"]["execution_subtab"][field] = "wrong"
            self.assertFalse(worktree_execution_evidence(invalid, requests, workspace, "project", "device", "tab"))
        invalid = copy.deepcopy(records); invalid[1]["turn"]["content"] = "TRACKED_WORKTREE_CONTENT"
        self.assertFalse(worktree_execution_evidence(invalid, requests, workspace, "project", "device", "tab"))
        self.assertFalse(worktree_execution_evidence(records, [], workspace, "project", "device", "tab"))


class ExtensionWorkspaceEvidenceTests(unittest.TestCase):
    def test_initialize_hooks_tool_output_and_restoration_must_agree(self):
        workspace = Path("/fixture/workspace")
        records, events = [], []
        for index, name in enumerate(("directory-one", "directory-two")):
            generation, recovery = index * 2 + 1, index * 2 + 2
            records.extend([{"event":{"type":"extensions_workspace_selected","workspace_root":str(workspace / name),
                "generation":generation,"restore_generation":recovery}},
                {"event":{"type":"extensions_workspace_restored","generation":recovery}}])
            for hook in ("initialize", "session_start", "agent_start", "before_tool", "tools/call", "after_tool", "agent_end", "session_close"):
                events.append({"hook":hook,"generation":generation,"workspace":str(workspace / name),
                    "contents":f"must survive tab close: {name}\n" if hook == "tools/call" else None})
            for hook in ("initialize", "session_start", "session_close"):
                events.append({"hook":hook,"generation":recovery,"workspace":str(workspace)})
        self.assertTrue(extension_workspace_evidence(records, events, workspace))
        for index in range(len(events)):
            self.assertFalse(extension_workspace_evidence(records, events[:index] + events[index + 1:], workspace))
            invalid = copy.deepcopy(events); invalid[index]["workspace"] = "wrong"
            self.assertFalse(extension_workspace_evidence(records, invalid, workspace))
        invalid = copy.deepcopy(events); invalid[4]["contents"] = "wrong root"
        self.assertFalse(extension_workspace_evidence(records, invalid, workspace))
        self.assertFalse(extension_workspace_evidence(records, events + [events[0]], workspace))
        self.assertFalse(extension_workspace_evidence(records[1:], events, workspace))
        invalid = copy.deepcopy(records); invalid[2]["event"]["restore_generation"] = 2
        self.assertFalse(extension_workspace_evidence(invalid, events, workspace))


class DirectoryExecutionEvidenceTests(unittest.TestCase):
    def test_selection_journal_tool_result_and_provider_output_must_agree(self):
        records, requests = [], []
        workspace = Path("/fixture/workspace")
        for name, marker in (("directory-one", "STAGE2_ANDROID_DIRECTORY_ONE"), ("directory-two", "STAGE2_ANDROID_DIRECTORY_TWO")):
            records.append({"event":{"type":"remote_prompt_admission_v1","state":"accepted","device_id":"device",
                "result":{"project_id":"project"}, "execution_subtab":{"root":str(workspace / name),"id":"a" * 64,"kind":"in_place"}}})
            content = f"must survive tab close: {name}"
            records.append({"kind":"turn","turn":{"role":"tool","metadata":{"tool_call_id":"read-" + marker},"content":content}})
            records.append({"kind":"turn","turn":{"role":"assistant","content":marker + "_DONE"}})
            requests.append({"input":[{"type":"function_call_output","call_id":"read-" + marker,"output":content}]})
        self.assertTrue(directory_execution_evidence(records, requests, workspace, "project", "device"))
        for field in ("root", "id", "kind"):
            invalid = copy.deepcopy(records); invalid[0]["event"]["execution_subtab"][field] = "wrong"
            self.assertFalse(directory_execution_evidence(invalid, requests, workspace, "project", "device"))
        for invalid in (records[1:], records + [records[1]]):
            self.assertFalse(directory_execution_evidence(invalid, requests, workspace, "project", "device"))
        invalid = copy.deepcopy(records); invalid[1]["turn"]["content"] = "must survive tab close: directory-two"
        self.assertFalse(directory_execution_evidence(invalid, requests, workspace, "project", "device"))
        self.assertFalse(directory_execution_evidence(records, [], workspace, "project", "device"))


class SteeringEvidenceTests(unittest.TestCase):
    def test_receipt_queue_and_provider_context_must_all_agree(self):
        receipt = {"device_id":"device", "request_id":"steer", "fingerprint":"a" * 64,
            "project_id":"project", "session_id":"session", "target_request_id":"prompt", "state":"accepted", "code":None}
        input_id = hashlib.sha256(b"remote-steer-v1\ndevice\nsteer").hexdigest()
        events = [{"type":"input_queue", "change":{"action":"enqueued", "input":{"id":input_id, "kind":"steer", "text":"STAGE2_ANDROID_PROMPT_STEER"}}},
            {"type":"input_queue", "change":{"action":"applied", "kind":"steer", "ids":[input_id]}},
            {"type":"remote_prompt_admission_v1", "state":"accepted", "request_id":"prompt", "device_id":"device"}]
        requests = [{"input":[{"role":"user","content":[{"type":"input_text","text":"STAGE2_ANDROID_PROMPT_REQUEST"}]},
            {"role":"user","content":"STAGE2_ANDROID_PROMPT_STEER"}]}]
        self.assertTrue(steering_evidence([receipt], events, requests, "device", "project", "session"))
        for field in receipt:
            invalid = dict(receipt); invalid[field] = "wrong"
            self.assertFalse(steering_evidence([invalid], events, requests, "device", "project", "session"))
        self.assertFalse(steering_evidence([receipt], events + [events[0]], requests, "device", "project", "session"))
        self.assertFalse(steering_evidence([receipt], events, [], "device", "project", "session"))
        for index in (0, 1):
            invalid = copy.deepcopy(events); invalid[index]["change"]["kind" if index else "input"] = "wrong"
            if index == 0:
                invalid[index]["change"]["input"] = {"id":"wrong"}
            self.assertFalse(steering_evidence([receipt], invalid, requests, "device", "project", "session"))


class DirectoryStressEvidenceTests(unittest.TestCase):
    def test_independent_receipts_must_be_contiguous_unique_and_target_correct(self):
        before = {"remote_workspace":{"revision":5,"receipts":[{"result":{"required_owner":"owner"}}]},
            "subtabs":{"project":{"revision":3,"active_id":"two","tabs":[{"name":"directory-one","id":"one"},{"name":"directory-two","id":"two"}]}}}
        after = copy.deepcopy(before)
        after["remote_workspace"]["revision"] += 12
        after["subtabs"]["project"]["revision"] += 12
        for index in range(12):
            after["remote_workspace"]["receipts"].append({"device_id":"device","request_id":str(index),"fingerprint":"a" * 64,
                "result":{"project_id":"project","active_project_id":"project","required_owner":"owner", "workspace_revision":6 + index,
                    "subtab_revision":4 + index,"active_subtab_id":"one" if index % 2 == 0 else "two"}})
        self.assertTrue(directory_stress_evidence(before, after, "device", "project"))
        for change in (lambda value: value["remote_workspace"].update(revision=99),
            lambda value: value["remote_workspace"]["receipts"][-1].update(request_id="0"),
            lambda value: value["remote_workspace"]["receipts"][-1]["result"].update(active_subtab_id="one"),
            lambda value: value["remote_workspace"]["receipts"][-1]["result"].update(subtab_revision=99),
            lambda value: value["remote_workspace"]["receipts"][-1].update(device_id="foreign"),
            lambda value: value["subtabs"]["project"].update(active_id="one")):
            invalid = copy.deepcopy(after); change(invalid)
            self.assertFalse(directory_stress_evidence(before, invalid, "device", "project"))


class PromptBindingEvidenceTests(unittest.TestCase):
    def test_checkpoint_bindings_must_match_unique_journal_admissions(self):
        bindings = [{"device_id":"device", "request_id":str(index), "fingerprint":"a" * 64,
            "project_id":"project", "session_id":"session"} for index in range(3)]
        events = [dict(value, type="remote_prompt_admission_v1", state="accepted") for value in bindings]
        self.assertTrue(prompt_binding_evidence(bindings, events, "device", "project", "session"))
        for field in ("device_id", "project_id", "session_id", "fingerprint", "request_id"):
            bad = copy.deepcopy(bindings); bad[0][field] = "wrong"
            self.assertFalse(prompt_binding_evidence(bad, events, "device", "project", "session"))
        for bad in (bindings[:2], bindings + [bindings[0]], [bindings[0]] * 3):
            self.assertFalse(prompt_binding_evidence(bad, events, "device", "project", "session"))
        bad = copy.deepcopy(events); bad[0]["state"] = "reserved"
        self.assertFalse(prompt_binding_evidence(bindings, bad, "device", "project", "session"))


class ApprovalEvidenceTests(unittest.TestCase):
    def test_tool_side_effect_admission_and_identity_must_all_match(self):
        receipts = [{"decision":choice, "digest":"a" * 64, "binding":{"state":"accepted", "code":None, "device_id":"device",
            "project_id":"project", "session_id":"session", "target_request_id":choice, "fingerprint":"b" * 64,
            "request_id":"12345678-1234-1234-1234-123456789abc"}} for choice in ("allow", "deny")]
        events = [{"type":"approval_resolved", "call_id":"approval-" + choice, "request_id":choice, "decision":choice,
            "remember":False, "source":"host_answer"} for choice in ("allow", "deny")]
        events.append({"type":"approval_consumed", "call_id":"approval-allow", "decision":"allow", "request_id":"allow"})
        for receipt, event in zip(receipts, events):
            event["remote"] = {field:receipt["binding"][field] for field in ("device_id", "request_id", "fingerprint", "project_id", "session_id")} | {"request_digest":receipt["digest"]}
        self.assertTrue(approval_evidence(receipts, events, "device", "project", "session"))
        for field in ("device_id", "project_id", "session_id", "target_request_id", "state", "fingerprint"):
            bad = copy.deepcopy(receipts); bad[0]["binding"][field] = "wrong"
            self.assertFalse(approval_evidence(bad, events, "device", "project", "session"))
        for change in (lambda data: data.append({"type":"approval_consumed", "call_id":"approval-deny", "decision":"deny"}),
            lambda data: data[0].update(remember=True), lambda data: data[0].update(source="worker_allow_after_preflight"),
            lambda data: data[0].pop("remote"), lambda data: data[0]["remote"].update(device_id="foreign"),
            lambda data: data[0]["remote"].update(request_digest="f" * 64), lambda data: data[2].update(request_id="foreign")):
            bad = copy.deepcopy(events); change(bad)
            self.assertFalse(approval_evidence(receipts, bad, "device", "project", "session"))


class DirectRouteTests(unittest.TestCase):
    def test_only_literal_rfc1918_addresses_are_admitted(self):
        for value in ("10.0.2.2", "172.16.0.1", "172.31.255.254", "192.168.50.14"):
            self.assertEqual(private_lan_ip(value), value)
        self.assertEqual(private_lan_ip("FD00::0012"), "fd00::12")
        for value in ("127.0.0.1", "0.0.0.0", "8.8.8.8", "169.254.1.1", "172.32.0.1", "224.0.0.1", "::1", "fe80::1", "192.0.2.1", "host.local", "192.168.50.14:1234"):
            with self.subTest(value=value), self.assertRaises(argparse.ArgumentTypeError):
                private_lan_ip(value)

    def test_existing_reverse_mapping_fails_without_removing_it(self):
        adb = ["adb", "-s", "fixture"]
        with patch("stage2_desktop_android_smoke.subprocess.run", return_value=subprocess.CompletedProcess([], 0, "fixture tcp:1234 tcp:1234\n")) as run:
            with self.assertRaises(RuntimeError): require_no_reverse(adb)
            run.assert_called_once_with(adb + ["reverse", "--list"], capture_output=True, text=True, check=True, timeout=10)

    def test_empty_reverse_table_is_required(self):
        with patch("stage2_desktop_android_smoke.subprocess.run", return_value=subprocess.CompletedProcess([], 0, "\n")):
            require_no_reverse(["adb", "-s", "fixture"])

    def test_adb_failure_is_not_evidence_of_absent_forwarding(self):
        with patch("stage2_desktop_android_smoke.subprocess.run", side_effect=subprocess.CalledProcessError(1, "adb")):
            with self.assertRaises(subprocess.CalledProcessError): require_no_reverse(["adb"])


class ProcessDeathTests(unittest.TestCase):
    def test_exact_live_pid_then_absent_process_proves_force_stop(self):
        adb = ["adb", "-s", "fixture"]
        with patch("stage2_desktop_android_smoke.subprocess.run", side_effect=[
            subprocess.CompletedProcess([], 0, "1234\n"),
            subprocess.CompletedProcess([], 0, ""),
            subprocess.CompletedProcess([], 1, ""),
        ]) as run:
            self.assertEqual(force_stop_live_android(adb), "1234")
            self.assertEqual(run.call_args_list[1].args[0], adb + ["shell", "am", "force-stop", "app.zenpi.android"])

    def test_missing_ambiguous_or_failed_pid_lookup_never_kills(self):
        for code, output in [(1, ""), (0, "12 34"), (0, ""), (2, "1234")]:
            with self.subTest(code=code, output=output), patch("stage2_desktop_android_smoke.subprocess.run",
                return_value=subprocess.CompletedProcess([], code, output)) as run:
                with self.assertRaises(RuntimeError): force_stop_live_android(["adb"])
                self.assertEqual(run.call_count, 1)

    def test_remaining_pid_or_failed_observation_does_not_prove_death(self):
        for code, output in [(0, "1234"), (2, "")]:
            with self.subTest(code=code, output=output), patch("stage2_desktop_android_smoke.subprocess.run", side_effect=[
                subprocess.CompletedProcess([], 0, "1234"),
                subprocess.CompletedProcess([], 0, ""),
                subprocess.CompletedProcess([], code, output),
            ]):
                with self.assertRaises(RuntimeError): force_stop_live_android(["adb"])


if __name__ == "__main__": unittest.main()
