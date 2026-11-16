# 0013. Desktop app: fully Rust, with an in-house headless editor engine

- **Status:** Accepted — fully Rust (2026-11-16). Execution gated by the editor-engine slice described below; if that gate cannot be met, a new ADR must choose the fallback. No silent fallback to Qt or Tauri.
- **Date:** 2026-11-16
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** 0004
- **Superseded by:** —

> **Update 2026-11-16:** ADR 0014 retired the Tauri desktop from `main`; this record’s “Tauri remains the shipping desktop” transition clause is historical. The fully Rust target is unchanged.

## Context

Android and the desktop are both products. Decision 0004 chose Qt 6 for the desktop because `QTextDocument` provides a mature rich-text engine, and because a vault app should not ship a browser engine. That decision accepted C++ in the presentation layer, LGPLv3 dynamic linking, and a C ABI between the Rust core and the Qt UI.

Two things changed the trade-off:

1. **The product now wants to own its editor.** The block editor is the product's dominant surface. Owning a headless engine means owning the document model, the ProseMirror-compatible contract, custom blocks, accessibility and the long-term roadmap — and it can be open-sourced and eventually reused by other apps without exposing Enclave's vault or sync layers.
2. **A fully Rust path is now credible.** Rust has the necessary building blocks: `winit`/`wgpu`/`vello` for windows and painting, Parley and CosmicText for shaping and line breaking, AccessKit for accessibility, and a young but real rich-text ecosystem (`canvist` is the closest existing engine). There is still no mature, production-grade Rust rich-text editor, so the editor is the gate — not the shell, the core, the AI service or sync.

The team's strength is Rust. C++ is new to the team, and the project's mission — secure, local-first, zero-knowledge, no cloud — rewards a small, auditable language surface. Android is Kotlin + Jetpack Compose (0005) and stays that way; the core is the shared implementation of vault, crypto, storage, search, sync and updates (0006).

Tauri remains the shipping desktop product during the transition. This decision is not a flag day.

## Decision

**Build the desktop application in Rust, with no C++ and no C ABI.**

- **Presentation:** a Rust UI shell. The initial shell candidate is egui/eframe because it gives custom painting for the editor, whiteboard and graph, has IME events and AccessKit integration, and is fast to build. The shell is replaceable; the editor engine and core are not tied to it.
- **Editor:** an in-house **headless block editor engine** plus a desktop view adapter:
  - `editor-model`: schema, block tree, inline runs, marks, attributes, stable IDs, ProseMirror/TipTap JSON import/export, validation and limits.
  - `editor-engine`: transactions/steps, selection, history/undo, commands, input rules, keymaps and clipboard semantics.
  - `editor-layout`: block and inline layout (Parley or CosmicText), tables, images, code blocks, caret/selection geometry, hit testing and a renderer-agnostic display list.
  - `editor-view-*`: platform adapters. The first is the desktop view; the engine stays UI-framework-independent.
  - Accessibility uses AccessKit; the editor exposes text runs, selection, roles and actions. IME preedit, commit and candidate placement are first-class.
  - The engine is headless: the AI agent, tests and import/export use it without a window.
- **Core boundary:** `apps/desktop` depends on `core-api` as a normal Rust crate. Shared types, tokio events and direct calls; no C ABI, no JSON-over-FFI shell boundary, no duplicate document model.
- **Android:** unchanged. Kotlin + Jetpack Compose (0005) consumes `core-api` through UniFFI. The Rust core remains the only implementation of crypto, storage, the block schema and sync.
- **AI:** a separate Rust AI service/crate, not part of the core and not part of the editor:
  - local embeddings and hybrid retrieval by default;
  - local model inference where the platform can support it;
  - remote providers strictly opt-in, with the exact context shown before a request;
  - the app executes agent tools after policy checks and approvals; the model never touches the vault, filesystem, shell or network directly;
  - the AI service never receives the vault key.
- **Sync:** protocol v4 adds block-level deltas, compression, attachments, settings/templates, resumable transfers and a conflict policy, with v3 compatibility. Android gets the benefit first; the Rust desktop consumes the same core implementation.
- **Transition:** Tauri remains the shipping desktop until the Rust desktop passes a published parity checklist. The Rust app ships as a beta alongside it.
- **Open source:** the headless editor engine is designed to be extracted into its own MIT/Apache-2.0 project once the desktop editor slice proves it. Enclave is the first consumer; Enclave-specific blocks (database, whiteboard, embeds) stay in the app or as engine extensions. A separate brand and governance are part of that extraction, not of this ADR.

