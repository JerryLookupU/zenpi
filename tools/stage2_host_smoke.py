#!/usr/bin/env python3
"""Run the production Rust host-pair CLI and validate its QR invite envelope.

This is a bounded host-side smoke only. It never claims Android camera or
physical-LAN success; use the Android harness for those device gates.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import ipaddress
import json
import os
import selectors
import subprocess
import tempfile
import time
from pathlib import Path
from urllib.parse import parse_qs, urlsplit
from stage2_host_build import require_current_host

ROOT = Path(__file__).resolve().parents[1]


def validate_invite(value: str, now: int | None = None) -> dict[str, str]:
    if len(value.encode("utf-8")) > 2048 or any(ord(char) < 32 or ord(char) == 127 for char in value):
        raise ValueError("invalid zenpi pair URI")
    parsed = urlsplit(value)
    if parsed.scheme != "zenpi" or parsed.netloc != "pair" or parsed.path or parsed.fragment:
        raise ValueError("invalid zenpi pair URI")
    fields = parse_qs(parsed.query, keep_blank_values=True, strict_parsing=True)
    if set(fields) != {"v", "address", "fingerprint", "token", "expires"} or any(len(v) != 1 for v in fields.values()):
        raise ValueError("invite fields are not exact")
    result = {key: values[0] for key, values in fields.items()}
    try:
        # urlsplit handles both the emitted IPv4 form and bracketed IPv6;
        # reject hostnames, scoped IPv6 literals, and unusable endpoints.
        endpoint = urlsplit("//" + result["address"])
        host = endpoint.hostname
        port = endpoint.port
        parsed_ip = ipaddress.ip_address(host or "")
        effective_ip = getattr(parsed_ip, "ipv4_mapped", None) or parsed_ip
        if (endpoint.path or endpoint.query or endpoint.fragment or endpoint.username or endpoint.password
                or "%" in (host or "") or port is None or not 1 <= port <= 65535
                or effective_ip.is_unspecified or effective_ip.is_multicast or effective_ip.is_link_local
                or effective_ip == ipaddress.IPv4Address("255.255.255.255")):
            raise ValueError
    except (ValueError, UnicodeError):
        raise ValueError("invite address invalid") from None
    if result["v"] != "1" or len(result["fingerprint"]) != 64 or any(c not in "0123456789abcdef" for c in result["fingerprint"]):
        raise ValueError("invite fingerprint/version invalid")
    token = result["token"]
    # Pair tokens are unpadded base64url (32 random bytes); receipt output
    # below deliberately never persists this one-use credential.
    try:
        decoded_token = base64.urlsafe_b64decode(token + "=")
    except (ValueError, base64.binascii.Error):
        decoded_token = b""
    if (len(token) != 43 or any(char not in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_" for char in token)
            or len(decoded_token) != 32 or base64.urlsafe_b64encode(decoded_token).decode().rstrip("=") != token
            or len(result["expires"]) > 19 or not result["expires"].isascii() or not result["expires"].isdigit()):
        raise ValueError("invite token/expiry invalid")
    expiry = int(result["expires"])
    current = int(time.time() if now is None else now)
    # The host emits a short-lived one-use invite.  A far-future value is not
    # a valid production envelope even though it is technically not expired.
    if expiry <= current or expiry > current + 300:
        raise ValueError("invite expired")
    return result


def run(binary: Path, bind: str, advertise: str | None, receipt: Path | None) -> dict:
    binary = binary.absolute()
    provenance = require_current_host(binary, ROOT)
    started = time.time()
    with tempfile.TemporaryDirectory(prefix="zenpi-host-smoke-") as raw:
        identity = Path(raw) / "identity.json"
        argv = [str(binary), "host", "pair", "--bind", bind, "--plain", "--duration-seconds", "1", "--identity", str(identity)]
        if advertise:
            argv += ["--advertise", advertise]
        # Pairing only needs a small process environment.  Do not inherit
        # unrelated provider credentials (for example ANTHROPIC_API_KEY) into
        # the production host smoke child or its evidence-producing fixture.
        env = {
            key: value
            for key, value in os.environ.items()
            if key in {"PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE"}
        }
        env["ZENPI_HOME"] = str(Path(raw) / "home")
        process = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        stdout_chunks: list[bytes] = []
        stderr_chunks: list[bytes] = []
        emission_time: float | None = None
        returncode = None
        timed_out = False
        selector = selectors.DefaultSelector()
        try:
            for stream in (process.stdout, process.stderr):
                if stream is not None:
                    selector.register(stream, selectors.EVENT_READ)
            deadline = time.monotonic() + 12
            while selector.get_map() and time.monotonic() < deadline:
                for key, _ in selector.select(max(0, deadline - time.monotonic())):
                    chunk = key.fileobj.read1(65536) if hasattr(key.fileobj, "read1") else key.fileobj.read(65536)
                    if chunk:
                        (stdout_chunks if key.fileobj is process.stdout else stderr_chunks).append(chunk)
                        if emission_time is None and key.fileobj is process.stdout:
                            joined = b"".join(stdout_chunks)
                            if b"\n" in joined:
                                emission_time = time.time()
                    else:
                        selector.unregister(key.fileobj)
                        key.fileobj.close()
                if process.poll() is not None and not selector.get_map():
                    break
            if selector.get_map():
                timed_out = True
                process.kill()
                for key in list(selector.get_map().values()):
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
            # EOF can precede process exit. Wait within the original budget
            # rather than killing a host that has just closed its output.
            try:
                returncode = process.wait(timeout=2 if timed_out else max(0, deadline - time.monotonic()))
            except subprocess.TimeoutExpired:
                timed_out = True
                process.kill()
                returncode = process.wait(timeout=2)
        finally:
            selector.close()
            if process.poll() is None:
                process.kill()
                process.wait()
        stdout = b"".join(stdout_chunks).decode("utf-8", errors="replace")
        stderr = b"".join(stderr_chunks).decode("utf-8", errors="replace")
        invite = None
        error = None
        try:
            invite = validate_invite(stdout.splitlines()[0], now=int(emission_time or started))
        except (IndexError, ValueError) as failure:
            error = "invite not emitted" if isinstance(failure, IndexError) else str(failure)
            if "Operation not permitted" in stderr:
                error = f"{error}; host_stderr=redacted sandbox diagnostic"
            elif stderr:
                error = f"{error}; host_stderr=redacted"
        redacted_invite = None
        token_sha256 = None
        if invite is not None:
            token_sha256 = hashlib.sha256(invite["token"].encode()).hexdigest()
            redacted_invite = {key: value for key, value in invite.items() if key != "token"}
            redacted_invite["token_sha256"] = token_sha256
        provenance_current = False
        try:
            provenance_current = require_current_host(binary, ROOT) == provenance
        except (ValueError, OSError):
            pass
        if not provenance_current:
            error = "Host build provenance changed during smoke"
        evidence = {
            # A valid invite is only a pass when the bounded host exits cleanly.
            # Killing it after the first line would hide shutdown regressions.
            "passed": invite is not None and error is None and returncode == 0 and not timed_out and provenance_current,
            "scope": "production Rust host pair invite envelope; no Android or physical-LAN claim",
            "argv": argv,
            "cwd": str(ROOT),
            "started_at_unix": started,
            "finished_at_unix": time.time(),
            "exit_code": returncode,
            "timed_out": timed_out,
            "stdout_sha256": hashlib.sha256(stdout.encode()).hexdigest(),
            "stderr_sha256": hashlib.sha256(stderr.encode()).hexdigest(),
            "invite": redacted_invite,
            "invite_token_sha256": token_sha256,
            "error": error,
            "blocked_by_sandbox": "Operation not permitted" in stderr,
            "binary": str(binary),
            "binary_sha256": provenance["artifact"]["sha256"],
            "host_build_provenance": provenance,
            "host_build_provenance_current": provenance_current,
        }
        if receipt:
            receipt.parent.mkdir(parents=True, exist_ok=True)
            receipt.write_text(json.dumps(evidence, indent=2) + "\n")
        return evidence


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/zenpi")
    parser.add_argument("--bind", default="127.0.0.1:0")
    parser.add_argument("--advertise")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    if not args.binary.is_file():
        raise SystemExit(f"missing binary: {args.binary}")
    try:
        evidence = run(args.binary, args.bind, args.advertise, args.receipt)
    except (ValueError, OSError) as error:
        raise SystemExit(str(error)) from error
    print(json.dumps(evidence, indent=2))
    if not evidence["passed"]:
        raise SystemExit("Rust host smoke failed or was blocked before invite emission")


if __name__ == "__main__":
    main()
