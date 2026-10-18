# Scanner Test Surface Count

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The scanner smoke harness's selected Compose surface now matches the actual
instrumented source: `ScannerResultTest` has 9 tests, `PairingScreenTest` has 7,
and `DiffScreenTest` has 5, for **21 tests**. The prior validator expected 20,
which would reject a genuinely complete run. Desktop smoke exact-count matching
was tightened at the same time to require a complete `OK (N tests)` line.

Validation:

```text
py_compile: OK
Ran 20 scanner/desktop validator tests ... OK
git diff --check: OK
```

Connected Android and physical-LAN execution remain unclaimed in this
environment.
