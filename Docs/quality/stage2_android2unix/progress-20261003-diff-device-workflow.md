# Production Diff Paging Workflow Preparation

Provisional S2-036/S2-051/S2-054/S2-055 progress on 2026-10-03. The preceding
goal implementation added bounded remote diff paging; the intervening RTK
request verified a local development tool, not Android product completion.
This turn revalidated Rust sources and prepares connected acceptance through
the existing production desktop/Android worktree workflow. No nested agents,
worker launch, checklist promotion, commit/push or unrelated cleanup.

## Connected Gate

WorkspaceModelTest now wires the production next/previous ViewModel callbacks.
After its existing short-worktree diff it requests a tracked UTF-8 fixture
larger than 64 KiB, follows more than eight pages, returns to the first page
and revisits the second with exactly the same content. The assembled patch
must match the independent host Git patch SHA-256 and byte count. The tail
must disable next. A controlled desktop edit then makes the retained previous
page's continuation fail without replacing verified text; refresh must obtain
a new digest. Actual authenticated host responses, rather than fixed sleeps,
synchronize fixture preparation, edit visibility and restoration.

The desktop harness owns and restores only diff-pages.txt in its temporary
fixture. It verifies active worktree identity, unchanged owner revision/tab
checkpoint/session journal during paging and exact fixture content at all
three phases. Existing dirty keep.txt refusal and safe worktree removal remain.
No provider request is added. The explicit suite remains 69 tests, or 70 with
Doze, because the workflow extends an existing test method.

The API matrix now requires android_worktree_diff_pages_verified plus ordered
requested/verified/changed phases and exactly one matching content digest and
byte-count status. Missing, reordered, duplicate, mismatched, short-patch or
boolean-only receipts fail closed. Positive and negative Python fixtures prove
the validator contract, not an actual Android execution.

## Verification

- Final real-Git remote_diff regression: one passed. Shared slash_actions
  regression rerun after all Rust changes: 31 passed.
- cargo fmt --all -- --check and strict Blueprint passed; all 36 rows remain
  unfinished. git diff --check passed.
- Python Stage2 discovery: 95 passed; desktop harness tests: 16 passed;
  lifecycle verifier: 18 passed; instrumentation verifier: five passed;
  Blueprint tests: five passed.
- Debug host rebuilt after test relink, then debug/release verify-only both
  passed. Release session 94115 observed terminal exit 0, never restarted on
  observation timeout.
- The first instrumentation source compile rejected a cross-module nullable
  smart cast. A stable non-null page metadata local corrected it. Final
  zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-diff-device-workflow-final2-20261003 exited 0 after the
  synchronizing-read change: main/unit/instrumentation Kotlin compiled and
  362 JVM tests passed. Instrumentation compiled only, not device-executed.
- The real desktop device entry point refused missing current APK provenance
  before adb/install/listener work. Old APKs were not installed. No unchanged
  Gradle/socket retry or sandbox bypass occurred.

```text
main.jar a604419a24f99e6ba48357455066ed6bdcbd1517e442e9b52c3c3af2e3e72e7c
unit.jar 2a76300ad5f41abdebb3c2b04c8b9f95546379eda056ae4b1eead438f62de2dd
instrumented.jar 303328d79dce40a8aa167aa48ca523622c46b6b6322dfe775271bb9bac2cfe5c
WorkspaceModelTest.kt 6a08dba0aaa14c9d3ecbc828839ca86ba2ccf4b84faa4b019f2f8d42e85a0945
stage2_desktop_android_smoke.py ae30658f0193946d30fd5d0f4b6227036708f9fa6b12d39573fc43707563185f
validate_stage2_android_lifecycle.py 5ccd8d1e490fc851b7715ca515c4f71ee4f5a541ee60f5ef955db6d59a06c1b6
host inputs 88d16e5b7b40331629dab164b6475d5af57c79b4403fd3d598f62a0d0af01906
debug receipt 286347f22d43b7085f70b6986c8ac0d25d6a278d7ef8b146a5cb056403eb8136
debug binary 1aff8e1c98ee5490ab4e6d91fd578c1b94c317b99eb2302e59da2ec22ed8412d
release receipt e5fe7e1c5327f89a6af76c33e44dd26092e55102c31f96c79f5fed938c1b5c04
release binary a02830728091019a43f105831c6efb32aa72de8fc8976591513aefdbce15f4df
```

Goal remains active. Fresh current-source APKs, connected full parity/TLS sync,
physical camera QR/direct LAN, required API lifecycle matrix and signed
install/upgrade are still missing behavioral acceptance, not completed gates.
