# Migration, Backup And Local Reset

Provisional S2-056 evidence. No whole-item acceptance; current APK/device,
actual backup transport, uninstall/reinstall and host release rollback are not
verified. No authoritative checkbox is promoted.

## Host Migration

The new `cargo test --locked --test remote_migration` target covers the actual
ProjectOwnerPool/SessionStore checkpoint path, without provider execution:

- A legacy owner checkpoint lacking remote_workspace/subtabs imports prompt,
  model and reasoning journal admission identities once; duplicate same-identity
  records do not duplicate bindings. Prompt operation serialization stays legacy
  compatible. Project/session/cursors remain bound; journal bytes stay unchanged.
- Missing checkpoint reconstructs journal identity bindings before exposing
  durable remote controls, without recreating a turn or rerunning selection.
- Fingerprint/operation conflicts, malformed identities and 129 distinct
  bindings refuse before partial checkpoint publication.
- An interrupted temporary file or held host migration lock preserves the old
  checkpoint and usable initial owner; explicit retry after resolving the
  obstruction succeeds. Migrated checkpoint permissions remain 0600 on Unix.
- Unknown schema, unknown fields and relative session mappings refuse while
  preserving their exact bytes for explicit recovery.

Existing remote_subtab tests separately cover deterministic index migration,
stable identities, restart, concurrent CAS and malformed layout refusal.

## Android Policy

Application has allowBackup=false plus explicit legacy fullBackupContent and
API 31+ dataExtractionRules references. Each legacy/cloud/device-transfer
branch excludes root, files, databases, preferences, external files and four
device-protected storage domains using path dot. Keystore keys remain outside
application file export; paired metadata remains in noBackupFilesDir. Any
reinstall/device move requires fresh QR pairing, not inheritance of cached
operation authority. Local reset is not host-side device revocation.

`tools/validate_stage2_android_backup.py` checks exact source references,
all-domain exclusions and absence of an app custom backup agent; the cached
source verifier invokes it. Negative fixture tests cover enabled backup,
redirected policies, omitted/partial/duplicate/unknown domains and inclusions.
This does not inspect the Gradle merged release manifest or exercise an OEM
backup/transfer service. AAPT2 compiled the actual resource tree and linked a
minimal temporary policy-only manifest to prove API 35 resource references;
that resource-only artifact is not an executable Zenpi APK.

## Prepared Android Device Tests

PairedHostMigrationTest compiles these tests using actual AndroidKeyStore and
encrypted store paths; none is claimed device-executed in this evidence:

- Legacy encrypted v1/v2/v3 display restores without write-on-read, preserves
  immutable pending requests, denies background resume while pending and rewrites
  current v4 cache only after confirmed completion. Legacy assistant turns
  retain a local summary-omission marker until authenticated snapshot recovery;
  migration alone does not clear it or advance the transcript cursor.
- Missing Keystore key refuses reads, resume and re-pair without creating a
  replacement key or changing original ciphertext; explicit reset permits new
  pairing.
- Copying ciphertext to another independently keyed namespace and tampering
  ciphertext both refuse; original bytes remain until explicit reset.
- Confirmed local wipe removes metadata/signing keys, pending records and
  interrupted .bak/.new/.reset files while preserving another host namespace.

BackupPolicyTest compiles inspection of the packaged application backup flag,
binary manifest references and complete cloud/transfer/legacy XML exclusions.
It must still run against the actual built APK on the supported device matrix.
An actual uninstall/reinstall test must independently prove removed signing
and encryption keys, no restored pairing and no silent host revocation claim.

Exact commands, counts and hashes are in
[progress-20261002-migration-backup.md](progress-20261002-migration-backup.md).
