#!/usr/bin/env bash
# Learn cron: opencode agent runtime subset -> zenpi 1:1 中文学习笔记 + Rust 映射.
#
# Contract: Docs/learn/opencode_runtime/subsets/opencode_agent_runtime/source_manifest.tsv
#   - workers may only advance [ ] -> [_]
#   - this script's master lane is the only actor that writes [x]
#   - concurrency: LEARN_WORKERS (default 12, hard cap 12)
#
# Usage: tools/learn_opencode_runtime.sh [--workers N] [--files-only] [--folders-only]
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LEARN_ROOT="$REPO_ROOT/Docs/learn/opencode_runtime"
SUBSET_DIR="$LEARN_ROOT/subsets/opencode_agent_runtime"
MANIFEST="$SUBSET_DIR/source_manifest.tsv"
SOURCE_ROOT="${OPENCODE_SOURCE_ROOT:-$HOME/GitHub/opencode}"
MAX_WORKERS="${LEARN_WORKERS:-12}"
case "$MAX_WORKERS" in ''|*[!0-9]*) MAX_WORKERS=12 ;; esac
[ "$MAX_WORKERS" -gt 12 ] && MAX_WORKERS=12
[ "$MAX_WORKERS" -lt 1 ] && MAX_WORKERS=1
AGENT="${LEARN_AGENT:-codex}"
MODEL="${LEARN_MODEL:-sonnet}"
EFFORT="${LEARN_EFFORT:-high}"
LOG_DIR="$LEARN_ROOT/.cron"
RECEIPTS_DIR="$LEARN_ROOT/receipts"
LOCK_DIR="$LOG_DIR/.manifest.lock"
PY=python3

FILES_ONLY=0
FOLDERS_ONLY=0
LIMIT=0
while [ $# -gt 0 ]; do
  case "$1" in
    --workers) MAX_WORKERS="$2"; shift 2 ;;
    --files-only) FILES_ONLY=1; shift ;;
    --folders-only) FOLDERS_ONLY=1; shift ;;
    --limit) LIMIT="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

mkdir -p "$LOG_DIR" "$RECEIPTS_DIR"

lock() { local n=0; while ! mkdir "$LOCK_DIR" 2>/dev/null; do n=$((n+1)); [ "$n" -gt 600 ] && { echo "manifest lock timeout" >&2; exit 1; }; sleep 0.2; done; }
unlock() { rmdir "$LOCK_DIR" 2>/dev/null || true; }

manifest_set() { # item status
  lock
  "$PY" - "$MANIFEST" "$1" "$2" <<'PYEOF'
import sys
path, item, status = sys.argv[1], sys.argv[2], sys.argv[3]
with open(path) as f:
    header = f.readline()
    rows = [line.rstrip('\n').split('\t') for line in f]
for cells in rows:
    if len(cells) >= 13 and cells[11] == item:
        cells[12] = status
with open(path, 'w') as f:
    f.write(header)
    for cells in rows:
        f.write('\t'.join(cells) + '\n')
PYEOF
  unlock
}

manifest_query() { # mode: pending|all  -> item,source_path,target_artifact,source_hash,source_bytes,folder_artifact
  "$PY" - "$MANIFEST" "$1" <<'PYEOF'
import sys
mode = sys.argv[2]
with open(sys.argv[1]) as f:
    f.readline()
    for line in f:
        c = line.rstrip('\n').split('\t')
        if len(c) < 13:
            continue
        if mode == 'pending' and c[12] != '[ ]':
            continue
        print('\t'.join([c[11], c[1], c[8], c[4], c[3], c[9]]))
PYEOF
}

