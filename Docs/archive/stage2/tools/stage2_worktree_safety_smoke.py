#!/usr/bin/env python3
"""Production desktop PTY: failed Git close retains tab/files; clean close succeeds."""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from tui_project_workspace_smoke import Terminal

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/zenpi")
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    started = time.time()
    with tempfile.TemporaryDirectory(prefix="zenpi-worktree-safety-") as raw:
        root = Path(raw)
        project = root / "initial"
        home = root / "config"
        for path in (project, home, root / "sessions"): path.mkdir()
        def git(*argv):
            result = subprocess.run(["git", "-C", str(project), *argv], capture_output=True, text=True, timeout=10)
            if result.returncode: raise RuntimeError("fixture Git failed: " + result.stderr[:1000])
        git("init", "-q"); git("config", "user.name", "fixture"); git("config", "user.email", "fixture@example.invalid")
        (project / "tracked.txt").write_text("original\n")
        (project / ".gitignore").write_text("ignored-local.txt\n")
        git("add", "tracked.txt", ".gitignore"); git("commit", "-qm", "initial")
        (home / "config.toml").write_text('backend="openai"\nmodel="gpt-4.1"\nbase_url="http://127.0.0.1:1/v1"\nwire_api="responses"\nrequires_openai_auth=false\nmax_retries=0\n')
        env = {key: value for key, value in os.environ.items() if not key.startswith(("ZENPI_", "OPENAI_"))}
        env.update(ZENPI_HOME=str(home), TERM="xterm-256color", NO_COLOR="1", NO_PROXY="127.0.0.1,localhost", no_proxy="127.0.0.1,localhost")
        env.update(GIT_DIR=str(root / "foreign-missing.git"), GIT_CONFIG_COUNT="1",
                   GIT_CONFIG_KEY_0="core.hooksPath", GIT_CONFIG_VALUE_0=str(root / "foreign-hooks"))
        terminal = None
        try:
            terminal = Terminal(binary, root, env)
            def command(text, expect=None, timeout=12):
                terminal.command(text)
                terminal.wait(lambda: terminal.draft() == "", timeout=timeout)
                if expect: terminal.expect(expect, timeout=timeout)
            command("/worktree add feature")
            checkpoint = root / "sessions/project-workspace.json"
            def domain():
                value = json.loads(checkpoint.read_text())
                project_id = value["workspace"]["active"]
                return value["subtabs"][project_id]
            terminal.wait(lambda: len(domain()["tabs"]) == 2)
            before = domain()
            target = Path(before["tabs"][1]["root"])
            (target / "tracked.txt").write_text("edited must survive\n")
            command("/worktree close 1", b"worktree close refused")
            terminal.wait(lambda: domain() == before)
            assert (target / "tracked.txt").read_text() == "edited must survive\n"
            tracked_preserved = True
            subprocess.run(["git", "-C", str(target), "checkout", "--", "tracked.txt"], check=True, capture_output=True, timeout=10)
            (target / "untracked.txt").write_text("untracked must survive\n")
            command("/worktree close 1", b"worktree close refused")
            terminal.wait(lambda: domain() == before)
            assert (target / "untracked.txt").read_text() == "untracked must survive\n"
            untracked_preserved = True
            (target / "untracked.txt").unlink()
            (target / "ignored-local.txt").write_text("ignored must survive\n")
            command("/worktree close 1", b"worktree close refused")
            terminal.wait(lambda: domain() == before)
            assert (target / "ignored-local.txt").read_text() == "ignored must survive\n"
            ignored_preserved = True
            (target / "ignored-local.txt").unlink()
            git("worktree", "lock", str(target))
            command("/worktree close 1", b"worktree close refused")
            terminal.wait(lambda: domain() == before)
            assert target.is_dir()
            locked_preserved = True
            git("worktree", "unlock", str(target))
            command("/worktree close 1")
            terminal.wait(lambda: len(domain()["tabs"]) == 1)
            assert not target.exists()
            git("show-ref", "--verify", "refs/heads/feature")
            before = domain()
            hook = project / ".git/hooks/post-checkout"
            hook.write_text('#!/bin/sh\nhead -c 262145 /dev/zero >&2\nsleep 30\n')
            hook.chmod(0o700)
            flood_started = time.monotonic()
            command("/worktree add flood", timeout=8)
            terminal.expect(b"git output exceeded the bound", timeout=8)
            terminal.wait(lambda: domain() == before)
            flood_target = project / ".zenpi-worktrees/flood"
            assert flood_target.is_dir(), "unknown Git outcome must not trigger automatic deletion"
            assert time.monotonic() - flood_started < 8
            hook.unlink()
            git("worktree", "remove", "--", str(flood_target))
            pid_path = root / "hook.pid"
            hook.write_text('#!/bin/sh\necho $$ > "' + str(pid_path) + '"\nsleep 30\n')
            hook.chmod(0o700)
            timeout_started = time.monotonic()
            command("/worktree add stalled", timeout=22)
            terminal.expect(b"git deadline exceeded", timeout=22)
            terminal.wait(lambda: domain() == before)
            stalled_target = project / ".zenpi-worktrees/stalled"
            assert stalled_target.is_dir()
            timeout_elapsed = time.monotonic() - timeout_started
            assert 14 <= timeout_elapsed < 22, timeout_elapsed
            hook_pid = int(pid_path.read_text())
            try:
                os.kill(hook_pid, 0)
            except ProcessLookupError:
                pass
            else:
                raise AssertionError("timed-out Git hook is still alive")
            hook.unlink()
            git("worktree", "remove", "--", str(stalled_target))
            command("/worktree add after-timeout")
            terminal.wait(lambda: len(domain()["tabs"]) == 2)
            command("/worktree close 1")
            terminal.wait(lambda: len(domain()["tabs"]) == 1)
            terminal.close()
            receipt = {"passed": True, "scope": "production desktop managed worktree close safety; no remote Git capability",
                "started_at_unix": started, "finished_at_unix": time.time(),
                "tracked_dirty_refusal_verified": tracked_preserved, "untracked_refusal_verified": untracked_preserved,
                "locked_refusal_verified": locked_preserved, "failed_close_tab_and_selection_retained": True,
                "ignored_file_refusal_verified": ignored_preserved,
                "clean_close_verified": True, "branch_retained_verified": True,
                "git_hook_stderr_limit_verified": True, "git_hook_deadline_verified": True,
                "git_hook_process_terminated_verified": True,
                "ambient_git_repository_and_config_injection_ignored": True,
                "unknown_git_creation_keeps_checkpoint_and_partial_directory": True,
                "desktop_usable_after_git_timeout": True, "git_timeout_elapsed_seconds": timeout_elapsed,
                "remote_git_control_verified": False, "crash_atomicity_verified": False,
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                "terminal_stdout_sha256": hashlib.sha256(terminal.output).hexdigest(),
                "integration_state": "provisional uncommitted worktree; no item acceptance"}
            args.receipt.parent.mkdir(parents=True, exist_ok=True)
            args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
            print(json.dumps(receipt, indent=2))
        finally:
            if terminal is not None: terminal.cleanup()


if __name__ == "__main__": main()
