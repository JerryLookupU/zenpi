# Transform Note: `pi-mono/packages/tui/src/tui.ts`

> **Stale (2026-10-15, ZE-406):** the coalesced-redraw description below contradicts the v2 audit. zenpi caches rendered lines per message, invalidates only the streaming one and does no idle redraw (ZE-309). Treat the rest of this note as history.

source_path: `pi-mono/packages/tui/src/tui.ts`
source_hash: `c66359caeee468529dae65905401d67b7ba3dfaaae19a145e94bae7601bb1d31`

The source coalesces render requests, tracks dimensions, performs differential
updates, and forces a safe redraw when wrapping changes. zenpi uses one
Ratatui double buffer with `RenderScheduler`, bounded transcript lines, and
Unicode display-width wrapping. A resize invalidates layout and the next
frame uses the current area; unchanged bursts do not trigger extra frames.
`TuiState` and `TestBackend` tests cover narrow and one-cell areas.
