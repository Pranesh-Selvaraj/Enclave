# Status — what's done, what's next

Living handoff for anyone (human or agent) picking this up. Keep it current in the same change that moves a workstream forward. The narrative record is in [`HISTORY.md`](HISTORY.md), the decisions in [`decisions/`](decisions/README.md), the plan in [`plans/dedicated-native-apps.md`](plans/dedicated-native-apps.md) and the desktop feature migration in [`plans/desktop-feature-parity.md`](plans/desktop-feature-parity.md).

Last updated: 2026-11-16 — `main` is **native-only**. The Tauri/Svelte desktop and Tauri Android shell are retired (ADR 0014); the Rust desktop and native Android app are the products. The Rust desktop now edits notes through the in-house headless engine and autosaves; the native editing bar (IME, AccessKit, list/table polish) is the open gate. Native release pipelines are the next release-level work.

## Where we are

| Area | State | Notes |
| --- | --- | --- |
| Rust core (`core/`) | ✅ In use | `core-db`, `core-network`, `core-api`; 73 tests; contract fixtures green |
| Contract (ADR 0006) | ✅ Complete | vault schema v1, block JSON v1, sync protocol v3; app boundary documented |
| Native Android (`apps/android`) | 🟡 Feature slice | builds and unit-tests clean; gate and much of the device pass verified on a Nothing A063; remaining hardware checklist in #188 |
| Rust desktop (`apps/desktop`) | 🟡 Editor prototype | vault gate, notes list, editable text/marks/tasks, split/merge, undo/redo, autosave; 19 tests; IME/accessibility and advanced blocks next |
| Tauri/Svelte app | 🗄️ Retired from `main` | code in Git history and `v1.15.0`; historical installers remain; no new builds |
| Public homepage (`site/`) | ✅ Static | no framework, no third-party requests, no analytics |

## Rust desktop — done

- Vault create, recovery phrase shown once, unlock, lock; passwords and the phrase are zeroized; a missing runtime or failed lock is surfaced instead of pretending to lock.
- Shared `com.enclave.app` data directory so existing vaults can be opened during migration; `ENCLAVE_APP_DIR` isolation.
- Notes list, note creation modal, load selected note through the core and a formatted document view.
- Editable text blocks: paragraphs, headings, marks, lists, tasks, quotes, code, tables, rules and image placeholders.
- Headless engine: ProseMirror JSON, validation, selection, insert/delete/replace with mark preservation, block split/merge, headings, task toggles and undo/redo.
- Save path through `core-api`: the doc block is created or updated; autosave is debounced, `Ctrl/Cmd+S` works, switch/lock/close save best-effort, and a failed switch-save restores the old note.
- Mark toolbar (B/I/S/code), save state and a headless render test of the canonical fixture.

## Rust desktop — next (in order)

1. **Native editing bar:** real IME pass (CJK/dead keys/combining), AccessKit/screen-reader and keyboard-only pass, 2,000-block latency measurement.
2. **Editor completeness:** list-item Enter/nesting, table row/column/cell navigation, title rename, Markdown paste/input rules, true bold typography ([#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237)).
3. **Home and organisation:** folders, favorites, tags, search, layouts, trash/archive, templates ([#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236)).
4. **Advanced surfaces:** whiteboard, databases, graph, split view, command palette ([#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238)).
5. **AI service:** local embeddings/RAG, opt-in remote, read-only agent with app-enforced permissions ([#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239)).
6. **Settings/platform:** themes, auto-lock, backup/restore, native menus/dialogs/tray/notifications, packaging, signed updates ([#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240), [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241)).

## Native Android — done

- Vault gate: create, recovery phrase acknowledgement, unlock, lock; recovery-phrase restore and forgotten-PIN flow.
- Branded gate, no length policy (ADR 0011), four-dot placeholder, unlock throttling, `FLAG_SECURE` in release builds.
- Notes home: search, four layouts, sort, folder chips, create stack, user templates, long-press actions, emoji icons, Keep-inspired drawer.
- Editable block editor over the ProseMirror contract, task toggles, debounced saves, golden-fixture round-trip.
- Sync controller, foreground service, mDNS/manual peer, encrypted frames; widgets, Quick Settings tile, launcher shortcuts and share target over the Keystore-wrapped cache.
- Settings, theme/accent, editor size/width, auto-lock, backup/restore, update check, About; one-call `home_notes()` and core-side previews.

## Native Android — next

1. **Real-device pass:** widgets + interactive checklist, Quick Settings tile, background sync to a desktop peer, TalkBack, release-signed update install ([#188](https://github.com/Pranesh-Selvaraj/Enclave/issues/188)).
2. **Cross-editor parity** with the Rust desktop and the retired Tauri vaults.
3. **Instrumented tests** for the flows that regressed on device ([#227](https://github.com/Pranesh-Selvaraj/Enclave/issues/227)).
4. **Polish:** favourites/pins, folder rename/delete, archive/trash, shorter copy.

## Core — next

1. **Sync v4:** block-level deltas, compression, attachments, settings/templates and resumable transfers; Android first, then the Rust desktop. This benefits the primary product before desktop parity.
2. **App-side interop CI:** native Android + Rust desktop against one core SHA and the golden fixtures, including a Tauri-created vault fixture.
3. **No C ABI:** the desktop depends on `core-api` as a crate; Android keeps UniFFI. Keep it that way.
4. Nothing pending for the widget projection; `widget_notes()` and the revision counter are covered by tests.

## Release and operations

- Release pipelines are being rebuilt for the native apps ([#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241)). Until native Android signing/upload and Rust desktop packaging/updates land, **tag releases pause**; the historical Tauri releases remain installable.
- `scripts/release-check.sh` validates the desktop Cargo version, Android `version.properties` and the CHANGELOG head before tagging.
- CI: `.github/workflows/desktop.yml` (core + Rust desktop), `.github/workflows/android-app.yml` (native Android debug + tag release job), `.github/workflows/pages.yml` (homepage).
- Changelog and release-note conventions live in `CONTRIBUTING.md`; the release body and `CHANGELOG.md` must carry the same detailed prose.

## Compatibility and known gaps

- **Tauri vaults:** schema v1 / protocol v3 are unchanged. A Tauri-created vault must open in both native apps; the Tauri app is a historical peer, not a current build.
- **Desktop feature parity is explicit:** see [`plans/desktop-feature-parity.md`](plans/desktop-feature-parity.md). Whiteboard, databases, graph, split view, AI, widgets, import/export, accessibility and packaging are planned, not silently dropped.
- **Android is not yet the signed release build:** the native APK/AAB is built in CI for tags but the release-signing/update path needs the device/release pass.
- **Templates do not sync between devices** (vault settings are local); sync v4 is the place to change that.
- **Deletes are permanent in practice**; the document row is tombstoned for sync but blocks/embeddings/search rows are removed. Trash is #204.
- **Multiple vaults** (ADR 0012) are planned, not implemented.
- **Tauri/Svelte code is not in `main`:** use the `v1.15.0` tag or earlier history as the feature reference. Do not merge old branches that reintroduce it.

## How to resume

```bash
# core
source ~/.enclave-dev/env.sh
cargo test --manifest-path core/Cargo.toml

# desktop
cargo test --locked --manifest-path apps/desktop/Cargo.toml
cargo run --manifest-path apps/desktop/Cargo.toml

# android
source ~/.enclave-dev/android-env.sh
./apps/android/scripts/build-core.sh
cd apps/android && ./gradlew assembleDebug testDebugUnitTest

# release pre-flight
bash scripts/release-check.sh
```
