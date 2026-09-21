#!/usr/bin/env python3
"""Collect zenpi errors into one durable store for later triage.

zenpi emits turn/backend errors only on the live event stream (headless
stdout JSONL or the TUI message area); session journals record operations
and turns but no durable turn-failure records. This tool normalizes errors
from both sources into a single append-only store that other agents can
mine:

    store: ~/.zenpi/errors/errors.jsonl   (override with ZENPI_ERROR_HOME)

Commands:
    ingest [FILE ...]   read headless/event-stream JSONL (default: stdin)
    scan [FILE ...]     read session journal files (default: ZENPI_HOME journals)
    summary             group the store by normalized message
    tail [-n N]         print the newest records
    path                print the store location

Typical use:
    zenpi --mode headless --session s.jsonl < in.jsonl | tee out.jsonl
    python3 tools/zenpi_errorlog.py ingest out.jsonl
    python3 tools/zenpi_errorlog.py scan
    python3 tools/zenpi_errorlog.py summary
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys
from datetime import datetime, timezone

FAILED_OUTCOMES = {"success"}
IDISH = re.compile(
    r"(session|turn|call|req|request|operation|resp|item|msg|fc)_[-0-9A-Za-z]{8,}"
    r"|-\d{13,}-\d+|\b\d{13,}\b"
)


def zenpi_home():
    return Path(os.environ.get("ZENPI_HOME", Path.home() / ".zenpi"))


def store_path():
    root = Path(os.environ.get("ZENPI_ERROR_HOME", zenpi_home() / "errors"))
    root.mkdir(parents=True, exist_ok=True)
    return root / "errors.jsonl"


def now_iso():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def normalize(message):
    return IDISH.sub("<id>", message or "")


def fingerprint(record):
    key = "|".join(
        str(record.get(field) or "")
        for field in ("source", "session_id", "seq", "sequence", "kind", "message")
    )
    return hashlib.sha1(key.encode()).hexdigest()


def record(source, kind, message, code=None, raw=None, **context):
    entry = {
        "collected_at": now_iso(),
        "source": source,
        "kind": kind,
        "message": message,
        "code": code,
    }
    entry.update({k: v for k, v in context.items() if v is not None})
    if raw is not None:
        entry["raw"] = raw
    entry["fingerprint"] = fingerprint(entry)
    return entry


def stream_records(value):
    """Extract error records from one headless/event-stream JSONL line."""
    out = []
    project = value.get("project") or {}
    session_id = project.get("session_id") or value.get("session_id")
    cwd = project.get("cwd")
    event = value.get("event") if isinstance(value.get("event"), dict) else {}
    etype = event.get("type", "")
    if etype == "error":
        block = event.get("block") or {}
        out.append(
            record(
                "stream",
                block.get("code") or "error",
                event.get("message") or block.get("message"),
                code=block.get("code"),
                session_id=session_id,
                cwd=cwd,
                turn_id=value.get("turn_id"),
                request_id=value.get("request_id"),
                sequence=value.get("sequence"),
                retryable=block.get("retryable"),
            )
        )
    status = (event.get("status") or "").lower()
    if status == "failed" or etype in ("tool_failed", "turn_failed"):
        out.append(
            record(
                "stream",
                etype or "failed",
                event.get("message") or json.dumps(event, ensure_ascii=False)[:500],
                session_id=session_id,
                cwd=cwd,
                turn_id=value.get("turn_id"),
                request_id=value.get("request_id"),
                sequence=value.get("sequence"),
            )
        )
    if value.get("type") == "response" and value.get("success") is False:
        out.append(
            record(
                "stream",
                value.get("code") or "request_failed",
                value.get("error"),
                code=value.get("code"),
                session_id=session_id,
                cwd=cwd,
                request_id=value.get("id"),
            )
        )
    return out


def journal_records(value, path):
    """Extract error records from one session-journal JSONL line."""
    out = []
    event = value.get("event") if isinstance(value.get("event"), dict) else value
    if not isinstance(event, dict):
        return out
    etype = event.get("type", "")
    base = dict(
        session_id=value.get("session_id"),
        seq=value.get("seq"),
        file=str(path),
        timestamp=value.get("timestamp"),
    )
    if "error" in etype or "failed" in etype:
        out.append(
            record("journal", etype, json.dumps(event, ensure_ascii=False)[:500], **base)
        )
    if etype == "operation_finished":
        outcome = event.get("outcome")
        if outcome and outcome not in FAILED_OUTCOMES:
            out.append(
                record(
                    "journal",
                    f"operation_{outcome}",
                    f"operation {event.get('operation_id')} finished with outcome {outcome}",
                    operation_id=event.get("operation_id"),
                    **base,
                )
            )
    capture = event.get("output_capture") if isinstance(event.get("output_capture"), dict) else None
    if capture and capture.get("errors"):
        out.append(
            record(
                "journal",
                "tool_output_capture_errors",
                json.dumps(capture["errors"], ensure_ascii=False)[:500],
                turn_id=event.get("turn_id"),
                **base,
            )
        )
    return out


def iter_jsonl(path_or_stdin):
    if path_or_stdin is None:
        for line in sys.stdin:
            yield "<stdin>", line
        return
    path = Path(path_or_stdin)
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            yield str(path), line


def load_fingerprints(store):
    seen = set()
    if store.exists():
        with store.open("r", encoding="utf-8", errors="replace") as handle:
            for line in handle:
                try:
                    seen.add(json.loads(line).get("fingerprint"))
                except ValueError:
                    continue
    return seen


def append_records(records):
    store = store_path()
    seen = load_fingerprints(store)
    added = 0
    with store.open("a", encoding="utf-8") as handle:
        for entry in records:
            if entry["fingerprint"] in seen:
                continue
            seen.add(entry["fingerprint"])
            handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
            added += 1
    return added, store


def cmd_ingest(args):
    records = []
    files = args.files or [None]
    for name in files:
        for _, line in iter_jsonl(name):
            try:
                value = json.loads(line)
            except ValueError:
                continue
            records.extend(stream_records(value))
    added, store = append_records(records)
    print(f"ingest: {len(records)} error records found, {added} new -> {store}")


def default_journals():
    home = zenpi_home()
    candidates = sorted(home.glob("sessions/**/*.jsonl")) + sorted(home.glob("*.jsonl"))
    return [p for p in candidates if p.is_file()]


def cmd_scan(args):
    files = [Path(f) for f in args.files] if args.files else default_journals()
    records = []
    for path in files:
        if not path.is_file():
            print(f"scan: skipping missing {path}", file=sys.stderr)
            continue
        for _, line in iter_jsonl(path):
            try:
                value = json.loads(line)
            except ValueError:
                continue
            records.extend(journal_records(value, path))
    added, store = append_records(records)
    print(f"scan: {len(files)} journals, {len(records)} error records found, {added} new -> {store}")


def read_store():
    store = store_path()
    if not store.exists():
        return []
    with store.open("r", encoding="utf-8", errors="replace") as handle:
        return [json.loads(line) for line in handle if line.strip()]


def cmd_summary(args):
    entries = read_store()
    groups = {}
    for entry in entries:
        key = normalize(entry.get("message"))
        group = groups.setdefault(key, {"count": 0, "first": None, "last": None, "kinds": set(), "example": entry})
        group["count"] += 1
        group["kinds"].add(entry.get("kind"))
        seen_at = entry.get("timestamp") or entry.get("collected_at")
        if group["first"] is None:
            group["first"] = seen_at
        group["last"] = seen_at
    if args.json:
        printable = {
            k: {**v, "kinds": sorted(v["kinds"]), "example": v["example"]["message"]}
            for k, v in groups.items()
        }
        print(json.dumps(printable, ensure_ascii=False, indent=2))
        return
    print(f"store: {store_path()}  records: {len(entries)}  groups: {len(groups)}\n")
    for key, group in sorted(groups.items(), key=lambda item: -item[1]["count"]):
        kinds = ",".join(sorted(group["kinds"]))
        print(f"[{group['count']:>4}x] {kinds}  first={group['first']} last={group['last']}")
        print(f"       {key[:180]}")


def cmd_tail(args):
    entries = read_store()[-args.n :]
    for entry in entries:
        if args.json:
            print(json.dumps(entry, ensure_ascii=False))
        else:
            context = " ".join(
                f"{k}={entry[k]}" for k in ("session_id", "turn_id", "file") if entry.get(k)
            )
            print(f"{entry.get('collected_at')} [{entry.get('kind')}] {entry.get('message')}")
            if context:
                print(f"    {context}")


def cmd_path(_args):
    print(store_path())


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    for name, handler in (("ingest", cmd_ingest), ("scan", cmd_scan)):
        sub = commands.add_parser(name)
        sub.add_argument("files", nargs="*")
        sub.set_defaults(handler=handler)
    summary = commands.add_parser("summary")
    summary.add_argument("--json", action="store_true")
    summary.set_defaults(handler=cmd_summary)
    tail = commands.add_parser("tail")
    tail.add_argument("-n", type=int, default=20)
    tail.add_argument("--json", action="store_true")
    tail.set_defaults(handler=cmd_tail)
    path = commands.add_parser("path")
    path.set_defaults(handler=cmd_path)
    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
