# Status — what's done, what's next

Living handoff for anyone (human or agent) picking this up. Keep it current in the same change that moves a phase forward. The narrative record is in [`HISTORY.md`](HISTORY.md), the decisions in [`decisions/`](decisions/README.md), and the plan in [`plans/dedicated-native-apps.md`](plans/dedicated-native-apps.md).

Last updated: 2026-11-12 (v1.15.0) — the branded gate (lockup → register → repeat PIN → seed → login) is verified on a real Nothing A063, can restore a backup or recover a forgotten PIN with the recovery phrase without creating a vault, and Settings now has auto-lock, editor text size/page width, a visual custom-accent picker, a Monochrome theme and per-page widget sharing, and the home/note surfaces now use a Keep-inspired drawer, card previews and action sheet; the Phase 1 contract is complete; the Figma documentation plugin and its token export were retired, and the brand vectors live in [`docs/brand/`](brand/README.md).

## Where we are

| Area | State | Notes |
| --- | --- | --- |
| Rust core (`core/`) | ✅ In use | `core-db`, `core-network`, `core-api`; its own workspace; 68 tests |
| Contract (ADR 0006) | ✅ Complete | vault schema v1 + ordered migrations + golden block fixture; transport/app sync frames and handshake crypto locked by fixtures; [`CONTRACT.md`](CONTRACT.md) documents all four surfaces and the compatibility matrix |
| Android app (`apps/android`) | 🟡 Feature slice | builds and unit-tests clean; the branded gate is verified on a real phone; the rest of the manual pass is pending |
| Desktop app | ⏸ Parked | ADR 0004 is *Proposed* (Qt 6 leaning; per-OS trio an escalation; Avalonia a measured fallback). PR #177 is intentionally **unmerged** |
| Tauri app (current desktop + Android) | ✅ Shipping | `v1.13.0`; `main` is migration code — see the README notice |
| Web frontend (`apps/frontend`) | ✅ Used by Tauri | retired when the native apps replace it |

## Android — done

