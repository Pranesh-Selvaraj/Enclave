# Project history

This is the narrative record of **what Enclave was built with, why, what changed, and why**. `CHANGELOG.md` is the user-facing record of what shipped; `docs/decisions/` holds the reasoning in detail; this file is the map between them.

**How to update:** append a row to the timeline whenever the stack, architecture, contract or platform strategy changes, or when a release marks an era boundary. Link the ADR that explains it. Never rewrite history — correct it with a dated note instead.

## Stack eras

| Era | Period | UI | Shell | Core | Why it was chosen | What turned out to be wrong / why we moved on |
| --- | --- | --- | --- | --- | --- | --- |
| **0. Prototype** | Jul 2026 | SvelteKit (early) | Tauri v2 | Rust | Notion-style block editor, encrypted local storage, desktop-first. | Foundation only; renamed Aethernote → Enclave before the first release. |
| **1. Shared web UI** | Jul 2026 – Oct 2026 (current) | SvelteKit + Svelte 5, Tiptap 3 | Tauri v2 (desktop + Android) | Rust (`core-db`, `core-network`, `core-api`) | One UI for desktop and Android, small binaries, fast editor iteration, Rust for crypto/storage/sync. | Browser-engine attack surface, bridged native features, webview battery/memory, no native desktop feel. → ADR 0001, superseded by 0002. |
| **1a. Native Android experiment** | Sep 2026 (v1.9.0), abandoned in v1.10.0 | Jetpack Compose (Keep-style shell) | Tauri webview for the full app | Rust + UniFFI | Native background sync, widgets and feel on Android. | Could not reach feature parity with the shared editor; two UIs to maintain. Reverted to the shared web UI. The UniFFI binding work was kept. → ADR 0001 prior art. |
| **2. Dedicated native apps** | Planned | Native per platform: Qt 6 (desktop, proposed) and Kotlin + Compose (Android) | None (no webview) | The same Rust core, versioned contract | Remove the browser engine from a vault app, get native performance and platform integration, keep one audited core. | *Not yet built — see ADRs 0002–0006 and the plan.* |

## Timeline

