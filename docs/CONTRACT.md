# The Enclave contract

Two apps share no UI code, but they must open the same vaults and sync with each other and with every build already on disk. Four surfaces make that possible, and the Rust core owns all four. This document is the developer-facing summary required by [ADR 0006](decisions/0006-shared-contract-versioning.md): what each surface is, the exact version this build speaks, and the rules for changing it.

The **golden fixtures** are the executable half of this document. They are checked in, generated from the real constructors, and asserted on every `cargo test`; if the fixture and the code disagree, the test fails before two apps can silently drift apart.

| Surface | Owner | Version | Canonical fixture / guard |
| --- | --- | --- | --- |
| Vault / SQLite schema | `core-db` | **v1** | `vault_stamps_and_migrates_schema_version`, `vault_schema_contract_is_stable` |
| Block / document JSON | `core-db` / `core-api` | **v1** | `core-db/tests/fixtures/block-schema-v1.json` |
| Sync protocol | `core-network` / `core-api` | **v3** | `core-network/tests/fixtures/*-v3.json`, `core-api/tests/fixtures/*-v3.json` |
| FFI boundary | `core-api` | core semver | `core-api/src/ffi.rs`, generated Kotlin bindings |

Operating rules, from ADR 0006:

- **A core release is a contract release.** Any change to a surface above updates the fixtures and this document in the same commit.
- **Apps pin the core.** Released apps pin an exact core version; development may track a range.
- **No per-app schemas.** An app must not invent document shapes, protocol frames or storage layouts; it consumes and produces what the core validates.
- **Never silently reinterpret.** A version mismatch is refused with user-actionable guidance, not read as best-effort.

## 1. Vault schema — v1

`core-db::SCHEMA_VERSION` (currently `1`) is stored in SQLite's `PRAGMA user_version`, inside the encrypted database file, so any build can tell exactly how old a vault is.

Tables in v1: `documents`, `blocks`, `folders`, `embeddings`, `blocks_fts` (FTS5) and `settings`. The `documents` columns are `id`, `title`, `created_at`, `updated_at`, `is_favorite`, `is_archived`, `rev`, `deleted_at`, `folder_id`; `rev` is the LWW clock for sync, `deleted_at` is a tombstone, `folder_id` is the parent folder (`NULL` = root).

Rules:

- Opening a vault runs `migrate`: every step is guarded by its version and idempotent, ordered, and only ever moves forward. Existing migrations are never rewritten or renumbered.
- A fresh vault is stamped with `SCHEMA_VERSION`; a vault from before versioning (`user_version = 0`) is migrated to v1 on open.
- A vault written by a **newer** build is refused with `vault schema vN is newer than this build supports (v1) — update Enclave` — never half-read.
- Columns are append-only within a version. Removing or renaming one requires a version bump and a migration.

## 2. Block / document JSON — v1

An editor document is ProseMirror/TipTap JSON stored in `blocks.content` (a JSON value) and shipped verbatim: the core never rewrites editor JSON. The canonical editor document — every node and mark the contract supports — is `core/core-db/tests/fixtures/block-schema-v1.json`, and `block_schema_v1_json_round_trips_unchanged` proves the core stores and returns it byte-for-byte.

Rules:

- Additive changes (a new node or mark that older builds can ignore) are compatible within v1; a breaking change needs a migration and a schema bump.
- The version is declared by the fixture name and this document, not by a field inside each document. If a future change needs per-document versioning, it becomes part of vault schema v2.
- Apps must produce and consume exactly this shape — the editors on desktop and Android are separate implementations of one document model.

## 3. Sync protocol — v3

`core-network::PROTOCOL_VERSION` is `3`. It is advertised in the opening handshake and checked before a single payload moves.

### Transport and auth

Peers find each other over mDNS (or a manually entered `host:port`) and open a WebSocket on the LAN. `ws://` is acceptable because every post-handshake frame is sealed at the application layer. A session runs in two phases:

1. **Handshake (plaintext JSON text frames).** Both sides send an `auth` frame, then an `auth_proof` proving knowledge of the vault-derived sync key. The version check happens on the peer's `auth` frame.
2. **Data phase (binary, sealed frames).** `hello` is the first sealed frame; every later frame carries app payloads such as `digest` and `snapshot`.

| kind | fields | notes |
| --- | --- | --- |
| `auth` | `v`, `peer_id`, `name`, `challenge` | `challenge` is 32 random bytes, base64. `v` is the protocol version. |
| `auth_proof` | `peer_id`, `proof` | `proof` is a 32-byte HMAC-SHA256 over both challenges and the sender's id, base64. |
| `hello` | `peer_id`, `name` | First sealed frame; registers the session with the app layer. |
| `session_failed` | `host`, `error` | A failure the UI must surface (wrong vault key, dead handshake). |

Version rules: equal versions proceed; an older or newer peer is refused with explicit "update this device" / "update the peer" guidance; a peer that sends no `v` (anything before v1.9.0) is refused with "update Enclave on both devices". A wrong key, a forged frame or a text frame after auth kills the session.

### Cryptography

Everything derives from the vault key (shared by all devices of one owner; account pairing transfers it and is a separate protocol):

- **Sync key**: `HKDF-SHA256(salt = "enclave-sync-v1", ikm = vault key, info = "sync-key")` → 32-byte PSK. Never equals the database key.
- **Proof**: `HMAC-SHA256(key = sync key, "enclave-auth-v1" || sorted(challenge_a, challenge_b) || peer_id)`.
- **Session key**: `HKDF-SHA256(salt = sorted(challenge_a) || sorted(challenge_b), ikm = sync key, info = "enclave-session-v1")`.
- **Sealed frame**: `24-byte random nonce || XChaCha20-Poly1305(ciphertext + 16-byte tag)`. Sorting the challenges makes both directions derive identical keys.

