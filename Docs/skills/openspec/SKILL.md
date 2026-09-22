---
name: openspec
description: Use when the workspace contains an openspec/ directory or the user mentions spec-driven development, change proposals, capability specs, or OpenSpec. Follows the OpenSpec three-stage workflow with the openspec CLI so changes are proposed, validated and approved before implementation.
---

# OpenSpec workflow

OpenSpec is a spec-driven development workflow shared with opencode/codex. When
`openspec/` exists in the workspace, read `openspec/AGENTS.md` first and treat it
as authoritative for this repo.

## Quick checklist

```bash
openspec spec list --long      # existing capabilities
openspec list                  # active changes
openspec validate <change-id> --strict
openspec show <change-id>
```

## Three stages

1. **Propose** (do not implement yet):
   - Pick a unique kebab-case, verb-led change id (`add-`, `update-`, `remove-`, `refactor-`).
   - Scaffold `openspec/changes/<id>/`: `proposal.md`, `tasks.md`, `design.md` (only if needed),
     plus delta specs per affected capability.
   - Delta specs use `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` and every requirement
     needs at least one `#### Scenario:`.
   - Validate with `openspec validate <id> --strict`; fix all issues.
   - Stop and request approval before writing implementation code.
2. **Implement** after approval: follow `tasks.md` in order, keep the spec deltas in sync.
3. **Archive** when done: `openspec archive <id>` (or the repo's documented archive step).

## Rules

- Never start implementation before the proposal is approved.
- Never edit existing spec files directly to "make room"; express changes as deltas.
- Bug fixes, typos, formatting, non-breaking dependency and config changes skip the proposal.
- If `openspec/AGENTS.md` conflicts with this skill, follow the repository file.
