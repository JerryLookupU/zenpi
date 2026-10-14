#!/usr/bin/env python3
"""ZE-255: a TUI panic must restore the terminal through the panic hook.

Runs the debug binary in a PTY with ZENPI_DEBUG_PANIC_AFTER_START=1 (the guard
is forgotten before the panic, so only the hook can restore the terminal) and
asserts that the alternate screen is left and termios/`stty -a` match the
pre-launch state.
"""
from __future__ import annotations

import argparse
import fcntl
import json
import os
import select
import struct
import subprocess
import sys
import termios
import time
from pathlib import Path
from tempfile import TemporaryDirectory

LEAVE_ALT = b"\x1b[?1049l"
ENTER_ALT = b"\x1b[?1049h"


# The shell is the session leader and stays alive across the TUI's exit, so
# both `stty -a` snapshots are taken on the live controlling terminal (macOS
# revokes the slave once the session leader exits).
HARNESS = 'stty -a > "$ZE255_DIR/before"; "$@"; code=$?; stty -a > "$ZE255_DIR/after"; exit $code'


def lflags(snapshot: str) -> set[str]:
    line = next(row for row in snapshot.splitlines() if row.startswith("lflags:"))
    return set(line.split()[1:])


def comparable(snapshot: str) -> list[str]:
    # BSD kernels set PENDIN themselves whenever ICANON is switched back on
    # (any raw -> cooked transition, including a normal exit); it is a
    # transient retype-pending marker, not a user-visible terminal setting.
    return [token for token in snapshot.split() if token.lstrip("-") != "pendin"]


def controlling_tty() -> None:
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


def run(binary: Path) -> dict:
    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    output = bytearray()
    with TemporaryDirectory(prefix="zenpi-panic-restore-") as raw:
        root = Path(raw)
        home, work = root / "home", root / "work"
        home.mkdir()
        work.mkdir()
        # The provider is never contacted; the panic fires right after terminal entry.
        (home / "config.toml").write_text(
            'backend="openai"\nmodel="gpt-4.1"\nbase_url="http://127.0.0.1:9/v1"\n'
            'requires_openai_auth=false\nmax_retries=0\n'
        )
        auth = home / "auth.json"
        auth.write_text(json.dumps({"OPENAI_API_KEY": "panic-restore-fixture"}))
        auth.chmod(0o600)
        env = {k: v for k, v in os.environ.items() if not k.startswith(("ZENPI_", "OPENAI_"))}
        env.update(
            ZENPI_HOME=str(home), TERM="xterm-256color", ZENPI_DEBUG_PANIC_AFTER_START="1", ZE255_DIR=str(root)
        )
        command = [str(binary), "--mode", "tui", "--backend", "openai", "--session", str(root / "s.jsonl")]
        process = subprocess.Popen(
            ["/bin/sh", "-c", HARNESS, "sh", *command],
            cwd=work,
            env=env,
            stdin=slave,
            stdout=slave,
            stderr=slave,
            preexec_fn=controlling_tty,
        )
        deadline = time.monotonic() + 30
        try:
            while time.monotonic() < deadline:
                ready, _, _ = select.select([master], [], [], 0.05)
                chunk = b""
                if ready:
                    try:
                        chunk = os.read(master, 65536)
                    except OSError:
                        break
                    output += chunk
                # macOS reports a hung-up master as readable with empty reads.
                if not chunk and process.poll() is not None:
                    break
            else:
                raise AssertionError(f"TUI did not exit after the debug panic: {bytes(output[-3000:])!r}")
        finally:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)
        before_stty = (root / "before").read_text()
        after_stty = (root / "after").read_text()
    os.close(master)
    os.close(slave)

    text = output.decode(errors="replace")
    checks = {
        "entered_alternate_screen": ENTER_ALT in output,
        "panicked": "ZENPI_DEBUG_PANIC_AFTER_START" in text,
        "nonzero_exit": process.returncode != 0,
        "before_was_cooked": {"icanon", "echo"} <= lflags(before_stty),
        "left_alternate_screen_after_enter": LEAVE_ALT in output[output.find(ENTER_ALT):],
        "cursor_shown": b"\x1b[?25h" in output,
        "icanon_restored": "icanon" in lflags(after_stty),
        "echo_restored": "echo" in lflags(after_stty),
        "stty_matches": comparable(after_stty) == comparable(before_stty),
    }
    result = {"binary": str(binary), "exit_code": process.returncode, "checks": checks}
    failed = [name for name, ok in checks.items() if not ok]
    if failed:
        result["output_tail"] = text[-2000:]
        result["stty_before"], result["stty_after"] = before_stty, after_stty
        raise AssertionError(json.dumps({"failed": failed, **result}, indent=2))
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/zenpi"))
    args = parser.parse_args()
    result = run(args.binary.resolve())
    print(json.dumps(result, indent=2))
    print("tui_panic_restore_smoke: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
