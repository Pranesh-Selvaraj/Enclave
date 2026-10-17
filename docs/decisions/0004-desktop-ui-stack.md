# 0004. Desktop UI stack

- **Status:** Proposed — decision gate: the Phase 0 editor spike (`docs/plans/dedicated-native-apps.md`)
- **Date:** 2026-10-17
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

Decision 0002 calls for a dedicated desktop application for Windows, macOS and Linux that shares no UI code with Android and links the Rust core. The hard problems, in order:

1. **The block editor.** Enclave's editor supports rich text, tables, images, code blocks, bookmarks, embeds, databases, a whiteboard and a graph. Tiptap/ProseMirror gives us all of that on the web; on desktop we must rebuild it. This is the dominant cost and the main risk.
2. **Security posture.** The UI renders untrusted content (Markdown, HTML in previews). Memory-safe UI code is preferable, though all key material and parsing that matters stays in Rust.
3. **Platform integration.** Menus, tray/menu bar, file dialogs, drag-and-drop, notifications, global shortcuts, auto-update, signing and notarisation.
4. **Compatibility.** Windows, macOS, Linux, on x86_64 and arm64, with packaging that users will actually install.

The team knows TypeScript/Svelte and Kotlin; C++ and C# would both be new.

## Candidates

| Candidate | Editor head start | Memory safety | Native integration | Footprint / startup | Accessibility | Velocity | Licence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **Qt 6** (Widgets + QML, C++) | **Best** — `QTextDocument`/`QTextEdit` provide a mature rich-text document model, tables, undo, clipboard, IME | C++ — needs care; core stays Rust | **Best** — mature desktop integration | Good | Good (Widgets) | Medium (new language, C++) | LGPLv3 / commercial; dynamic linking is fine for a MIT app |
| **Avalonia** (.NET 9, C#) | Weak — build rich text on `TextLayout`/`TextPresenter` | **Good** — managed runtime | Good | Good; self-contained or AOT | Improving, not mature | **High** (XAML/C#, strong tooling) | MIT |
| **Compose Desktop** (Kotlin/JVM) | Medium — Skia text stack, rich text is custom | **Good** — managed | Medium — expect/actual per platform | JVM bundle, slower cold start | Weak on desktop | High — same language as Android | Apache-2.0 |
| **Flutter Desktop** (Dart) | Medium — custom rendering, custom text editing | **Good** — Dart | Medium — plugins | Good | Weak on desktop | Medium — new language | BSD |
| **Per-OS native trio** (WinUI 3 + SwiftUI + GTK4) | Best per platform | Varies | **Best** | **Best** | **Best** | **Low** — 3× codebases | Mixed |
| **Rust-native** (Slint / iced) | Very weak — no rich document model | **Best** — Rust | Weak | **Best** | Weak | Low | Mixed |

## Decision

**Do not decide from the table; decide from a prototype.** The Phase 0 spike builds the same vertical slice — a block editor with a table, images, code blocks, undo/redo, IME and keyboard navigation, plus a Markdown preview and one database-style view — in the two leading candidates:

1. **Qt 6** (Widgets for the editor/document surface, QML for the surrounding chrome).
2. **Avalonia** (.NET 9).

The spike also verifies Rust interop (C ABI/UniFFI from the new UI), packaging, auto-update and code signing, and a screen-reader pass.

**Provisional leaning:** Qt 6. The editor is the product, and `QTextDocument` is the only mature cross-platform rich-text document model available without a webview; that head start plausibly saves more time than C++ costs, and the security-critical code stays in Rust regardless. **Avalonia is the alternative** if the team rejects C++ or the spike shows the .NET text stack is adequate; its managed runtime, MIT licence and faster development loop are real advantages.

**Not selected:** Compose Desktop and Flutter (not native, weaker desktop text/accessibility), the per-OS trio (three codebases), and Rust-native GUI toolkits (no viable rich-text editor; would consume the product's core surface).

The spike outcome is recorded by superseding this ADR with an `Accepted` one — including the measurements and what the prototype did and did not cover.

## Consequences

- Until the spike completes, desktop work stays on the current Tauri app; no native desktop code is committed beyond the prototype.
- If Qt is chosen: C++ enters the repo but is confined to presentation; all vault, crypto, parsing and network logic remains in Rust, and the FFI boundary is a C ABI.
- If Avalonia is chosen: the editor/document stack is a first-class subproject with its own budget.
- Either way the desktop app is one codebase for all three OSes; per-OS native shells are explicitly out of scope.

## Alternatives considered

Covered in the candidate table; the rejection reasons are summarised under Decision.

## Prior art / what this replaces

The current desktop app is Tauri + SvelteKit (0001). Its editor is Tiptap 3/ProseMirror, which cannot be reused here: a native desktop UI means a native document surface, so the editor is a rebuild either way. That shared cost is what makes the editor capability the correct deciding criterion.
