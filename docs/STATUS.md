# Status — what's done, what's next

Living handoff for anyone (human or agent) picking this up. Keep it current in the same change that moves a phase forward. The narrative record is in [`HISTORY.md`](HISTORY.md), the decisions in [`decisions/`](decisions/README.md), and the plan in [`plans/dedicated-native-apps.md`](plans/dedicated-native-apps.md).

Last updated: 2026-10-25 — the branded gate (lockup → register → repeat PIN → seed → login) is verified on a real Nothing A063, can restore a backup or recover a forgotten PIN with the recovery phrase without creating a vault, and Settings now has auto-lock, editor text size/page width, a visual custom-accent picker, a Monochrome theme and per-page widget sharing; the Phase 1 contract is complete; design tokens and the generated Figma documentation live in [`docs/design/`](design/README.md).

## Where we are

| Area | State | Notes |
| --- | --- | --- |
| Rust core (`core/`) | ✅ In use | `core-db`, `core-network`, `core-api`; its own workspace; 68 tests |
| Contract (ADR 0006) | ✅ Complete | vault schema v1 + ordered migrations + golden block fixture; transport/app sync frames and handshake crypto locked by fixtures; [`CONTRACT.md`](CONTRACT.md) documents all four surfaces and the compatibility matrix |
| Android app (`apps/android`) | 🟡 Feature slice | builds and unit-tests clean; the branded gate is verified on a real phone; the rest of the manual pass is pending |
| Design system (`docs/design/`) | ✅ In use | canonical [`tokens.json`](design/tokens.json), the generated Figma documentation plugin, brand watermarks and curated gate screenshots |
| Desktop app | ⏸ Parked | ADR 0004 is *Proposed* (Qt 6 leaning; per-OS trio an escalation; Avalonia a measured fallback). PR #177 is intentionally **unmerged** |
| Tauri app (current desktop + Android) | ✅ Shipping | `v1.13.0`; `main` is migration code — see the README notice |
| Web frontend (`apps/frontend`) | ✅ Used by Tauri | retired when the native apps replace it |

## Android — done

- Vault gate: create (recovery phrase shown once with acknowledgement), unlock, lock.
- **Branded gate**: the official lockup glides into the form; sliding PIN/Password switch; 4-digit PIN with repeat-to-confirm and an in-app keypad; centred 2-per-row recovery phrase; the selected accent drives the switch, dots, buttons and logo gradient.
- **Restore from the gate**: register and login can pick a taken backup before any vault exists. A recovery-phrase restore sets a fresh PIN (a device that still holds the key file can restore with the password); the Settings restore remains.
- **Forgot PIN/password**: the login form offers a recovery-phrase path that unlocks the vault and then requires a new 4-digit PIN through `set_vault_password`, so the next unlock works. A wrong phrase leaves the vault locked.
- Notes home: search pill with layout/sort controls, folder chips, **four layouts** (List / Gallery / Cards / Icons), three sort orders, create stack (Text / Checklist / Journal / Meeting notes / Folder) with templates shared with the web editor.
- **Editable block editor** over the core's ProseMirror contract: marks kept per character, task toggles write through the core, debounced saves, golden-fixture round-trip test.
- **Sync**: `SyncController` (device-local choice, re-arms on unlock), foreground `SyncService` with Wi-Fi/multicast locks and a live notification, sync screen (address, peers, add-peer), widget-refresh via the core's mutation revision (no JNI hook).
- **Widgets and entry points**: notes list, pinned note, quick capture, Quick Settings tile, launcher shortcuts, share target — over the Keystore-wrapped cache.
- **Settings**: branded page; theme mode (System/Light/Dark/Monochrome), five presets + a visual custom accent picker (hue bar, saturation/brightness square, hex and swatches); editor text size and page width; auto-lock (Off/1/5/15/30 min); hide-widgets-while-locked (vault data); backup/restore; update check; About. The duplicate “Lock now” row was removed — the home header keeps the lock action.
- **Per-page widgets**: a note menu controls **Show in widgets** and **Allow editing from widgets**; the widget cache carries the per-page flag and the pinned widget opens a read-only page instead of ticking its checklist.
- **Backup**: SAF export and restore; restore re-authenticates and only accepts a backup of the same vault (ADR 0007).
- **Updates**: opt-in check, core-side SHA-256 + Ed25519 verification (shared with the desktop shell), package-installer hand-off.

## Android — next (in order)

