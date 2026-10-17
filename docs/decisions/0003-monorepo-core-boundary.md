# 0003. Monorepo with the core as the only shared code

- **Status:** Accepted
- **Date:** 2026-10-17
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

Decision 0002 creates two dedicated apps that share no UI code but must share the vault, crypto, block schema and sync protocol. Whatever the repository layout, those contract surfaces have to stay in lockstep: a desktop app that writes protocol v4 while Android still speaks v3 breaks sync; two implementations of the crypto break the security model.

The question is therefore not "share code or not" — the core must be shared — but where that shared code lives and how the apps consume it.

## Decision

Keep **one repository** with the core as the only shared code and the apps as fully independent projects:

```text
enclave/
├── core/                     # the only shared code (Rust) — the contract
│   ├── core-db/              # SQLCipher, FTS5, sqlite-vec, vault crypto
│   ├── core-network/         # mDNS, WebSocket, authenticated + AEAD sync
│   └── core-api/             # stable FFI surface (UniFFI; C ABI for desktop)
├── apps/
│   ├── desktop/              # dedicated desktop app — its own stack, own UI
│   └── android/              # dedicated Android app — its own stack, own UI
├── docs/                     # decisions, history, plans, protocol spec
└── .github/workflows/        # path-filtered: core, desktop, android
```

- CI is **path-filtered**: an Android-only change never builds desktop and vice versa; a core change builds both and runs the interop tests.
- Releases are tagged per component (`core-vX.Y.Z`, `desktop-vX.Y.Z`, `android-vX.Y.Z`) so cadences stay independent.
- If team size or release cadence later justifies separate repositories, the core is extracted into its own repository and **published as versioned artifacts** (crate for Rust consumers, `.so` + Kotlin bindings for Android, static/dynamic library with a C ABI for desktop). Apps pin exact core versions for releases. A git submodule or a copied core is never acceptable.

## Consequences

**Why this is the right default now**

- A contract change (new block type, sync frame or vault migration) lands in one commit across core, desktop and Android — no coordinated three-repo dance.
- One CI run can build both apps against the same core SHA and run vault-format and sync-protocol fixtures across them, so cross-device compatibility is proven per commit instead of hoped for.
- One security review surface: one `cargo audit`, one place where crypto and key handling are reviewed, one dependency policy.
- One compatibility matrix (app ↔ core ↔ protocol ↔ schema) is easier to maintain than three.
- Repository layout is orthogonal to code sharing: the monorepo does **not** force shared UI. `apps/desktop` and `apps/android` can share zero files.

**Costs**

- Mixed toolchains (Rust, Gradle/Kotlin, and the desktop toolkit from 0004) in one checkout, and a larger repository to clone.
- CI must be scoped carefully so Android jobs do not run for desktop-only changes (path filters).
- Ownership boundaries need discipline (CODEOWNERS per directory).

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Three separate repositories (core, desktop, android) now | Turns every contract change into coordinated PRs and tag bumps; the failure mode is silent protocol skew between devices. Worth doing later if cadence or ownership demands it, not while the team is small. |
| Git submodule for the core | Brittle, easy to pin inconsistently, poor tooling for cross-repo refactors. |
| Monorepo with shared UI packages | Directly contradicts 0002's "no shared UI". |
| Copy the core into each app | Duplicates crypto and sync; the first divergence breaks compatibility or security. |

## Prior art / what this replaces

The repository already is a monorepo: `apps/frontend`, `packages/*` and `src-tauri/crates/core-*` coexist under path-filtered workflows. This ADR makes the boundary explicit: after the migration, `core/` is the only shared directory, and the shared UI packages (`packages/ui`, `packages/editor`) become part of the current Tauri app rather than a cross-platform layer.
