# Resume Retry Authority Policy

Provisional Stage 2 lifecycle progress at 2026-10-02T12:38:33Z. The current
dirty worktree and all user changes were preserved. No nested agents, commit,
push, checklist promotion, or cleanup was performed.

## Implementation

`ResumeWorker` now classifies typed host authority refusals before applying its
bounded retry budget. `unauthorized`, `revoked_device`, and `owner_mismatch`
are terminal because retrying the same saved device grant cannot repair them.
Typed transient host refusals and transport failures retain the existing two
attempt retry bound. Untyped wire messages and socket exceptions containing a
similar word are not treated as authority evidence.

## Verification

Command completed with exit code 0:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-resume-policy-check2
```

The source check compiled cached API 35 production, unit, and instrumented
artifacts and ran 252 JVM tests successfully. It emitted only existing Kotlin
deprecation/unnecessary-safe-call warnings. Artifact hashes:

```text
2a90eb780529e33f8f458b970b84d17b06ed2b960f6b14cc745f5fa5d20920bc  /private/tmp/zenpi-resume-policy-check2/main.jar
ea6cf71f187dd065a4ada02299ff95a71dbdf09a09a3d48d6c10764812561917  /private/tmp/zenpi-resume-policy-check2/unit.jar
0f50b87051d946725292401124cd8fabde3b65405d7c556e793eace8c3122e53  /private/tmp/zenpi-resume-policy-check2/instrumented.jar
```

The normal Gradle unit command could not run because the sandbox denied access
to the pre-existing Gradle wrapper lock under `~/.gradle`; no permission
escalation or retry was attempted. No emulator/device, TCP/LAN, QR or signed
APK behavior is claimed by this receipt.