### App frames (inside sealed frames)

| kind | fields | meaning |
| --- | --- | --- |
| `digest` | `docs`: `[{id, rev, updated_at, deleted_at}]` | What the sender has. The receiver diffs it against its own index. |
| `need` | `ids` | The ids the receiver wants after that diff. |
| `snapshot` | `docs`, `blocks`, `folders` | The requested documents, their blocks and the folders involved. Folders arrive before documents so placement is correct. |
| `ack` | `docs_changed`, `blocks_changed` | The receiver merged the snapshot with these statistics. |

Convergence: `hello` → `digest` both ways → `need` → `snapshot` → merge (document-level last-write-wins on `rev`) → `ack`. A converged pair exchanges `need([])` → `snapshot([])` → `ack`, which refreshes "last synced" without moving payload. Sync carries `documents`, `blocks` and `folders`; embeddings, FTS and settings are local and are not synced. A malformed digest, need or snapshot is logged and dropped — never merged as an empty set and never reported as a successful sync.

### Fixtures

`core/core-network/tests/fixtures/`: `auth-v3.json`, `auth_proof-v3.json`, `hello-v3.json`, `session_failed-v3.json`, and `handshake-v3.json` (fixed vault key, challenges, proofs, session key and a sealed `hello` — byte-exact key-derivation and AEAD vectors).

`core/core-api/tests/fixtures/`: `digest-v3.json`, `need-v3.json`, `snapshot-v3.json`, `ack-v3.json` (canonical app payloads, including a document, a block with editor JSON and a folder).

Tests: `core-network/tests/wire_format.rs` and `core-api/tests/wire_format.rs`.

### App-level blocks

Some block types belong to an app rather than to the editor document. They are
stored in the same `blocks` table, they are never rendered as content, and a
reader that does not know the type must ignore it (the editor's parser already
does):

| Type | Payload | Written by |
| --- | --- | --- |
| `doc` | The canonical editor document (see §2) | every editor |
| `widget` | `{"enabled": bool, "editable": bool}` — whether the page may appear on the home screen and be edited there | Android (per-page widget sharing) |
| `icon` | `{"emoji": string}` — the glyph the Icons layout shows | Android |

They travel with the document through sync and backup like any other block, so a
desktop client opening a phone-created note sees at most an extra block it
ignores. Adding a type here is additive and needs no schema bump; **changing the
payload of an existing type is a contract change** and needs the rules in §5.

## 4. FFI boundary

`core-api` exposes the shell-agnostic surface; when built with the `uniffi` feature it generates the Kotlin bindings consumed by the native Android app (committed at `apps/android/app/src/main/java/uniffi/core_api/core_api.kt`, loaded through JNA). The UniFFI records and functions in `core-api/src/ffi.rs` are the contract: **additive changes within a core major are compatible; a breaking change requires a core major version and app updates.** The surface carries no separate version number — the core's semver plus the fixture tests are the guard.

The C ABI for the desktop stack (ADR 0004) is planned but **not implemented yet**; it will be added with the desktop app and documented here when it exists.

## Compatibility matrix

| Build | Core | Sync protocol | Vault schema | Status |
| --- | --- | --- | --- | --- |
| Tauri app v1.14.0 (desktop + Android) | core 1.x | v3 | v1 | Supported product; the interop baseline. |
| Tauri app v1.13.0 (desktop + Android) | core 1.x | v3 | v1 | Previous release. Opens and syncs with v1.14.0: same schema, same protocol. |
| Native Android `0.1.0-native` (`main`) | core 1.x | v3 | v1 | Next stable, unreleased. Protocol-compatible on paper with both Tauri builds, but **not supported against v1.13.0**: sync has not been exercised against the Tauri desktop, and it writes the app-level `icon`/`widget` blocks that build ignores. |
| Anything released before v1.9.0 | — | no `v` field | unversioned, migrated to v1 on open | Cannot sync with current builds; refused with update guidance. |
| Future desktop app | core 1.x | v3 (or negotiated) | v1 (or migrated) | Must land with a C ABI and fixtures before it ships. |

The matrix is about **interoperation**, not feature parity: a Tauri v1.13.0 vault opens and syncs with the native Android app on `main` because both speak protocol v3 against schema v1.

## Verifying the contract

```bash
source ~/.enclave-dev/env.sh
cargo test --manifest-path core/Cargo.toml
```

A green run means: the vault schema is stamped and locked, the block fixture round-trips verbatim, every protocol frame still serializes to its checked-in bytes, and the handshake crypto still derives the checked-in keys.

## Changing the contract

1. Change the code and the **version constant** (`SCHEMA_VERSION` or `PROTOCOL_VERSION`) in the same commit.
2. For a vault change, append an ordered migration in `migrate`; never touch a released one. For a frame change, add new `vN+1` fixtures rather than editing `vN` files.
3. Regenerate the fixtures from the real constructors and commit them with the updated tests and this document (`v3` → `v4` on a protocol bump).
4. Record an ADR only if the **decision** changes (a new surface, a new compatibility rule). Implementing ADR 0006 needs no new ADR, but a contract version change gets a `docs/HISTORY.md` row.
5. Call out compatibility in the changelog and release notes: what older builds will do (refuse, with which message) and what users must update.

## Related

- [ADR 0006](decisions/0006-shared-contract-versioning.md) — the decision this document implements
- [ADR 0003](decisions/0003-monorepo-core-boundary.md) — monorepo and the core boundary
- [docs/STATUS.md](STATUS.md) — what is done and what is next
- [docs/HISTORY.md](HISTORY.md) — the project timeline
