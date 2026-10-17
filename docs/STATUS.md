# Status — what's done, what's next

Living handoff for anyone (human or agent) picking this up. Keep it current in the same change that moves a phase forward. The narrative record is in [`HISTORY.md`](HISTORY.md), the decisions in [`decisions/`](decisions/README.md), and the plan in [`plans/dedicated-native-apps.md`](plans/dedicated-native-apps.md).

Last updated: 2026-10-17 — native Android app is feature-complete for a first release (editor → sync → widgets → settings → backup → updater → restore); nothing has run on real hardware yet.

## Where we are

| Area | State | Notes |
| --- | --- | --- |
| Rust core (`core/`) | ✅ In use | `core-db`, `core-network`, `core-api`; its own workspace; 62 tests |
| Contract (ADR 0006) | 🟡 Started | vault schema v1 + ordered migrations + golden block fixture; sync-frame fixtures and the contract doc pending |
| Android app (`apps/android`) | 🟡 Feature slice | builds and unit-tests clean; no hardware run yet |
| Desktop app | ⏸ Parked | ADR 0004 is *Proposed* (Qt 6 leaning; per-OS trio an escalation; Avalonia a measured fallback). PR #177 is intentionally **unmerged** |
| Tauri app (current desktop + Android) | ✅ Shipping | `v1.13.0`; `main` is migration code — see the README notice |
| Web frontend (`apps/frontend`) | ✅ Used by Tauri | retired when the native apps replace it |

## Android — done

- Vault gate: create (recovery phrase shown once with acknowledgement), unlock, lock.
- Notes home: search pill with layout/sort controls, folder chips, **four layouts** (List / Gallery / Cards / Icons), three sort orders, create stack (Text / Checklist / Journal / Meeting notes / Folder) with templates shared with the web editor.
- **Editable block editor** over the core's ProseMirror contract: marks kept per character, task toggles write through the core, debounced saves, golden-fixture round-trip test.
- **Sync**: `SyncController` (device-local choice, re-arms on unlock), foreground `SyncService` with Wi-Fi/multicast locks and a live notification, sync screen (address, peers, add-peer), widget-refresh via the core's mutation revision (no JNI hook).
- **Widgets and entry points**: notes list, pinned note, quick capture, Quick Settings tile, launcher shortcuts, share target — over the Keystore-wrapped cache.
- **Settings**: theme mode + five accents (device-local), hide-widgets-while-locked (vault data), Lock now, About.
- **Backup**: SAF export and restore; restore re-authenticates and only accepts a backup of the same vault (ADR 0007).
- **Updates**: opt-in check, core-side SHA-256 + Ed25519 verification (shared with the desktop shell), package-installer hand-off.

## Android — next (in order)

1. **Real-device pass.** Install on a phone/emulator and run the manual checklist: unlock → edit → widget render + interactive checklist → background sync to desktop → share target → export → restore → update install. This is the biggest confidence gap: everything so far is build- and unit-tested only.
2. **Cross-editor parity.** Edit the same vault from the native app and the web editor; confirm the JSON opens identically in both, including tables, tasks and marks.
3. **Instrumented tests.** Vault flow and widget cache on-device; Compose UI tests for the editor and home.
4. **Settings gaps** present in the Tauri app: page width, editor font size, auto-lock timeout.
5. **Polish**: relative timestamps, snippets in the layouts, favorite/pin from the note menu, folder rename/delete, archive and trash screens.

## Core — next

1. **Sync-frame golden fixtures** — capture the handshake / digest / need / snapshot / ack frames and assert them in CI (ADR 0006).
2. **Contract document** — a developer-facing summary of the four versioned surfaces (vault schema, block schema, sync protocol, FFI) plus the compatibility matrix.
3. Nothing pending for the widget projection; `widget_notes()` and the revision counter are covered by tests.

## Desktop — parked

ADR 0004 stays *Proposed* until the stack is settled. Phase 0's editor slice was spiked in Avalonia (evidence in `~/.enclave-dev/phase0-native-apps/`), and Qt has not been verified because it cannot be installed on the current dev machine (no passwordless sudo). **Do not merge PR #177** until the desktop decision is made on purpose.

## Release and operations

- Changelog and release conventions live on `main`; a release publishes the matching changelog entry as the release body (CI enforces it).
- `v1.13.0` is the latest stable release and the supported product; `main` is migration code, and the README says so.
- **Open PR: #180** — the whole native Android app (editor → restore). It needs a review; the branch is up to date with `main`.

## Known gaps and limitations

- **No hardware run yet** — the dev machine has no `/dev/kvm`, so no emulator; nothing has been installed on a phone.
- **Foreign-vault restore** is deliberately out of scope (ADR 0007).
- **Desktop updater orchestration** is still duplicated between the shell and the core; only the signature verifier and pinned key are shared.
- **`feat/web-mobile-apps`** (remote) is an abandoned pre-restructure experiment ("split into desktop/web/mobile apps"); do not merge — delete when convenient.
- **`brand/identity`** (local) is merged as #181 and safe to delete.

## How to resume

```bash
# core (host toolchain)
source ~/.enclave-dev/env.sh
cargo test --manifest-path core/Cargo.toml

# android: build the core into the app, then the APK
source ~/.enclave-dev/android-env.sh
./apps/android/scripts/build-core.sh
cd apps/android && ./gradlew assembleDebug testDebugUnitTest

# release pre-flight before tagging
npm run release:check
```