| Date | Release / event | What changed | Why | Record |
| --- | --- | --- | --- | --- |
| 2026-07-14 | v0.1.0 | First build: SvelteKit + Tauri, block editor, BIP39/Argon2id/AES-256-GCM storage, Windows CI + NSIS. | Establish the local-first, encrypted foundation. | ADR 0001 |
| 2026-07-15 | v0.3.0 | CI publishes installers only; drops the system OpenSSL dependency. | Clean release pipeline; fewer build prerequisites. | — |
| 2026-08-01 | v0.4.0 | Database v2, Edgeless v2, LAN sync v2, comments and mentions, local AI. | Reach a feature-complete desktop app. | — |
| 2026-08-06 | v1.0.0 | RAG assistants, sync hardening; web platform later removed. | First stable release. | — |
| 2026-08-10 | v1.1.0 / v1.1.1 | RAG v2: `sqlite-vec` inside the encrypted vault, offline ONNX embeddings, generic AI client; CSP WASM fix. | Retrieval stays local and inside the vault. | — |
| 2026-08-12 | v1.2.0 | Android app (beta) via Tauri mobile; P2P sync becomes mutually authenticated and end-to-end encrypted. | Mobile peer; close the sync security model. | — |
| 2026-08-18 | v1.3.0 / v1.3.1 | In-app updates for desktop and Android; unsigned-APK selection fix. | Keep installed copies current without uninstalling. | — |
| 2026-08-25 | v1.4.0 / v1.5.0 | Mobile UX and offline-first opt-in updates; brand mark, icon system, editor upgrades, PDF support. | Mobile usability and product identity. | — |
| 2026-09-01 | v1.6.0 / v1.7.0 | Folders, context menus, Markdown tables/source view; split screen, widgets, mobile glass UI, P2P reliability. | Organization and desktop/mobile workflow depth. | — |
| 2026-09-04 | v1.8.0 | Matte design language, background styles, custom accents, dense desktop dashboard. | Coherent design system. | — |
| 2026-09-09 | v1.9.0 | Sync protocol v3, incremental sync, Android background sync, sync card; native Keep-style Compose Android shell. | Make cross-device sync practical; try a native Android UI. | ADR 0001 prior art |
| 2026-09-18 | v1.10.0 / v1.10.1 | Full web UI on Android; native widgets, share target, quick capture; `core-api` + UniFFI extraction; launch-crash hotfix. | One UI and one trusted core across platforms; the shared UI beat the native shell on parity. | ADR 0001 |
| 2026-09-29 | v1.11.0 | Keep-style phone UI inside the web app; Tiptap 3 migration; dependency security fixes. | Mobile-first notes experience without a second UI. | — |
| 2026-10-01 | v1.12.0 | PIN/password unlock, desktop→phone account pairing, folder sync, Android back gesture, UI-size fix. | Onboarding and sync ergonomics. | — |
| 2026-10-17 | v1.13.0 | Ed25519-signed updates, event-driven Android widgets, SAF file access, UI-size and Glass fixes. | Harden distribution and the Android backlog. | — |
| 2026-10-17 | Changelog and release conventions rewritten; all releases retitled to `vX.Y.Z`, bodies synced to the changelog; `v1.9.0` retroactively tagged; Android `versionCode` drift fixed. | Consistent, professional release history. | The forge is part of the product. | — |
| 2026-10-17 | Decision to build dedicated native apps on the shared Rust core. | Remove the webview; native platform behaviour; keep one audited core. | ADR 0002 |
| 2026-10-17 | Monorepo with the core as the only shared code. | Atomic contract changes and one interop test surface. | ADR 0003 |
| 2026-10-17 | Android app stack: Kotlin + Jetpack Compose. | Native Android with the proven UniFFI path. | ADR 0005 |
| 2026-10-17 | Desktop stack: Qt 6 (provisional) vs Avalonia, decided by the Phase 0 editor spike. | The editor is the deciding cost and risk. | ADR 0004 |
| 2026-10-17 | Contract versioning formalised: vault schema, block schema, sync protocol, FFI. | Two independent apps must never drift. | ADR 0006 |
| 2026-10-17 | Vault schema versioning implemented: `PRAGMA user_version`, ordered migrations, refusal of newer vaults, and a contract test that locks the v1 table/column set. The canonical editor document is now a checked-in golden fixture that the core must preserve verbatim. | First piece of ADR 0006 in code: every released build can tell how old a vault is, migrate it safely, and prove the document format it reads and writes. | ADR 0006 |
| 2026-10-17 | Repository restructured around the plan: the core moved to `core/` (its own Cargo workspace, the only shared code) and `apps/android` was created for the native app; the Tauri desktop shell consumes `core/` by path and keeps shipping. CI, CODEOWNERS and docs follow the new layout. | Implements ADR 0003 and makes the core an independently consumable artifact. | ADR 0003 |
| 2026-10-17 | Native Android app started: a Kotlin + Jetpack Compose project that links the Rust core through the UniFFI/JNA bindings, with the matte theme, vault create/unlock, notes home (search pill, Cards grid, FAB) and a read-only ProseMirror block renderer. It builds a debug APK with `libcore_api.so` for arm64 and x86_64. | First Phase 2 slice: the app now owns its UI and consumes the same core, with no webview. The editable surface is next. | ADR 0005 |

## Open questions

- **Desktop stack** — ADR 0004 is Proposed until the Phase 0 spike records measurements in an Accepted record.
- **Android app** — the first slice (vault, notes home, read-only blocks) builds into an APK. Next: the editable Compose editor, the Create stack and the other three layouts, folders/tags/settings, sync UI, and porting the foreground sync service and Glance widgets from the Tauri Android project.
- **iOS** — out of scope for the current plan, but the contract must not preclude it.

## Notable corrections

- **v1.9.0 was never tagged or released** when it shipped; it was retroactively tagged on 2026-10-17 so the release history is complete. Its release carries no build assets, because the historical build pipeline was not cryptographically signed and was not re-run.
- **Android `versionCode` drifted** at v1.13.0 (the bumped counter was not committed back), causing v1.12.0 and v1.13.0 to ship the same value. Fixed in the 2026-10-17 housekeeping change; `scripts/release-check.sh` fails the next release if it recurs.
