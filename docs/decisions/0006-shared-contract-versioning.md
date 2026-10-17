# 0006. Shared contract: vault, block schema, sync protocol, FFI

- **Status:** Accepted
- **Date:** 2026-10-17
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

From 0002 and 0003: two apps share no UI code but must stay compatible with each other and with every vault already on disk. Compatibility is not a nice-to-have here — it decides whether two devices can sync and whether a user's data survives an app upgrade. The surfaces that must never drift are the vault/database schema, the block/document JSON schema, the sync protocol and the FFI boundary between the core and each app.

There is partial prior art: the sync protocol already carries an explicit version (`PROTOCOL_VERSION = 3` in `core-network`), and `core-api` already exposes a frozen UniFFI surface. The vault schema, however, has grown through ad-hoc migrations and does not yet have a single declared version.

## Decision

The Rust core owns all four contract surfaces, and each carries an explicit version:

| Surface | Owner | Version mechanism | Compatibility rule |
| --- | --- | --- | --- |
| Vault / SQLite schema | `core-db` | a declared schema version stored in the database, with ordered migrations | forward migrations only; every released version must be able to open every older vault |
| Block / document JSON | `core-api` / `core-db` | a declared schema version in document metadata | additive changes are compatible; breaking changes require a migration and a version bump |
| Sync protocol | `core-network` | `PROTOCOL_VERSION` (currently 3), negotiated in the handshake | peers refuse mismatched versions with explicit "update this/the peer device" guidance; never silently misinterpret frames |
| FFI boundary | `core-api` | UniFFI surface + C ABI, versioned with the core's semver | additive changes within a major; breaking changes require a core major version and app updates |

Operating rules:

- **A core release is a contract release.** Any change to a surface above updates the contract tests in the same commit.
- **Golden fixtures**: the core test suite keeps canonical vault files, block-schema documents and sync frames. Every app build in CI runs the interop tests against the same core SHA, so "desktop and Android can still sync" is a build result, not an assumption.
- **Apps pin the core**: released apps pin an exact core version; development may track a range. A compatibility matrix (app ↔ core ↔ protocol ↔ schema) is published with the core.
- **No per-app schemas.** An app must not invent document shapes, protocol frames or storage layouts; it consumes and produces what the core validates.

## Consequences

- Contract changes are cheaper and safer: one authoritative implementation, one CI signal.
- Apps can be released independently as long as they agree on the contract version.
- The core's release process is heavier: a protocol or schema change needs fixtures, an ADR, and a note in the changelog.
- Introducing an explicit vault schema version (and formalising existing migrations) is required work in Phase 1, since the current code lacks it.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Let each app own its schema and translate | Guarantees eventual divergence and sync corruption; impossible to audit. |
| Negotiate protocol downgrades with older peers | The current design refuses mismatches on purpose; silent interpretation of frames with a shared vault key is exactly the failure mode to avoid. |
| Version the core by app version instead of semver | Couples unrelated apps and makes "which core can open this vault" unanswerable. |

## Prior art / what this replaces

This formalises what already exists: a versioned sync protocol from v1.9.0 and a frozen UniFFI surface from v1.10.0. It adds the missing pieces — an explicit vault schema version and enforced interop tests — required before two independently-developed apps rely on the same core.
