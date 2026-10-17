# 0001. Stack v1: Rust core + Tauri v2 + SvelteKit

- **Status:** Accepted — in force until 0002 lands
- **Date:** 2026-07-14
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** (partially) 0002 for the UI layer; the Rust core is kept

## Context

Build a secure, local-first, zero-knowledge knowledge base with peer-to-peer sync over the local network and an offline AI assistant. The product must run on Windows, macOS and Linux first, with Android later. The editor is the core experience. The team is very small, so shipping velocity and a single codebase per layer matter, but a vault application cannot compromise on the safety of its cryptography or its offline guarantees.

The repository is a SvelteKit + Tauri application (originally prototyped as "Aethernote", renamed Enclave).

## Decision

- **Core**: a Rust workspace owning everything security- and data-critical — `core-db` (SQLCipher, FTS5, `sqlite-vec`, vault crypto), `core-network` (mDNS discovery, WebSocket transport, authenticated and AEAD-encrypted sync), `core-api` (the shell-agnostic API and, later, the UniFFI surface).
- **Shells**: Tauri v2 on desktop (WebKitGTK / WebView2 / WKWebView) and on Android (system WebView).
- **UI**: one SvelteKit + Svelte 5 frontend, static adapter, shared across desktop and Android.
- **Editor**: Tiptap 3 (ProseMirror) with a Svelte 5 wrapper.
- **Storage**: SQLite + SQLCipher (AES-256-CBC + HMAC-SHA512), Argon2id key derivation, BIP39 12-word seed phrase.
- **Sync**: mDNS discovery + WebSocket, document-level last-write-wins, mutual-auth handshake (HMAC-SHA256) and XChaCha20-Poly1305 frames.
- **Android**: initially Tauri mobile; `core-api` gained UniFFI/Kotlin bindings in v1.10.0 so a native shell could consume the same core.

## Consequences

**What this bought us**

- One UI codebase across desktop and Android; features land once.
- A single, auditable Rust core: crypto, vault format and sync protocol exist in exactly one implementation.
- Small installers and less memory than an Electron app; the system webview is reused rather than bundled.
- Fast iteration on the editor (the web ecosystem has the best rich-text tooling) and a straightforward path to Windows/macOS/Linux bundles.
- Cross-device sync works between desktop and Android today.

**What it costs**

- The UI runs inside a browser engine: a large attack surface to keep patched in a vault app, plus webview memory/CPU and startup cost.
- Native capabilities are bridged rather than native: widgets, background sync, share targets and file access need plugins, JNI bridges and Kotlin services.
- Tauri IPC serializes command payloads as JSON, which is fine at small scale but not free for bulk document and block traffic.
- Platform feel is "one web app on every OS": macOS/Windows/Linux conventions, accessibility and desktop input behaviour are approximated rather than native.
- The editor is inseparable from the webview; there is no path to a native document surface without leaving the stack.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Electron | Bundles a full Chromium + Node runtime per app; much larger memory/disk footprint and a bigger attack surface than reusing the system webview. |
| Native per-OS desktop (WinUI 3 + SwiftUI + GTK4) | Three UI codebases for a small team; incompatible feature timelines. |
| Flutter / Compose Multiplatform UI | Not truly platform-native either, and it would have discarded the ProseMirror editor investment. |
| Pure web app / PWA | Cannot do LAN mDNS discovery, background sync, an encrypted on-disk vault, or a hardware-backed keystore. |
| Rust-native GUI (egui/iced) | Immature for a rich document editor and accessibility; not viable for the editor this product needs. |

## Prior art / what this replaces

The first Android attempt (v1.9.0) was a native Jetpack Compose shell ("Keep-style") with a Rust core behind it. It was replaced in v1.10.0 by the full Tauri UI because the native shell could not reach feature parity with the desktop editor, and maintaining a second UI implementation for every feature was slower than sharing the web UI. The extraction of `core-api` and its UniFFI bindings during that work proved that a native shell can consume the same trusted core — which is the foundation for decision 0002.

## Why this is being revisited

The webview-based UI is the weakest part of an otherwise native, security-first product: browser-engine attack surface, mobile battery and background limits, bridged native features, and no native desktop feel. Decision 0002 moves the UI off the webview while keeping the Rust core that this ADR established.
