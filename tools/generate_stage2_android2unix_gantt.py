#!/usr/bin/env python3
"""Generate the read-only same-prefix Stage 2 Gantt projection."""
from __future__ import annotations

import datetime as dt
import hashlib
import os
import re
import tempfile
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BP = ROOT / "Docs/stage2_android2unix_blueprint.md"
SPEC = ROOT / "Docs/stage2_android2unix_spec.md"
OUT = ROOT / "Docs/stage2_android2unix_gantt.md"
ROW = re.compile(r"^- \[([ _x])\] \*\*(S2-[0-9]{3})\*\* — (.*?)；layer `([^`]+)` \| (.*)$")


def atomic(path: Path, content: str) -> None:
    fd, temp = tempfile.mkstemp(prefix=".stage2-gantt-", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp, path)
    finally:
        if os.path.exists(temp):
            os.unlink(temp)


def items() -> list[dict[str, object]]:
    result = []
    for line in BP.read_text(encoding="utf-8").splitlines():
        match = ROW.match(line)
        if not match:
            continue
        mark, item_id, title, layer, fields = match.groups()
        values = {}
        for cell in fields.split(" | "):
            key, _, value = cell.partition(": ")
            values[key] = value
        raw = values["Depends"]
        depends = [] if raw in {"—", "-", "none"} else [part.strip() for part in raw.split(",") if part.strip()]
        result.append(dict(mark=mark, id=item_id, title=title, layer=layer, depends=depends))
    if not result:
        raise SystemExit("no Stage 2 checklist rows")
    return result


def depth(item_map: dict[str, dict[str, object]], item_id: str, visiting: set[str] | None = None) -> int:
    visiting = visiting or set()
    if item_id in visiting:
        raise SystemExit(f"dependency cycle at {item_id}")
    deps = item_map[item_id]["depends"]
    if not deps:
        return 1
    return 1 + max(depth(item_map, dep, visiting | {item_id}) for dep in deps)


def main() -> None:
    rows = items()
    item_map = {str(row["id"]): row for row in rows}
    depths = {item_id: depth(item_map, item_id) for item_id in item_map}
    now = dt.datetime.now(dt.timezone.utc).astimezone().isoformat(timespec="seconds")
    bp_sha = hashlib.sha256(BP.read_bytes()).hexdigest()
    spec_sha = hashlib.sha256(SPEC.read_bytes()).hexdigest()
    counts = Counter("[x]" if r["mark"] == "x" else "[_]" if r["mark"] == "_" else "[ ]" for r in rows)
    lines = [
        "# Zenpi Stage 2 Android ↔ Unix — Gantt",
        "",
        "```yaml",
        "schema_version: execution-gantt/stage2",
        "source_path: Docs/stage2_android2unix_blueprint.md",
        "spec_path: Docs/stage2_android2unix_spec.md",
        "projection_authority: false",
        f"generated_at: '{now}'",
        f"source_sha256: {bp_sha}",
        f"spec_sha256: {spec_sha}",
        f"unclaimed: {counts['[ ]']}",
        f"self_tested: {counts['[_]']}",
        f"master_accepted: {counts['[x]']}",
        "unscheduled_policy: visible without invented calendar dates",
        "```",
        "",
        "> 只读投影；唯一权威来源是 `Docs/stage2_android2unix_blueprint.md`。横轴表示依赖深度，不表示日历工期。",
        "",
        "## Progress",
        "",
        "| State | Count |",
        "|---|---:|",
        f"| master accepted | {counts['[x]']} |",
        f"| worker self-tested | {counts['[_]']} |",
        f"| unfinished | {counts['[ ]']} |",
        "",
        "## Dependency Gantt",
        "",
        "```mermaid",
        "gantt",
        "    title Stage 2 dependency projection (not calendar duration)",
        "    dateFormat X",
        "    axisFormat %s",
    ]
    for layer in sorted({str(row["layer"]) for row in rows}):
        lines.append(f"    section {layer}")
        for row in rows:
            if row["layer"] != layer:
                continue
            mark = "done, " if row["mark"] == "x" else "active, " if row["mark"] == "_" else ""
            label = str(row["title"]).replace(":", "：")[:55]
            lines.append(f"    {row['id']} {label} :{mark}{row['id']}, {depths[row['id']]}, 1s")
    lines += [
        "```",
        "",
        "## Monitoring index",
        "",
        "| ID | State | Depends on | Claim/owner | Startup/live | Handoff/integration/repair | Scheduling note |",
        "|---|---|---|---|---|---|---|",
    ]
    for row in rows:
        state = "master_accepted" if row["mark"] == "x" else "self_tested" if row["mark"] == "_" else "unclaimed"
        deps = ",".join(row["depends"]) or "—"
        lines.append(f"| {row['id']} | {state} | {deps} | unclaimed | not_started | none | unscheduled; estimate is not a calendar date |")
    lines.append("")
    atomic(OUT, "\n".join(lines))
    print(f"generated {OUT.relative_to(ROOT)} ({len(rows)} items, source {bp_sha})")


if __name__ == "__main__":
    main()