- Vault gate: create (recovery phrase shown once with acknowledgement), unlock, lock.
- **Branded gate**: the official lockup glides into the form; sliding PIN/Password switch; unlock secret with **no length policy** — any non-empty string, one character included (ADR 0011) — a four-dot placeholder that only grows past four characters, so a short secret is not advertised to whoever is looking at the screen, confirmed with the keypad's check key — with repeat-to-confirm and an in-app keypad; centred 2-per-row recovery phrase; the selected accent drives the switch, dots, buttons and logo gradient.
- **Restore from the gate**: register and login can pick a taken backup before any vault exists. A recovery-phrase restore sets a fresh PIN (a device that still holds the key file can restore with the password); the Settings restore remains.
- **Forgot PIN/password**: the login form offers a recovery-phrase path that unlocks the vault and then requires a new 4–12-digit PIN through `set_vault_password`, so the next unlock works. A wrong phrase leaves the vault locked.
- Notes home: search pill with layout/sort controls, folder chips, **four layouts** (List / Gallery / Cards / Icons), three sort orders, create stack (Text / Checklist / Folder, plus New template and the user's own templates).
- **Folders you can actually manage**: creating one asks for a name (both entry points), long-pressing one in the drawer offers Rename and Delete, and deleting moves its pages back to the root list after saying so — the core's `delete_folder` never deletes documents.
- **Long-press actions**: notes (all four layouts) open a sheet with Open, Pin/Unpin, Send, Show in widgets / Hide, Make a copy and Delete; folders get Rename and Delete. One reusable `ActionSheet` dims the actions that do not apply and tints the destructive one.
- **The gate does not count out loud.** The dot row is decorative and silent to screen readers (`clearAndSetSemantics`), because TalkBack speaking "5 characters entered" would tell anyone in the room the length and undo the four-dot placeholder. A screen-reader user still finds the field by its own label.
- **About shows the installed build**: name and version code read from the package manager (`apps/android/version.properties`, overridable from a tag with `-PappVersionName/-PappVersionCode`), plus the commit and build type. The update check compares that installed version rather than a compile-time constant.
- **Throttled unlock (ADR 0011)**: consecutive failures are counted device-locally (they survive a restart) and attempts are refused on a schedule — the first two cost nothing, then 1 s, 3 s, 10 s, 30 s, a minute, capped at five — with the button reading "Try again in 2s". A successful unlock clears the counter. With no length policy this is the only barrier left against guessing a short secret.
- **Deleting a note asks first**, from the note's own screen and from the long-press sheet, and says plainly that the content is removed and cannot be undone (`delete_document` drops the blocks, embeddings and search rows; only the document row is tombstoned). Screenshots and the recents thumbnail are blocked in release builds (`FLAG_SECURE`); debug builds stay capturable so the device pass can take screenshots.
- **One call for the home screen**: `home_notes()` returns every note with its emoji and plain-text preview, replacing `list_documents()` plus two `get_blocks()` calls *per card*. The preview projection lives in the core (`preview.rs`) and is the same code that builds a share body.
- **User templates (ADR 0010)**: two ways to make one — "New template" in the create stack opens the editor with no page behind it, and "Save as template" in a page's options sheet stores that page's blocks — both under a name the user types. The create stack lists them under YOUR TEMPLATES, a tap starts a page from one, a long press deletes one without touching pages made from it, and they live in the vault's `settings` table, so the schema is unchanged.
- **The create stack no longer ships Journal and Meeting notes**: those were fixed skeletons for work the user can now template themselves. Text, Checklist and Folder remain as creation primitives. The desktop app (`packages/editor/src/templates.ts`) still offers both; documents stay compatible either way.
- **Editable block editor** over the core's ProseMirror contract: marks kept per character, task toggles write through the core, debounced saves, golden-fixture round-trip test, and a placeholder plus a 44dp target on empty blocks so a new page's body can be found and typed into; the note screen has pin/share/overflow actions and a widget/copy/send/save-as-template/delete sheet. The footer action bar was removed — it duplicated the toolbar, and its one unique control (add block) now opens from the sheet.
- **Keep-inspired home**: a navigation drawer (lockup, Notes, New folder, folders, Sync, Settings, Lock now, Help), where Help opens a sheet that routes to the GitHub issue chooser, the open-issue list and the security policy instead of dumping you in Settings, outlined cards with plain-text/checkbox previews, and full-width create pills with a close X. The header is one row — menu, search, lock — with the layout and sort pickers inside the search pill; there is no greeting line. Notes carry an optional emoji (an `icon` block, like the `widget` one) that the Icons layout shows instead of the default note glyph, set from the note's sheet or from a long press on the note. The chosen layout and sort, the note list and its rendered previews are cached in memory for the unlocked session, so returning from a page paints the finished screen instead of an empty grid; the cache is dropped on lock and on restore. The launcher icon is an adaptive icon (foreground vector inside the 72dp safe zone) with a monochrome layer for themed icons, and the sync notification uses a dedicated status-bar silhouette rather than the padded launcher artwork. Opening the stack blurs and dims the home behind a tap-swallowing scrim, so a missed tap closes the stack instead of opening the note underneath.
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

Tracked as GitHub issues: [#184](https://github.com/Pranesh-Selvaraj/Enclave/issues/184) multiple vaults (feature idea), [#185](https://github.com/Pranesh-Selvaraj/Enclave/issues/185) self-contained backups, [#186](https://github.com/Pranesh-Selvaraj/Enclave/issues/186) untitled shared notes, [#187](https://github.com/Pranesh-Selvaraj/Enclave/issues/187) the debug-build updater, [#188](https://github.com/Pranesh-Selvaraj/Enclave/issues/188) the remaining real-device pass. [#183](https://github.com/Pranesh-Selvaraj/Enclave/issues/183) (restore from the gate), [#189](https://github.com/Pranesh-Selvaraj/Enclave/issues/189) (auto-lock/editor preferences), [#190](https://github.com/Pranesh-Selvaraj/Enclave/issues/190) (custom accents), [#191](https://github.com/Pranesh-Selvaraj/Enclave/issues/191) (per-page widgets), [#192](https://github.com/Pranesh-Selvaraj/Enclave/issues/192) (shorter copy), [#194](https://github.com/Pranesh-Selvaraj/Enclave/issues/194) (forgot PIN/password), [#195](https://github.com/Pranesh-Selvaraj/Enclave/issues/195) (remove Lock now), [#196](https://github.com/Pranesh-Selvaraj/Enclave/issues/196) (visual accent picker) and [#197](https://github.com/Pranesh-Selvaraj/Enclave/issues/197) (Monochrome theme) and [#207](https://github.com/Pranesh-Selvaraj/Enclave/issues/207) (Keep-inspired navigation and note actions) are implemented. The remaining Settings ideas are individual requests: biometric unlock [#198](https://github.com/Pranesh-Selvaraj/Enclave/issues/198), screenshot/recents protection [#199](https://github.com/Pranesh-Selvaraj/Enclave/issues/199), background sync policy [#200](https://github.com/Pranesh-Selvaraj/Enclave/issues/200), backup reminders [#201](https://github.com/Pranesh-Selvaraj/Enclave/issues/201), default creation type [#202](https://github.com/Pranesh-Selvaraj/Enclave/issues/202), haptics/reduce motion [#203](https://github.com/Pranesh-Selvaraj/Enclave/issues/203), delete/trash [#204](https://github.com/Pranesh-Selvaraj/Enclave/issues/204), update channel [#205](https://github.com/Pranesh-Selvaraj/Enclave/issues/205) and widget accent [#206](https://github.com/Pranesh-Selvaraj/Enclave/issues/206).

## Platform status and compatibility

| Platform | State |
| --- | --- |
| Android, Tauri | Shipping — `v1.14.0`; rebuilt against the updated core, no visual changes |
| Android, native (Kotlin/Compose) | **Next stable, in development** — in the repository and used on a real device (Nothing A063, Android 15), not released; `0.2.0-native`, unsigned, `versionCode` 2, About shows the installed version and build commit |
| Desktop, Tauri | Shipping — `v1.14.0`; unchanged beyond the core |
| Desktop, native (Qt 6) | **In development** — ADR 0004 accepted, verification gate still open, no code |

**Supported pairing: two builds from the same release train.** The native Android stable is **not supported against the previous stable desktop (v1.13.0)** — different app on the same core, sync never exercised against the Tauri desktop, and it writes the app-level blocks (`icon`, `widget`) that build ignores. The vault schema (v1) and wire protocol (v3) are unchanged, so this is a support boundary, not a format break.

## Planned, not started

- **Multiple vaults with independent secrets (ADR 0012, #222).** One install, several vaults, one unlock field, and no surface that reveals how many exist — so a user under coercion can open an innocuous vault. The execution criteria are in [`plans/multiple-vaults.md`](plans/multiple-vaults.md), which states plainly that the claim is *coercion resistance*, not deniability against device forensics. Nothing is implemented yet.

## Release and operations

- Changelog and release conventions live on `main`; a release publishes the matching changelog entry as the release body (CI enforces it).
- `v1.15.0` is the latest stable release and the supported product; `main` is migration code, and the README says so. Its artifacts are the Tauri desktop/Android builds — the native Android app is not in this release (it is at `0.2.0-native`, unsigned, `versionCode` 2).
- **#180 is merged** into `main` (the Android app up to the restore work). The local `feat/android-editor` commits (contract fixtures, branded gate, design docs, gate restore + settings) are still local and need a PR.

## Known gaps and limitations

- **Full real-device pass pending** — the gate, share target, export, restore and update check are verified on a Nothing A063; widgets, the QS tile, background sync, TalkBack and a release-signed update install still need the checklist. The emulator (no `/dev/kvm`) was only usable for launch/render checks.
- **The create stacks differ between the apps.** The native app offers Text / Checklist / Folder plus user templates; the Tauri app still offers Journal and Meeting notes. Pages stay compatible, but the two stacks are no longer the same list.
- **Templates do not sync between devices.** They live in the vault's `settings` table, which the sync protocol does not carry (documents, blocks and folders only), so a template made on the phone reaches the desktop through backup/restore, not LAN sync (ADR 0010 records the trade-off).
- **Deletes are permanent in practice.** The document row is tombstoned for sync, but its blocks, embeddings and search rows are deleted on the spot, so there is no trash to restore from (#204 tracks the UI). The confirmation dialogs say so rather than implying an undo.
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