1. **Real-device pass.** Verified on a Nothing A063: the gate (register → confirm → seed → login → unlock), the running-app share target, export, restore and the update check. What remains: widgets + interactive checklist, the QS tile, background sync to the desktop app, a TalkBack pass and a release-signed update install. The emulator (no KVM) only proved build/install/render; screenshots from that run are in `~/.enclave-dev/emulator-pass-2026-10-17/`.
2. **Cross-editor parity.** Edit the same vault from the native app and the web editor; confirm the JSON opens identically in both, including tables, tasks and marks.
3. **Instrumented tests.** Vault flow and widget cache on-device; Compose UI tests for the editor and home.
4. **Polish**: snippets in the layouts, favorite/pin from the note menu, folder rename/delete, archive and trash screens.

## Core — next

1. **App-side interop CI** — Phase 4: run the same core SHA and golden fixtures from the app builds (Tauri + native Android) so "desktop and Android can still sync" is a CI result, not an assumption.
2. **C ABI** — only once ADR 0004 names the desktop stack; the contract doc marks it as planned, not implemented.
3. Nothing pending for the widget projection; `widget_notes()` and the revision counter are covered by tests.

## Desktop — parked

ADR 0004 stays *Proposed* until the stack is settled. Phase 0's editor slice was spiked in Avalonia (evidence in `~/.enclave-dev/phase0-native-apps/`), and Qt has not been verified because it cannot be installed on the current dev machine (no passwordless sudo). **Do not merge PR #177** until the desktop decision is made on purpose.

## Open work

Tracked as GitHub issues: [#184](https://github.com/Pranesh-Selvaraj/Enclave/issues/184) multiple vaults (feature idea), [#185](https://github.com/Pranesh-Selvaraj/Enclave/issues/185) self-contained backups, [#186](https://github.com/Pranesh-Selvaraj/Enclave/issues/186) untitled shared notes, [#187](https://github.com/Pranesh-Selvaraj/Enclave/issues/187) the debug-build updater, [#188](https://github.com/Pranesh-Selvaraj/Enclave/issues/188) the remaining real-device pass. [#183](https://github.com/Pranesh-Selvaraj/Enclave/issues/183) (restore from the gate), [#189](https://github.com/Pranesh-Selvaraj/Enclave/issues/189) (auto-lock/editor preferences), [#190](https://github.com/Pranesh-Selvaraj/Enclave/issues/190) (custom accents), [#191](https://github.com/Pranesh-Selvaraj/Enclave/issues/191) (per-page widgets), [#192](https://github.com/Pranesh-Selvaraj/Enclave/issues/192) (shorter copy), [#194](https://github.com/Pranesh-Selvaraj/Enclave/issues/194) (forgot PIN/password), [#195](https://github.com/Pranesh-Selvaraj/Enclave/issues/195) (remove Lock now), [#196](https://github.com/Pranesh-Selvaraj/Enclave/issues/196) (visual accent picker) and [#197](https://github.com/Pranesh-Selvaraj/Enclave/issues/197) (Monochrome theme) are implemented. The remaining Settings ideas are individual requests: biometric unlock [#198](https://github.com/Pranesh-Selvaraj/Enclave/issues/198), screenshot/recents protection [#199](https://github.com/Pranesh-Selvaraj/Enclave/issues/199), background sync policy [#200](https://github.com/Pranesh-Selvaraj/Enclave/issues/200), backup reminders [#201](https://github.com/Pranesh-Selvaraj/Enclave/issues/201), default creation type [#202](https://github.com/Pranesh-Selvaraj/Enclave/issues/202), haptics/reduce motion [#203](https://github.com/Pranesh-Selvaraj/Enclave/issues/203), delete/trash [#204](https://github.com/Pranesh-Selvaraj/Enclave/issues/204), update channel [#205](https://github.com/Pranesh-Selvaraj/Enclave/issues/205) and widget accent [#206](https://github.com/Pranesh-Selvaraj/Enclave/issues/206).

## Release and operations

- Changelog and release conventions live on `main`; a release publishes the matching changelog entry as the release body (CI enforces it).
- `v1.13.0` is the latest stable release and the supported product; `main` is migration code, and the README says so.
- **#180 is merged** into `main` (the Android app up to the restore work). The local `feat/android-editor` commits (contract fixtures, branded gate, design docs, gate restore + settings) are still local and need a PR.

## Known gaps and limitations

- **Full real-device pass pending** — the gate, share target, export, restore and update check are verified on a Nothing A063; widgets, the QS tile, background sync, TalkBack and a release-signed update install still need the checklist. The emulator (no `/dev/kvm`) was only usable for launch/render checks.
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
