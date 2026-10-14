# Gradle Environment Diagnostic

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The Android source gate remains independent of Gradle and passes. A targeted
Gradle diagnosis found the following sequence:

1. The cached `libnative-platform.dylib` is present and is a valid macOS
   arm64 Mach-O library.
2. A fresh writable `GRADLE_USER_HOME` with a copied native library lets
   Gradle reach daemon startup.
3. The daemon then fails while binding its local TCP server with
   `java.net.SocketException: Operation not permitted`.

This confirms the remaining Gradle failure is the sandbox's local socket policy,
not a missing SDK, wrong CPU architecture, or missing native Gradle library.
No socket or daemon restriction was bypassed.
