# Status — what's done, what's next

Living handoff for anyone (human or agent) picking this up. Keep it current in the same change that moves a phase forward. The narrative record is in [`HISTORY.md`](HISTORY.md), the decisions in [`decisions/`](decisions/README.md), and the plan in [`plans/dedicated-native-apps.md`](plans/dedicated-native-apps.md).

Last updated: 2026-11-02 — the native Android app's first slice is merged (#180), a second work stream of eight commits (including the rest of the Phase 1 contract) sits **unmerged on the local branch `feat/android-editor`** and was partly verified on a real phone (#188), and the public homepage is live on GitHub Pages (ADR 0009, outside the release train).

## Where we are

| Area | State | Notes |
| --- | --- | --- |
| Rust core (`core/`) | ✅ In use | `core-db`, `core-network`, `core-api`; its own workspace; 62 tests on `main`, 68 on the unmerged branch |
| Contract (ADR 0006) | 🟡 Written but unmerged | vault schema v1 + ordered migrations + golden block fixture are on `main`; the sync-frame fixtures and `docs/CONTRACT.md` exist only on `feat/android-editor` |
| Android app (`apps/android`) | 🟡 Feature slice + unmerged sequel | the first slice is on `main` (#180); eight further commits (PIN gate, settings, accents, recovery, restore, monochrome) are unmerged; the unmerged build was partly verified on a Nothing A063 (#188) |
| Desktop app | ⏸ Not started | ADR 0004 is *Accepted* (Qt 6) with the Phase 0 verification gate still open: Qt is not installed on the current dev machine, so no desktop code exists |
| Tauri app (current desktop + Android) | ✅ Shipping | `v1.13.0`; `main` is migration code — see the README notice |
| Web frontend (`apps/frontend`) | ✅ Used by Tauri | retired when the native apps replace it |
| Homepage (`site/`) | ✅ Live | hand-written static site, built by `scripts/build-site.mjs`, deployed by `.github/workflows/pages.yml`; ADR 0009 |

## Unmerged work — `feat/android-editor` (check this first)

Eight commits dated 2026-10-25 sit on the local branch `feat/android-editor`, were never pushed and are not reachable from `main` or from any remote. They were re-verified on 2026-11-02: `cargo test --manifest-path core/Cargo.toml` passes (68 tests, six of them new wire-format tests), the Android app builds and `:app:testDebugUnitTest` passes, and merging `main` into the branch conflicts only in `docs/HISTORY.md` (both sides appended timeline rows) and, once this file is updated, in `docs/STATUS.md`. The branch's commit messages close #183 and #189–#197, but those issues are still open because it never merged. Its build is also the only one that has been on a phone: #188 records a pass on a Nothing A063 (Android 15) covering the PIN gate (register → confirm → recovery phrase → login → unlock), the share target while the app was running, SAF export, restore with re-authentication, and the opt-in update check.

| Commit | What it carries |
| --- | --- |
| `f7dc9ee` | **Phase 1, item 1**: `core-network`/`core-api` sync frames are locked by checked-in v3 fixtures (`handshake`, `hello`, `auth`, `auth_proof`, `session_failed`, `digest`, `need`, `snapshot`, `ack`) plus round-trip tests. No `main` equivalent. |
| `a8f4bbf` | The vault gate rebuilt around a PIN keypad, with the brand lockup. |
| `d553566` | **Phase 1, item 2**: `docs/CONTRACT.md`, the developer-facing summary of the four versioned surfaces and the compatibility rules. No `main` equivalent. |
| `79ac4cd` | Restore a backup from the register/login screen (#183). |
| `ed88544` | Settings page with auto-lock, editor text size and page width (#189). |
| `8b70f63` | Custom accents, per-page widget sharing (#190, #191, #192). |
| `7feb0fa` | Recovery with the phrase, monochrome theme, visual accent picker (#194–#197). |
| `109e8da` | Retires the Figma documentation plugin. It cites "ADR 0008", which was never committed — `docs/decisions/` jumps from 0007 to 0009, so either write that record or drop the reference. |

The decision this waits on is whether to validate and merge it as a PR (one `HISTORY.md` conflict) or to land the two core commits separately and re-open the Android work. Until it is pushed it exists on this machine only: one `git branch -D` and it is gone.

## Android — done (on `main`)

- Vault gate: create (recovery phrase shown once with acknowledgement), unlock, lock.
- Notes home: search pill with layout/sort controls, folder chips, **four layouts** (List / Gallery / Cards / Icons), three sort orders, create stack (Text / Checklist / Journal / Meeting notes / Folder) with templates shared with the web editor.
- **Editable block editor** over the core's ProseMirror contract: marks kept per character, task toggles write through the core, debounced saves, golden-fixture round-trip test.
- **Sync**: `SyncController` (device-local choice, re-arms on unlock), foreground `SyncService` with Wi-Fi/multicast locks and a live notification, sync screen (address, peers, add-peer), widget-refresh via the core's mutation revision (no JNI hook).
- **Widgets and entry points**: notes list, pinned note, quick capture, Quick Settings tile, launcher shortcuts, share target — over the Keystore-wrapped cache.
- **Settings**: theme mode + five accents (device-local), hide-widgets-while-locked (vault data), Lock now, About.
- **Backup**: SAF export and restore; restore re-authenticates and only accepts a backup of the same vault (ADR 0007).
- **Updates**: opt-in check, core-side SHA-256 + Ed25519 verification (shared with the desktop shell), package-installer hand-off.

## Android — next (in order)

1. **Land the unmerged branch** (above): rebase or merge it, get review, and close #183 and #189–#197 on merge. Re-run the core and Android tests first — they pass today, but nothing in CI would notice if they stopped.
2. **Finish the real-device pass (#188).** Already verified on a Nothing A063: the gate end to end, the share target, SAF export, restore and the update check. Still unverified: the three widgets (including the locked placeholder and an interactive checklist), the Quick Settings tile and launcher shortcuts, background sync to the desktop app over the LAN with Doze/battery behaviour, a TalkBack pass, and installing a release-signed update. The desktop app is not running yet, so the sync item needs it built on the same network.
3. **Cross-editor parity.** Edit the same vault from the native app and the web editor; confirm the JSON opens identically in both, including tables, tasks and marks.
4. **Instrumented tests.** Vault flow and widget cache on-device; Compose UI tests for the editor and home.
5. **Polish**: relative timestamps, snippets in the layouts, favorite/pin from the note menu, folder rename/delete, archive and trash screens.

## Core — next

1. **Land the sync-frame golden fixtures** — they already exist on `feat/android-editor` (`f7dc9ee`), not on `main` (ADR 0006).
2. **Land the contract document** — `docs/CONTRACT.md` exists on the same branch (`d553566`), covering the four versioned surfaces and the compatibility rules.
3. Nothing pending for the widget projection; `widget_notes()` and the revision counter are covered by tests.

## Desktop — decided, gated and unstarted

ADR 0004 is *Accepted* (Qt 6) but gated: Phase 0 must confirm the Qt toolchain, the editor slice, packaging/signing/update and accessibility before implementation, and Qt cannot be installed on the current dev machine (no passwordless sudo). No desktop implementation exists; if the gate fails, the measured Avalonia fallback supersedes the ADR (the spike evidence is recorded in ADR 0004). The Tauri desktop shell keeps shipping from `core/` in the meantime.

## Homepage — shipped, outside the release train

The public homepage in `site/` is live at <https://pranesh-selvaraj.github.io/Enclave/> and redeploys from `main` on any change to `site/`, `docs/brand/`, `docs/screenshots/`, the Inter sources or the build script. It is not part of an app release and carries no version: `npm run site:build` produces the disposable `site-dist/`, and the Pages workflow publishes it. Prose, screenshots and platform status are maintained by hand — the edit rules in [`site/README.md`](../site/README.md) make keeping them in step with `README.md` and `SECURITY.md` an explicit review obligation (ADR 0009).

## Release and operations

- Changelog and release conventions live on `main`; a release publishes the matching changelog entry as the release body (CI enforces it).
- `v1.13.0` is the latest stable release and the supported product; `main` is migration code, and the README says so.
- The Android app has landed (#180) but is not in a release: a first native release needs the real-device pass first.

## Known gaps and limitations

- **CI never builds or tests `apps/android` (#210).** No workflow references it: `android.yml` builds the Tauri shell (`src-tauri/gen/android`, flavours `universal`), and `build.yml` runs `cargo test` for the core and the desktop shell. The native app's compile errors, JVM tests and binding drift would all land silently.
- **Device coverage is partial (#188).** Nothing from `main` has run on a phone; the unmerged branch covered the gate, share, export, restore and the update check on a Nothing A063, while widgets, the tile, background sync, TalkBack and a release-signed update install have not been exercised on hardware. The KVM-less emulator is software-rendered and unusably slow: an attempt on 2026-10-17 reached the vault gate before the system and Pixel Launcher ANR'd, and six of its twelve captures are blank frames.
- **Foreign-vault restore** is deliberately out of scope (ADR 0007).
- **Desktop updater orchestration** is still duplicated between the shell and the core; only the signature verifier and pinned key are shared.
- **The homepage can drift from the product** — its prose, security claims and platform status are hand-maintained, and it is not versioned with the app, so it can describe `main` while the supported release is older.
- **`docs/decisions/0008` was never written** although the unmerged `109e8da` cites it; numbers are assigned once and never reused, so the record is owed.
- **The Android CI gate is red on `main` (#209).** The committed Kotlin bindings under `src-tauri/gen/android` are missing five FFI methods (`backup_vault`, `check_update`, `download_update`, `restore_vault`, `revision`) and the `UpdateInfo` record, so the "Verify UniFFI Kotlin bindings match core-api" step fails on every PR and nothing after it runs. The fix — regenerating them and pinning generation to `--no-format` — is committed on `fix/209-uniffi-bindings` and waits to be merged.
- **Open PR #208** is a Dependabot major bump (`@sveltejs/kit` 2 → 3) on the frontend that the migration retires; close it instead of merging, and skip future majors there.
- **Merged branches safe to delete**: `brand/identity` (#181; its two commits are squash-merged, verified file-by-file against `main`), `docs/desktop-stack-qt6` (#177), `feat/homepage`, `feat/desktop-app`. **Do not delete `feat/android-editor`** — it is the unmerged branch described above.
- **`feat/web-mobile-apps`** (remote) is an abandoned pre-restructure experiment ("split into desktop/web/mobile apps"); do not merge — delete when convenient.

## How to resume

```bash
# core (host toolchain)
source ~/.enclave-dev/env.sh
cargo test --manifest-path core/Cargo.toml

# android: build the core into the app, then the APK
source ~/.enclave-dev/android-env.sh
./apps/android/scripts/build-core.sh
cd apps/android && ./gradlew assembleDebug testDebugUnitTest

# homepage
npm run site:build && python3 -m http.server 8080 -d site-dist

# release pre-flight before tagging
npm run release:check
```
