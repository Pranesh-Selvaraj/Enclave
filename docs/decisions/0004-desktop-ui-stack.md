# 0004. Desktop UI stack: Qt 6

- **Status:** Accepted — Qt 6 (2026-10-17). Verification gate: Phase 0 must confirm the Qt toolchain and the editor slice before implementation; if that fails, this ADR is superseded.
- **Date:** 2026-10-17
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** — (finalises the proposal this record was created with)
- **Superseded by:** —

## Context

Decision 0002 calls for a dedicated desktop application for Windows, macOS and Linux that shares no UI code with Android and links the Rust core. Decision 0005 fixes Android on Kotlin + Compose.

The project's mission — *secure, local-first, zero-knowledge knowledge base with P2P sync over local Wi-Fi* — drives the criteria, in order:

1. **Security surface.** Zero-knowledge means no cloud trust and the smallest possible audited client surface. A vault app must not carry a browser engine or a general-purpose JS runtime, and the number of UI implementations is itself an attack surface.
2. **The editor.** Enclave's editor (rich text, tables, images, code, embeds, databases, whiteboard, graph) is the product and its dominant cost. It must be built once per OS, on a mature text engine.
3. **Sync-contract integrity.** The vault, block schema and wire protocol live in one Rust core (0006); every extra FFI path is another place for the contract to drift.
4. **Enterprise sustainability.** One codebase, one pipeline, one support surface, one hiring profile.
5. **Platform fidelity.** Native feel and accessibility matter, but they are not mission properties.

## Candidates evaluated

| Candidate | Editor head start | Security surface | Native integration | Accessibility | Cost |
| --- | --- | --- | --- | --- | --- |
| **Qt 6** (Widgets + QML, C++) | **Best** — `QTextDocument`/`QTextEdit`: rich text, tables, undo, clipboard, IME | One UI codebase, no browser engine; C++ in presentation only | **Best** — mature desktop integration | Good (QAccessible: UIA / NSAccessibility / AT-SPI) | One codebase; C++ is new to the team |
| **Per-OS trio** (WinUI 3 + SwiftUI + GTK4) | Mature engine per OS | Best per OS, but three codebases to audit | **Best** | **Best** | **3× editor, UI, integration, QA, signing** |
| **Avalonia** (.NET) | Weak — no rich-text model; spike built one (~520 lines for basics; selection/IME/input rules remaining) | Managed runtime, one codebase | Good | Improving, not mature | One codebase + a large custom editor |
| **Compose Desktop / Flutter** | Custom text editing | Managed runtime, one codebase | Medium | Weak on desktop | Not native; weaker text |
| **Rust-native GUI** (Slint/iced) | No rich document model | One codebase | Weak | Weak | Would consume the product's core surface |
| **Webview/TypeScript** (Tauri, Electron, NodeGui) | Solved (ProseMirror), or immature bindings | **Worst** — browser engine, and/or a Node runtime | Medium | Medium | Reintroduces exactly what 0002 removes |

## Decision

**Build the dedicated desktop app on Qt 6.**

- **Qt Widgets for the document/editor surface** — `QTextDocument`/`QTextEdit` provide the rich-text, table, undo, clipboard and IME primitives the block layer is built on. QML is used for modern chrome where it clearly helps; the editor is not reimplemented in QML.
- **C++ is confined to presentation.** All vault, crypto, document parsing and validation of untrusted content (Markdown, HTML, tag/meta JSON), networking and sync stay in the Rust core; the UI receives typed models only. This directly answers the C++ memory-safety objection: the untrusted-input boundary is Rust.
- **One FFI path: a C ABI** from the Rust core (`core-api`) for the desktop app. Android keeps its UniFFI/Kotlin path (0005).
- **One desktop codebase** for Windows, macOS and Linux. The per-OS trio is not adopted.
- **macOS gets a deliberate polish budget** (menu bar, services, trackpad, vibrancy). If macOS ever becomes a co-primary market where AppKit fidelity is a differentiator, a SwiftUI macOS app (2+1) is the escalation path — not a rewrite of all three.
- **Android remains Kotlin + Jetpack Compose** (0005); the shared asset is the Rust core, not the UI.

## Consequences

- Total language count stays small: **Kotlin, C++, Rust**, plus the existing web app during the transition.
- One pipeline, one editor implementation, one accessibility effort, one place where a UI bug is fixed.
- C++ is accepted in the UI layer. The mitigations are architectural, not aspirational: no untrusted parsing in C++, no key material in C++, a small typed C ABI, and the existing core tests as the contract gate.
- Licensing: the app is MIT, so Qt under LGPLv3 with dynamic linking is compatible; a commercial licence is an option later for static linking or support, not a requirement.
- Qt is **not yet verified on the development machine** (no system Qt, no passwordless sudo). Phase 0 must confirm the toolchain, the editor slice, packaging/signing/auto-update and a screen-reader pass before implementation begins. Avalonia's measurements are retained as the managed fallback if Qt cannot be made to work.

## Alternatives considered

- **Per-OS native trio** — best fidelity, but it multiplies the dominant cost (the editor) by three and triples the audited surface, the sync-contract bindings, the pipelines and the support burden. Mission-wise it trades the thing the mission protects (small, auditable, one core) for the thing it does not require (maximum platform feel). Kept as the escalation path for when platform teams or a compliance mandate exist.
- **Avalonia** — the managed, MIT fallback. Spiked: the editing surface was built and works (marks, caret, typing, undo, ProseMirror round-trip; evidence under `~/.enclave-dev/phase0-native-apps/`), but it required building a text engine the toolkit does not provide, and the block layer, selection, IME and input rules are still ahead. More total work than Qt for a weaker text stack.
- **Compose Desktop / Flutter** — not native and weaker desktop text/accessibility; would not solve the editor problem either.
- **Rust-native GUI toolkits** — no viable rich-text editor; adopting one would consume the product's core surface in UI plumbing.
- **Webview and TypeScript runtimes (Tauri, Electron, Wails, NodeGui, React Native)** — rejected on the mission: Tauri/Wails/Electron reintroduce a browser engine, Electron and NodeGui add a Node/V8 runtime and a supply chain in a vault app, and React Native has no first-class Linux target. TypeScript remains appropriate for build tooling, tests and a possible future web client — not for the desktop shell.

## Prior art / what this replaces

The current desktop app is Tauri + SvelteKit (0001). Its editor is Tiptap 3/ProseMirror, which cannot be reused once the webview goes: a native document surface means a native editor, so the editor is a rebuild under every option. That shared cost is why editor capability and cost dominate this decision — and why the option that provides a mature document engine *once* wins.