write_prompt() { # item source_path target hash bytes -> prompt path
  local item="$1" source_path="$2" target="$3" hash="$4" bytes="$5"
  local prompt="$LOG_DIR/$item.prompt.md"
  cat > "$prompt" <<PROMPT
你是 zenpi 项目的学习工人（learn_mode=understand，1:1 源文件学习）。只做这一件事，完成后不要做别的。

源文件（绝对路径）：$SOURCE_ROOT/$source_path
目标笔记（必须创建目录并写入）：$REPO_ROOT/$target
item_id: $item
source_path: $source_path
source_hash: $hash
source_bytes: $bytes

硬性要求：
1. 完整按顺序读取源文件（含注释、类型、导出符号）；引用行号用 \`L12-L40\` 形式。不得只读片段。
2. 笔记正文用中文；代码标识符、路径、命令、类型名保留原文。必须包含以下小节（缺一不可）：
   - \`# $item — $source_path\`
   - 元信息块：source_id/item_id、source_path、source_hash、source_bytes、source_lines、coverage（读到的字节范围与行范围）
   - \`## 完整行为复盘\`：逐函数/逐导出符号，带行号；输入输出、边界、默认值、错误路径、并发语义
   - \`## 状态、取消、恢复与副作用\`：取消/超时/重试/持久化/外部副作用
   - \`## 源内测试与行为判据\`：源文件或同目录测试引用的行为判据（没有就写“源内未包含测试”，并给出可独立验证的判据）
   - \`## zenpi Rust 映射\`：与 zenpi 现有 \`src/headless.rs\`、\`src/core.rs\`、\`src/session.rs\`、\`src/tool_runtime.rs\`、\`src/runtime.rs\`、\`src/protocol.rs\`、\`src/approval.rs\`、\`src/providers/**\` 对照；给出建议的 Rust 模块/类型/函数落点与差异清单（可执行、可验证）
   - \`## 未决问题\`：无法从源确认的点（没有就写“无”）
3. 信息密集、可核对；不要泛泛而谈，不要整段抄源码，不要写占位符或 TODO。
4. 只允许写目标笔记文件；不得修改 opencode 源码，也不得修改 zenpi 源码。
5. 目标笔记至少 1200 字节。
PROMPT
  echo "$prompt"
}

run_agent() { # prompt out
  case "$AGENT" in
    claude)
      claude -p --model "$MODEL" --effort "$EFFORT" --permission-mode bypassPermissions \
        --add-dir "$SOURCE_ROOT" --add-dir "$REPO_ROOT" < "$1" > "$2" 2>&1
      ;;
    codex)
      codex exec --cd "$REPO_ROOT" --skip-git-repo-check \
        --sandbox workspace-write --dangerously-bypass-approvals-and-sandbox \
        ${CODEX_MODEL:+--model "$CODEX_MODEL"} \
        -c model_reasoning_effort="${CODEX_REASONING_EFFORT:-high}" < "$1" > "$2" 2>&1
      ;;
    opencode)
      opencode run --dir "$REPO_ROOT" < "$1" > "$2" 2>&1
      ;;
    *)
      echo "unknown LEARN_AGENT: $AGENT" >&2
      return 2
      ;;
  esac
}

validate_artifact() { # target source_path
  "$PY" - "$REPO_ROOT/$1" "$2" <<'PYEOF'
import os, sys
target, source_path = sys.argv[1], sys.argv[2]
if not os.path.isfile(target):
    sys.exit(1)
data = open(target, encoding='utf-8', errors='replace').read()
if len(data.encode('utf-8')) < 1200:
    sys.exit(2)
for needle in ('## 完整行为复盘', '## zenpi Rust 映射', source_path):
    if needle not in data:
        sys.exit(3)
if 'TODO' in data or '占位' in data:
    sys.exit(4)
PYEOF
}

write_receipt() { # item source_path source_hash target attempt
  "$PY" - "$REPO_ROOT" "$RECEIPTS_DIR" "$1" "$2" "$3" "$4" "$5" <<'PYEOF'
import hashlib, json, os, sys, time
repo, receipts, item, source_path, source_hash, target, attempt = sys.argv[1:8]
artifact = os.path.join(repo, target)
data = open(artifact, 'rb').read()
now = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
record = {
    'item_id': item,
    'source_path': source_path,
    'source_hash': source_hash,
    'target_artifact': target,
    'artifact_bytes': len(data),
    'artifact_sha256': hashlib.sha256(data).hexdigest(),
    'attempts': int(attempt),
    'validated_by': 'master',
    'validated_at': now,
}
with open(os.path.join(receipts, item + '.master.json'), 'w') as f:
    json.dump(record, f, indent=2, ensure_ascii=False)
PYEOF
}

worker_cycle() { # item source_path target hash bytes
  local item="$1" source_path="$2" target="$3" hash="$4" bytes="$5"
  local prompt out attempt rc
  prompt="$(write_prompt "$item" "$source_path" "$target" "$hash" "$bytes")"
  out="$LOG_DIR/$item.worker.log"
  attempt=0
  while [ "$attempt" -lt 2 ]; do
    attempt=$((attempt + 1))
    run_agent "$prompt" "$out"
    rc=$?
    if [ "$rc" -eq 0 ] && validate_artifact "$target" "$source_path"; then
      write_receipt "$item" "$source_path" "$hash" "$target" "$attempt"
      manifest_set "$item" "[x]"
      echo "[$item] accepted ($attempt attempt(s)) $source_path"
      return 0
    fi
    echo "[$item] attempt $attempt failed (rc=$rc) $source_path" >> "$LOG_DIR/cron.log"
  done
  manifest_set "$item" "[ ]"
  echo "[$item] FAILED after 2 attempts: $source_path"
  return 1
}

folder_worker() { # folder_path folder_artifact
  local folder="$1" artifact="$2" item prompt out rc
  item="$(echo "$folder" | tr '/.' '__')"
  prompt="$LOG_DIR/folder_$item.prompt.md"
  out="$LOG_DIR/folder_$item.worker.log"
  {
    echo "你是 zenpi 项目的学习工人（learn_mode=understand，目录汇总）。"
    echo "目录（opencode 源码）：$SOURCE_ROOT/$folder"
    echo "目标目录汇总（必须创建目录并写入）：$REPO_ROOT/$artifact"
    echo
    echo "该目录下每个源文件的 1:1 中文笔记已经完成，位于："
    manifest_query all | while IFS=$'\t' read -r i sp ta hash bytes fa; do
      case "$sp" in
        "$folder"/*) echo "- $sp -> $ta" ;;
      esac
    done
    cat <<'PROMPT'

硬性要求：
1. 读取上述每个笔记（必要时回看源码确认），写一份目录级汇总（中文，代码标识符保留原文）。
2. 汇总必须包含：目录职责、模块清单（每个文件一句话 + 关键导出）、运行时数据流/控制流、错误与取消语义、与 zenpi Rust 的映射建议、未决问题。
3. 至少 800 字节；不要占位符或 TODO；只允许写目标汇总文件。
PROMPT
  } > "$prompt"
  run_agent "$prompt" "$out"
  rc=$?
  "$PY" - "$REPO_ROOT/$artifact" "$folder" "$rc" <<'PYEOF'
import os, sys
artifact, folder, rc = sys.argv[1], sys.argv[2], int(sys.argv[3])
ok = rc == 0 and os.path.isfile(artifact) and os.path.getsize(artifact) >= 800
if ok:
    data = open(artifact, encoding='utf-8', errors='replace').read()
    ok = folder in data
sys.exit(0 if ok else 1)
PYEOF
}

write_indices() {
  "$PY" - "$MANIFEST" "$LEARN_ROOT" <<'PYEOF'
import csv, os, sys
manifest, learn = sys.argv[1], sys.argv[2]
rows = list(csv.DictReader(open(manifest), delimiter='\t'))
with open(os.path.join(learn, 'file_learn_index.tsv'), 'w') as f:
    f.write('item_id\tsource_id\tsource_path\tsource_hash\ttarget_artifact\tstatus\n')
    for r in rows:
        f.write('\t'.join([r['item_id'], r['source_id'], r['source_path'], r['source_hash'], r['target_artifact'], r['status']]) + '\n')
folders = {}
for r in rows:
    folders.setdefault(r['folder_artifact'], []).append(r)
with open(os.path.join(learn, 'folder_learn_index.tsv'), 'w') as f:
    f.write('folder_path\tfolder_artifact\tfiles\taccepted\n')
    for fa, items in sorted(folders.items()):
        folder_path = fa[:-len('/current_folder_learn.md')]
        accepted = sum(1 for i in items if i['status'] == '[x]')
        f.write('\t'.join([folder_path, fa, str(len(items)), str(accepted)]) + '\n')
print('indices written:', len(rows), 'files,', len(folders), 'folders')
PYEOF
}

dispatch_pending() {
  local running=0 item source_path target hash bytes dispatched=0
  while :; do
    if [ "$LIMIT" -gt 0 ] && [ "$dispatched" -ge "$LIMIT" ] && [ "$(jobs -pr | wc -l | tr -d ' ')" -eq 0 ]; then
      break
    fi
    running=$(jobs -pr | wc -l | tr -d ' ')
    if [ "$running" -lt "$MAX_WORKERS" ]; then
      item=""
      while IFS=$'\t' read -r i sp ta h b fa; do
        if ! grep -q "^$i\$" "$LOG_DIR/.inflight" 2>/dev/null; then
          item="$i"; source_path="$sp"; target="$ta"; hash="$h"; bytes="$b"
          echo "$i" >> "$LOG_DIR/.inflight"
          break
        fi
      done < <(manifest_query pending)
      if [ -z "$item" ]; then
        [ "$running" -eq 0 ] && break
        sleep 2
        continue
      fi
      dispatched=$((dispatched + 1))
      worker_cycle "$item" "$source_path" "$target" "$hash" "$bytes" &
      sleep 1
    else
      sleep 2
    fi
  done
  wait
  rm -f "$LOG_DIR/.inflight"
}

main() {
  if [ ! -f "$MANIFEST" ]; then echo "missing manifest: $MANIFEST" >&2; exit 1; fi
  if [ ! -d "$SOURCE_ROOT/packages/opencode/src" ]; then echo "missing source root: $SOURCE_ROOT" >&2; exit 1; fi
  rm -f "$LOG_DIR/.inflight"
  if [ "$FOLDERS_ONLY" -eq 0 ]; then
    echo "=== files wave (workers=$MAX_WORKERS, agent=$AGENT/$MODEL/$EFFORT) ==="
    dispatch_pending
  fi
  write_indices
  local pending
  pending="$(manifest_query pending | wc -l | tr -d ' ')"
  echo "=== pending after files wave: $pending ==="
  if [ "$FILES_ONLY" -eq 0 ] && [ "$pending" -eq 0 ]; then
    echo "=== folder synthesis wave ==="
    local fa folder
    while IFS=$'\t' read -r folder fa; do
      folder_worker "$folder" "$fa" &
      while [ "$(jobs -pr | wc -l | tr -d ' ')" -ge "$MAX_WORKERS" ]; do sleep 1; done
    done < <("$PY" - "$MANIFEST" <<'PYEOF'
import csv, sys
seen = []
for r in csv.DictReader(open(sys.argv[1]), delimiter='\t'):
    fa = r['folder_artifact']
    if fa not in seen:
        seen.append(fa)
        print('\t'.join([fa[:-len('/current_folder_learn.md')], fa]))
PYEOF
)
    wait
    write_indices
  fi
  "$PY" - "$MANIFEST" "$REPO_ROOT" <<'PYEOF'
import csv, os, sys
manifest, repo = sys.argv[1], sys.argv[2]
rows = list(csv.DictReader(open(manifest), delimiter='\t'))
open_rows = [r for r in rows if r['status'] != '[x]']
missing = [r for r in rows if not os.path.isfile(os.path.join(repo, r['target_artifact']))]
folders = sorted({r['folder_artifact'] for r in rows})
missing_folders = [f for f in folders if not os.path.isfile(os.path.join(repo, f))]
print('coverage: rows=%d accepted=%d open=%d missing_artifacts=%d missing_folder_artifacts=%d'
      % (len(rows), len(rows) - len(open_rows), len(open_rows), len(missing), len(missing_folders)))
sys.exit(0 if not open_rows and not missing and not missing_folders else 1)
PYEOF
}

main "$@"
