# 0007. Vault restore semantics

- **Status:** Accepted
- **Date:** 2026-10-17
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

Export shipped first (ADR 0005): the core writes a consistent, still-encrypted snapshot with SQLCipher's `VACUUM INTO`, and Android copies it to a user-chosen SAF location. Restore is the other half, and it is destructive: it replaces the user's live vault. It also has to answer two questions the export does not:

1. **Which key opens the backup?** The database key derives from the 12-word seed phrase; `vault.key` is the seed re-encrypted under the user's password. A backup taken from *this* vault opens with this vault's key. A backup from a different vault would need *that* vault's phrase, and its `vault.key` is not part of the snapshot.
2. **Who is allowed to run it?** A restore is the most dangerous operation in the app. Reusing an unlocked session means anyone holding the unlocked phone can replace the vault.

## Decision

- **A restore re-authenticates the owner.** It takes the vault's password **or** its 12-word recovery phrase and derives the key from it; the core never retains a database key for this purpose.
- **Only a backup of the same vault is accepted.** The staged backup must open with the derived key. A foreign vault is refused with an explicit message ("belongs to a different vault"); restoring one is a separate flow because it also has to replace the `vault.key` pairing, and that decision is deliberately deferred.
- **Validate before touching anything.** The backup is written to a staging file, opened with the derived key and checked for a schema newer than this build (`SCHEMA_VERSION`). Only then is the live vault closed and its file replaced.
- **The replacement is atomic and ends locked.** The staged file is renamed over `enclave.db`; stale `-wal`/`-shm` files are removed, since SQLCipher would refuse a mismatched WAL. The core and its network are stopped first, and the vault is locked afterwards so the next unlock runs the normal migration path.
- **The shell stops background sync first.** A restore while the sync loop is merging would race the swap; the app pauses sync, and the core closes the network as part of locking.

## Consequences

- Restoring your own backup is a two-tap flow on the phone: pick the file, enter the password or phrase, unlock again.
- The backup is the encrypted database, so the file on disk or in cloud storage stays unreadable without the secret — a restore needs it, and a wrong secret is rejected before anything is written.
- Until the foreign-vault flow exists, moving between vaults is: restore on a desktop (which has its own restore path) or start fresh and sync from a peer. This is an explicit limitation, not an oversight.
- Memory cost is the size of the backup (it crosses the FFI as bytes), which is acceptable for a phone vault and is bounded by a size cap in the shell.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Restore under the unlocked session (no re-auth) | Anyone with the unlocked device could replace the vault; the cost of asking for the secret once is trivial. |
| Include `vault.key` in the snapshot and swap both | Would make foreign-vault restore seamless, but changes the backup format and widens the blast radius: a snapshot would then carry the password-protected seed too. Revisit when a foreign-vault flow is actually wanted. |
| Accept any SQLCipher file without validating | A wrong or corrupt file would replace a working vault and leave the user locked out. |
| Restore by copying files from the shell | The vault key handling belongs in the core; a shell doing file surgery on `enclave.db` is exactly the duplication ADR 0003/0006 forbid. |

## Prior art / what this replaces

Extends the export added in ADR 0005 and uses the schema version from ADR 0006 (a backup from a newer build is refused rather than opened by an older one).
