# Changelog

All notable changes to Enclave are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versioning follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.1.0] — 2026-08-10

### Added

- **In-DB vector search** — `sqlite-vec` ANN index (`vec0` virtual tables, per-dimension, cosine
  metric) inside the same encrypted SQLCipher file as the rest of the vault. No separate storage
  engine, no second encryption boundary.
- **Ranked RAG retrieval in Rust** — `search_embeddings` returns exact-cosine re-ranked top-k
  (4× recall buffer over the ANN candidates) with an exact-scan fallback for dimensions without an
  index yet. The frontend no longer ships every vector over IPC.
- **Fully-offline embeddings** — built-in ONNX embedding model (fastembed, `all-MiniLM-L6-v2`,
  384 dims). Downloads once (~25 MB) into the app data dir, then RAG retrieval works with zero
  external services; chat still needs an endpoint.
- **Generic AI client** — `ollama.ts` → `ai.ts`: any OpenAI-compatible endpoint. Local servers
  (Ollama, llama.cpp, LM Studio, vLLM) or frontier APIs with a Bearer API key.
- **Vault-encrypted AI settings** — endpoint, model and API key now live in the vault's `settings`
  table (SQLCipher), not localStorage. Legacy localStorage settings migrate once on first load.
- **Offline-embeddings toggle** in Settings — with it on, retrieval content never leaves the device.
- **Whiteboard opt-in per page** — pages without a whiteboard block show a single "Whiteboard"
  button instead of a permanent Paper|Whiteboard toggle; the block is created on first edit.
- **CODEOWNERS** + hardened branch protection (see below).
- **CHANGELOG.md** — this file.

### Fixed

- **Editor crash loop** (`effect_update_depth_exceeded`) — the TipTap editor now mounts from a
  `use:action` instead of a reactive `$effect` that read and wrote the same state, which re-created
  the Editor endlessly (in prod it crashed, in dev it remounted ~50×/s).
- **Drag-handle grip read as "::"** — the grip was the Braille glyph `⠿` (U+28FF); at 11px it
  renders as two vertical dot columns indistinguishable from `::` and followed the mouse beside
  every block while typing. Replaced with an inline 3×3 dot SVG.
- **Tauri permission denials** — `event.listen` (sync events) and `window.confirm` (delete/archive
  confirmations) were denied; the app shipped with no capability file. Added
  `core:default` + `dialog:default` + `dialog:allow-confirm`.
- **Deep links 404** — the dynamic `[id]` route inherited prerendering and baked a "Not Found" error
  into the static fallback; the route is now client-only.
- **IndexedDB `get()` contract** — (removed with the web platform, see below) the adapter returned
  `{id, value}` where the KV contract expects the bare value, breaking every record read on web
  builds.
- **Duplicate `codeBlock` extension** — StarterKit already provides one; the custom node view now
  configures `codeBlock: false` instead of registering a second.

### Removed

- **Web platform** — per project direction (desktop + mobile only): the IndexedDB/WebCrypto storage
  backend (`webStore.ts`) and its tests, browser fallbacks in backend/importExport/Whiteboard/
  capture, stale `apps/mobile` + `apps/web` build artifacts, and the `preview` scripts.

### Changed

- CSP now allows `https:` in `connect-src` for frontier AI endpoints (localhost remains the default).
- AI settings are stored in the encrypted vault instead of localStorage (API keys never sit in
  plaintext).
- `get_embeddings` (all vectors over IPC) replaced by `search_embeddings(query, limit)`.

## [1.0.0] — 2026-08-06

Secure, local-first, zero-knowledge knowledge base with P2P sync over local Wi-Fi. First stable
release, consolidating four stage branches plus housekeeping.

### Added

- **RAG assistants** — vault-wide retrieval-augmented generation: document embeddings, an
  OpenAI-compatible LLM client (chat + embeddings), context injection with rebuild-on-save, and a
  sources UI.
- **Web platform support** — static SPA with IndexedDB + WebCrypto storage routed through the
  invoke bridge (removed in 1.1.0).
- **Sync hardening** — dropped peers are redialed; health readout in the network panel.
- **Hand-rolled HTML→Markdown serializer** — dropped the `turndown` dependency.
- **Toolchain cleanup** — shared `tsconfig.base.json`; standard hex helper in the crypto self-check.

### Fixed

- #21 — redial dropped peer connections.
- #4 — replace turndown with the hand-rolled HTML→Markdown serializer.
- #2 — drop custom `bytesToHex` from the crypto self-check.

## [0.4.0] — 2026-08-01

Database v2, edgeless, LAN sync, comments, local AI.

## [0.3.0] — 2026-07-15

Initial app release.

[Unreleased]: https://github.com/Pranesh-Selvaraj/Enclave/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/Pranesh-Selvaraj/Enclave/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Pranesh-Selvaraj/Enclave/compare/v0.4.0...v1.0.0
[0.4.0]: https://github.com/Pranesh-Selvaraj/Enclave/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Pranesh-Selvaraj/Enclave/releases/tag/v0.3.0