## Consequences

**Easier / stronger**

- One language for the desktop product and the core; no C++/Qt/LGPL and no C ABI to audit or maintain.
- Direct use of the core's types, async runtime and sync events; fewer moving parts for P2P offline sync.
- Memory safety across the presentation and editor layers, with untrusted document content parsed in Rust.
- The editor engine is an owned asset: it can be tailored to Enclave's block model, shared with future apps and open-sourced.
- Android and desktop still share the only security-critical layer — the Rust core — and the versioned contract.
- A smaller desktop attack surface: no browser engine, no JS runtime, no Qt plugin/image parser stack.

**Harder / accepted costs**

- **The editor is a first-party subproject.** Text layout, IME, accessibility, tables, cross-block selection, undo and input rules are the long tail. The engine must be gated by a real slice before the desktop UI work grows around it.
- **The Rust UI ecosystem is young.** egui/iced/GPUI are pre-1.0, APIs churn, and database views, whiteboards and graphs are custom widgets rather than `QTableView`/`QGraphicsScene`.
- **Desktop parity takes longer.** The Tauri app remains the reference product until the Rust app is green.
- **The editor engine must be maintained forever**, including fuzzing, dependency review, accessibility and platform input-method testing. Open-sourcing adds docs, API stability, governance and community support.
- **No C ABI is a deliberate removal of a contract surface.** `docs/CONTRACT.md` must describe the desktop boundary as a Rust crate dependency, and the compatibility matrix must stop promising a C ABI.
- **Android sharing is later, not automatic.** A headless engine can be reused by Android through UniFFI/JNI, but that is a separate decision; the Compose editor stays for now.

## Verification gate

The decision is implemented only if the editor slice meets all of these on the desktop:

1. Loads `core/core-db/tests/fixtures/block-schema-v1.json`, edits it (text, marks, task toggle, table cell), exports ProseMirror JSON, and the core accepts it and returns it semantically unchanged.
2. Handles IME composition (accented Latin, CJK, combining sequences) without corrupting marks or the document.
3. Exposes an AccessKit tree that a real screen reader can read and edit; a keyboard-only pass works.
4. Edits tables and task lists from the keyboard.
5. Types responsively on a 2,000-block document (measured, not assumed).
6. Supports the custom-block extension point needed for databases, whiteboards and embeds without forking the engine.
7. Ships a Linux package and proves the update verification path with the core's Ed25519 verifier.

If the gate fails, the next ADR must choose explicitly between Qt (0004's path), a young Rust engine such as canvist, or a narrower desktop scope. The desktop code must not silently fall back.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| **Qt 6 + C++ presentation (0004)** | Mature editor, accessibility, IME and data grids, but C++ in the product, LGPLv3, a C ABI, and a second UI language. It does not satisfy the fully Rust direction. Kept as the documented fallback if the Rust editor gate cannot be met. |
| **Tauri/webview desktop** | Fastest parity and the current shipping product, but ships a browser engine and JS runtime in a vault app. Retained only as the transition product. |
| **canvist as the editor** | The closest fully Rust engine: ProseMirror-compatible JSON, tables, tasks, images, IME, AccessKit. Still 0.2.x, single-author and unproven; it is a spike candidate and a reference, not the decision. |
| **Compose Multiplatform desktop** | Shares UI code with Android, which is explicitly not the goal; not native on desktop and not the Rust stack. |
| **Per-OS native trio** | Three desktop codebases and three editors; beyond the team. |
| **Rust UI + Qt editor hybrid** | Two toolkits, two event loops, an FFI boundary and two packaging stories; worse than either pure option. |

## Prior art / what this replaces

The current desktop is Tauri + SvelteKit (0001), with Tiptap/ProseMirror as the editor. The native Android app already proves that `core-api` can be consumed without a webview and that Kotlin/Compose can own a native UI. Decision 0004 chose Qt for the desktop but never passed its Phase 0 gate: Qt was accepted on the strength of `QTextDocument`, not verified in this repository, and the Android-first work pushed the desktop to the background. This ADR replaces that stack choice deliberately, with the editor engine as the first milestone and the core/AI/sync layers unchanged.
