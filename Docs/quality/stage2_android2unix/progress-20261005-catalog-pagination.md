# Catalog Pagination And Current QR Acceptance

The previous goal turn was progress: a production empty-subtab crash was
repaired, five focused API 35 tests passed and the API 34 camera/network/Doze
workflow passed on its attributed APKs. This turn continues the original
Blueprint scope; no checklist checkbox was changed and the goal stays active.

## Production Repair

The current API 35 catalog baseline reproduced two failures: file paging
expected callbacks `[0, 128]` but received none, and resource paging expected
`[64, 0]` but received none. Its retained output is
`android-api35-catalog-baseline-20261005.instrumentation.txt` (five tests,
two failures). A screenshot confirmed pagination was below the visible
scroll area of a full catalog. The user had to traverse the entire page to
reach navigation, and synthetic clicks on those offscreen controls did not
exercise their callbacks.

File and resource dialogs now keep pagination in the fixed action area,
outside their scrolling entries. Existing navigation, read-only, loading,
offline, pending-write and malformed-scope policies remain intact. Changing
the resource snapshot/generation/page or file path/snapshot/page resets list
scroll to its start. Opening either catalog hides the composer keyboard and
releases its focus, while preserving the draft. This also prevents a catalog
action row from being clipped during the keyboard/dialog transition.

The connected tests now scroll to the last entry, require both paging
controls to remain displayed/enabled, click them and check their exact page
offsets. They install a valid following page, require its first entry to be
displayed without another scroll, then restore the prior page. All existing
no-send, draft preservation, metadata-only, invalid-chain, authority and
offline/loading assertions remain. A duplicate selected-project identity
hides the browse entry instead of retaining an enabled or disabled authority
target; its pagination is still explicitly required to be disabled. Resource
selection clicks now scroll the actual icon into view before asserting and
clicking, rather than relying on a synthetic offscreen click.

## Current Evidence

`android-api35-catalog-final-20261005.json` binds the current APK/source build,
both device-installed APK hashes and five named start/success JUnit pairs in
each of two viewport runs. The 320x640 portrait and owned-emulator 640x320
wide runs both pass. Their four PNGs were inspected: title, list, close,
previous/next arrows and page number are readable and non-overlapping, and
navigation remains outside the clipped list. The wide run uses a temporary
`wm size` override at density 160; it is layout evidence, not Activity
rotation-recreation acceptance. Original display override and rotation
settings were restored afterward.

App SHA-256:
`46ceae6cc632eb864fb85be5960d5720f677f94d05bfc4d135856d08c625c553`.
Instrumentation APK SHA-256:
`4f38d5ca63f25816b0a3ee48371edbe0e61b1bdea9f28138b378851c56b59746`.
Build receipt SHA-256:
`c862bc2dc23bdb8ba61456ee66e81dfa74c3fedd0f493e134a9d3d82fdfded6a`.

`emulator-camera-api34-catalog-final-20261005.json` passes the same current
APKs through production terminal QR, emulator imagefile camera, production
scanner confirmation and pinned TLS. Explicit Read/Control grants precede
desktop live update, background cache fencing, foreground replay and the
Android-originated prompt/desktop answer. Retained verification checks exact
JUnit identity, all 14 ordered phases, current Android and Rust provenance,
helper/output/PNG hashes, three provider requests, six appended durable turns
and the unchanged original session prefix. Actual deep idle and foreground
reauthentication also pass. Its final PNG was visually inspected. Transport
is the emulator host alias, with no ADB reverse. Physical camera, physical
phone LAN, direct LAN and whole-item acceptance are explicitly false.

After the production changes, actual Gradle debug/release unit variants each
collected 610 tests: 590 passed, 20 skipped, zero failures/errors. The final
invocation succeeded with those unit tasks up-to-date; the intervening edit
was only an instrumentation selector, not production or unit-test code.
The Python suite passes 182
tests; strict Blueprint validation and `git diff --check` pass. These results
do not replace the remaining broader desktop suite or full Rust gate.

## Retained Failures And Remaining Scope

Intermediate outputs stay immutable and are not relabeled as final passes:

- `android-api35-catalog-pinned-20261005.instrumentation.txt` passed four
  tests but exposed the obsolete duplicate-project browse-node expectation.
- `android-api35-catalog-current-landscape-20261005.instrumentation.txt`
  failed one test after system rotation recreated the fixture Activity and
  lost its test-supplied Compose content. The passing wide-size fixture is
  not presented as repair or acceptance of this rotation-recreation test.
- `android-api35-catalog-focus-portrait-20261005.instrumentation.txt` failed
  the resource-opening assertion after an offscreen icon click. The final
  test uses the real visible icon without relaxing its capability assertion.
- `emulator-camera-api34-catalog-focus-20261005.json` is failed. It completed
  the named device test but source changed before the harness could accept
  final build attribution; it remains unusable as current-workflow evidence.

Remaining work includes the full 93-test desktop suite, full Rust gates,
current-artifact API lifecycle matrix, real-phone QR/direct LAN, release
installation/upgrade and per-item Master acceptance. A physical phone was
requested asynchronously; local simulator work did not wait for its arrival.
The API 35 GUI emulator is retained for the operator. The camera harness
stopped only its owned API 34 emulator and bounded desktop/provider fixtures.
