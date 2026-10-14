# Retained Terminal QR Regression

`false-center.pgm` is the unmodified production `host_pair::qr_lines` output
retained from the failed source checker on 2026-10-04. SHA-256:
`b877ab32979ca2e0e4ae10d9b0d396560551941c8f97bd0a2251af090166a69b`.
`false-center.json` holds its exact expected payload, observation time and
safety code. The token and fingerprint are deterministic test values, not
credentials. The invitation is expired and no listener is associated with it.

ZXing core 3.4.1's single QR detector stops after confirming a false interior
finder at (58,116), alongside the actual top finders at (30,30)/(230,30),
before discovering the bottom-left finder. Extra margins and TRY_HARDER did
not repair this image. QRCodeMultiReader's geometric detector does decode it.

PairingQrReaderTest pins the artifact digest, reproduces the original detector
failure, checks the camera reader on exact/rotated/off-center/perspective
frames, and refuses ambiguous multiple-symbol frames. No PURE_BARCODE hint
is used by the camera reader. These are synthetic pixel tests, not physical
camera, current APK, LAN transport or device-lifecycle acceptance.
