# Rust Security Target Gate

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The blueprint's declared `remote_security` Cargo target now exists as a thin
integration target over the existing adversarial authentication/ACL suite,
keeping the repository's established tests while making the validator's named
security command executable.

```text
cargo test --offline --locked --test remote_security: 12 passed
```

Coverage includes proof vectors, same-device repair grant preservation, ACL
scope fencing, revocation, token/fingerprint refusal and redaction-adjacent
authorization checks. Android Gradle and connected-device gates remain
environment-limited.
