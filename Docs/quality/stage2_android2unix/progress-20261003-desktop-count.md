# Desktop Android Smoke Test Surface Count

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The desktop Android harness previously expected 17 tests for its base class
set and 35 for the workspace-control set. Counting the actual instrumented
source gives 20 base tests and 43 workspace-control tests; the optional doze
test adds one. The expected-count guard now follows those explicit class lists,
and still requires a complete `OK (N tests)` line.

The scanner harness separately expects 21 tests, matching its three selected
classes. Python smoke validators continue to pass. Connected emulator, camera
and physical-LAN execution remain unclaimed in this environment.
