# Desktop feature parity — retiring the Tauri/Svelte app

The retired Tauri desktop (the `v1.15.0` tag and earlier) is the **feature baseline**. The Rust desktop is the only desktop product on `main`. This document is the migration contract: no feature is considered dropped, and nothing is marked done without a test or a recorded manual pass.

- Tracking: [#235](https://github.com/Pranesh-Selvaraj/Enclave/issues/235); desktop track [#233](https://github.com/Pranesh-Selvaraj/Enclave/issues/233)
- Decision: [ADR 0014](../decisions/0014-native-only-repository.md)
- Retired code: Git history and the `v1.15.0` tag; it is not in `main` by design.

## Status legend

| Mark | Meaning |
| --- | --- |
| ✅ | Implemented and tested in the Rust desktop |
| 🟡 | Implemented in part, or waiting on IME/accessibility/manual verification |
| ⬜ | Planned; issue linked |
| 🗄️ | Retired with the Tauri/Svelte app; not a target (no browser engine) |

## Transfer rules

1. Every retired feature is either delivered, planned with an issue, or explicitly retired by an ADR. No silent drops.
2. The vault schema, ProseMirror block JSON and sync protocol stay unchanged. A Tauri-created vault must open in both native apps; the golden fixtures remain the gate.
3. The new desktop owns its UI. No Svelte/Tiptap component is embedded or ported as a webview.
4. “Done” means an automated test or a documented manual pass (including accessibility where relevant), not a screenshot.
5. New features added to the Rust desktop after the retirement are not required to exist in the baseline; the direction is one-way.

## 1. Shell, vault and home

| Feature | Status | Where it lands / issue |
| --- | --- | --- |
| Create vault, recovery phrase shown once, acknowledgement | ✅ | Branded vault gate + numbered recovery grid + worker |
| Unlock with password, lock now | ✅ | Branded desktop gate; lock now from the shell |
| Restore from a backup; restore with recovery phrase; forgotten-password flow | 🟡 | Core supports restore; desktop UI pending [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Auto-lock (Off/1/5/15/30 min) | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Multiple vaults with no discovery surface (ADR 0012) | ⬜ | #222, [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Home: recents, favorites, pinned notes | 🟡 | Note list exists; favorites/pins [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Folder tree: create, rename, delete, move pages | ⬜ | [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Tags and tag filtering | ⬜ | [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Search: FTS, command palette entry, scoped search | ⬜ | [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Four note layouts (List/Gallery/Cards/Icons), sort orders | ⬜ | [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Trash/archive screens, undo delete | ⬜ | [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Page templates and user templates (ADR 0010) | ⬜ | [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Create stack: text, checklist, folder, templates | 🟡 | New-note modal and text/checklist core; folder/template UI [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Command palette and keyboard shortcuts | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Split view with autosaving panes | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Desktop wallpaper widget / tray quick capture | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Native menus, file dialogs, drag-and-drop, notifications, global shortcuts | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Printing / PDF export | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |

## 2. Editor

| Feature | Status | Where it lands / issue |
| --- | --- | --- |
| Paragraphs, headings 1–3, bold/italic/strike/code/link | ✅ | Headless engine + native view |
| Page title rename | ✅ | Inline title field, saved through the core |
| Bullet, ordered and task lists; task toggles | ✅ | Rendering, toolbar creation, task toggle, Enter-to-new-item, empty-item Backspace and Tab/Shift+Tab nesting [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Block split/merge, Backspace merge, Enter split | 🟡 | Implemented; list/table edge cases [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Undo/redo across edits | 🟡 | Engine history + toolbar; text-editor shortcut polish [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Quote/blockquote, code blocks, horizontal rules | ✅ | Render/edit; language selector [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Tables: render and edit cells | 🟡 | Render + cell text edit; insert/delete rows/columns, cell navigation, merged cells [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Images and PDF/file attachment blocks | 🟡 | Image placeholder only; attachment storage/sync [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Callouts, toggles, bookmarks, page embeds, video/embed blocks | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Database blocks: typed tables, kanban/list/gallery/timeline, linked databases, filters/grouping/sort | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Slash commands, bubble menu, block drag handles/menu | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Page links `[[`, mentions `@`, backlinks | 🟡 | Core backlinks; UI [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Table of contents/outline | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Markdown source toggle, Markdown paste/input rules | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Markdown/HTML export, per-page import/export | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Copy/paste rich text, images, files; clipboard HTML sanitization | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| Find/replace | ⬜ | [#237](https://github.com/Pranesh-Selvaraj/Enclave/issues/237) |
| IME (CJK, dead keys, combining sequences) | 🟡 | TextEdit/IME guard exists; real IME pass pending [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Accessibility (AccessKit tree, screen reader, keyboard-only) | 🟡 | Native TextEdit/egui provides a tree; pass pending [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| 2,000-block typing latency / large-vault virtualization | ⬜ | [#233](https://github.com/Pranesh-Selvaraj/Enclave/issues/233) gate |

## 3. Whiteboard, graph and workspaces

| Feature | Status | Where it lands / issue |
| --- | --- | --- |
| Infinite whiteboard: pan/zoom/select | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Sticky notes, rectangles, ellipses, arrows, text, frames, page embeds | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Mind maps (Tab child), drag/align | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Presentation mode, PNG export | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Whiteboard layout persisted per page | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Backlink graph, zoom/pan/focus | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |
| Split view, draggable divider, autosave both panes | ⬜ | [#238](https://github.com/Pranesh-Selvaraj/Enclave/issues/238) |

## 4. AI and knowledge

| Feature | Status | Where it lands / issue |
| --- | --- | --- |
| Local embeddings for RAG, no endpoint required | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239); current `fastembed/ONNX` implementation is retired with the Tauri shell |
| OpenAI-compatible chat/embeddings (Ollama, llama.cpp, LM Studio, vLLM) | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Remote frontier API opt-in, key storage, no telemetry | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Vault-wide RAG, FTS fallback, sources/citations | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Ask-AI panel, streaming, stop, settings/model manager | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Local model/agent support | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Agent tools with approval and allowlists; no network/shell tools | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |

## 5. Sync and data

| Feature | Status | Where it lands / issue |
| --- | --- | --- |
| mDNS discovery, manual peer, encrypted WebSocket sync | ✅ | Core; desktop sync UI pending |
| Sync screen: address, peers, add peer, status/last sync | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Background sync, notification/status | ⬜ | [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Conflict handling and document-level LWW | ✅ | Core; UI surfacing pending |
| Sync v4: block deltas, compression, attachments, settings/templates, resumable transfers | ⬜ | Core work first (benefits Android); [#239](https://github.com/Pranesh-Selvaraj/Enclave/issues/239) |
| Backup/export, restore, same-vault rule (ADR 0007) | 🟡 | Core; desktop UI pending [#236](https://github.com/Pranesh-Selvaraj/Enclave/issues/236) |
| Open Tauri v1.x vaults and fixtures | ✅ | Core contract/fixtures; no Tauri build needed |
| Multiple vaults (ADR 0012) | ⬜ | #222, [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |

## 6. Appearance, settings and platform

| Feature | Status | Where it lands / issue |
| --- | --- | --- |
| Auto/light/dark/Monochrome, accent presets, custom accent | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Background styles (Matte/Soft glow/Glass), corners, fonts, density, OLED, reduced motion | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Editor font size/page width | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Backup/restore settings, update channel, About | 🟡 | Core version/updater pieces; UI [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Native menus, file dialogs, drag-and-drop, notifications, global shortcuts | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Tray/wallpaper widget, quick capture | ⬜ | [#240](https://github.com/Pranesh-Selvaraj/Enclave/issues/240) |
| Signed installers (.deb/.AppImage, .msi/NSIS, .dmg) and notarisation | ⬜ | [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241) |
| Verified auto-update (SHA-256 + Ed25519) | 🟡 | Core verifier exists; desktop orchestration [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241) |
| Android share target/widgets/Quick Settings tile | ✅ (Android) | Native Android, not desktop |
| iOS | ⬜ | Roadmap: a native iOS app on the same core/contract; not part of desktop parity |

## 7. Security and compatibility invariants

| Invariant | Status |
| --- | --- |
| SQLCipher at rest, Argon2id, BIP39 recovery | ✅ Core |
| No cloud, no accounts, no telemetry | ✅ |
| No webview/browser engine in `main` | ✅ after retirement |
| No C ABI on desktop; direct `core-api` dependency | ✅ |
| Tauri-created vaults open and sync | ✅ Core fixtures; manual pass per release |
| Golden block fixture round-trips | ✅ `block-schema-v1.json` tests |
| Sync protocol v3 compatibility; v4 planned | ✅ Core / ⬜ v4 |
| Desktop release signatures/updates | ⬜ [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241) |

## Retired with the Tauri app

These are **not** to be ported:

- The SvelteKit/Tiptap UI, `backend.ts` IPC bridge, Tauri command surface, tray/quick-capture shell and `src-tauri/gen/android`.
- The Svelte packages (`@enclave/editor`, `@enclave/ui`, `@enclave/crypto`, `@enclave/sync-engine`) as shared UI code. Their capabilities move into the Rust engine/core or are retired.
- The Tauri release workflow and Tauri-specific signing scripts. The native release pipeline is [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241).
- The UI-audit harness and desktop screenshots of the Tauri app.

The historical code and binaries remain available at `v1.15.0` and earlier tags for reference and existing users.

## Definition of parity

Parity is green when every ✅/🟡 becomes ✅, every ⬜ has either shipped or a new ADR explicitly retiring it, and the acceptance gates in [ADR 0013](../decisions/0013-fully-rust-desktop.md) and [#233](https://github.com/Pranesh-Selvaraj/Enclave/issues/233) pass: canonical fixture round-trip, IME, AccessKit/screen reader, tables/tasks, 2,000-block latency, signed installers and verified updates. The retired Tauri app is the baseline, not a dependency.
