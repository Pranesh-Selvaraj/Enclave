# 0002. Dedicated native apps with a shared Rust core

- **Status:** Accepted — direction approved 2026-10-17; execution gated by the Phase 0 spike (see `docs/plans/dedicated-native-apps.md`)
- **Date:** 2026-10-17
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** 0001 (UI layer only; the Rust core from 0001 is kept)
- **Superseded by:** —

## Context

Enclave's mission is a secure, local-first, zero-knowledge knowledge base with peer-to-peer sync over the local network and an offline AI assistant. Nothing in that mission requires a webview — quite the opposite: a browser engine is a large, continuously patched attack surface, and the platform capabilities the product needs (background sync, home-screen widgets, file access, keystores, native editors) are all native APIs. The current shared-web-UI architecture (0001) reached its limits: mobile background execution and widgets are bridged, desktop feels like a website, and the whole UI carries a browser-engine dependency forever.

At the same time, the Rust core is a genuine asset: `core-db`, `core-network` and `core-api` already contain the vault, the crypto and the sync protocol, and `core-api` already exposes them to a native Android shell through UniFFI. The expensive, security-critical part is done and shared.

## Decision

- Build **two dedicated applications that share no UI code**:
  - `apps/desktop` — a native desktop application for Windows, macOS and Linux (stack decided in 0004).
  - `apps/android` — a native Android application in Kotlin + Jetpack Compose (0005).
- Both link the **same versioned Rust core** through a stable FFI surface (`core-api`); the core remains the sole owner of cryptography, the vault, the block/document schema and the sync protocol.
- There is **no shared UI layer**: no shared components, design system, navigation or editor implementation. Platform behaviour is native on each platform.
- The current Tauri application keeps shipping until each native app reaches parity; this is a transition, not a flag day.
- The contract between core and apps is versioned and tested (0006).

## Consequences

**What becomes possible**

- No browser engine in a vault application: a materially smaller attack surface and no webview CVE treadmill.
- Native performance, memory and battery behaviour, and native platform integration (widgets, services, share targets, keystores, menus, file pickers).
- Each platform can do what is idiomatic there instead of what the lowest common denominator allows.
- The Rust core is reused unchanged, so the vault format and the sync protocol stay identical across apps — data compatibility is preserved by construction.

**What it costs**

- The UI is written twice (desktop and Android), and the editor is rebuilt natively on each platform. This is the largest single cost of the decision.
- A real contract must be designed and enforced: FFI surface, vault schema, block schema, sync protocol. Drift between apps breaks sync and security (0006).
- Two release pipelines, two platform-integration efforts, and a parity period in which three apps (Tauri, native desktop, native Android) coexist.
- Some current capabilities (whiteboard, graph, database views) are substantial native builds, not ports.

**Non-goals**

- A single cross-platform UI framework. That is precisely what we are moving away from.
- Cloud sync, accounts, or any server-side component.
- iOS for now; the contract must not preclude it later.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Keep Tauri everywhere | Keeps the browser engine, bridged native features and non-native feel on all platforms. Rejected as the target; retained as the shipping fallback during migration. |
| Tauri desktop + native Android (hybrid) | A defensible fallback: fixes the platform where the webview hurts most while keeping desktop velocity. Rejected as the end state because "dedicated desktop" is an explicit goal. |
| One Compose Multiplatform UI for both | Not platform-native; shares exactly the UI code we want to keep separate; ties both apps to one toolkit and one release cadence. |
| Flutter for both | Same objections as Compose Multiplatform, plus a new language and weaker desktop text input. |
| Per-OS native desktop trio (WinUI 3 + SwiftUI + GTK4) | Three desktop codebases is beyond the team; the cross-platform desktop stack in 0004 is the compromise. |

## Prior art / what this replaces

See 0001. In short: v1.9.0 proved a native Compose Android shell could run on the shared Rust core, but it was abandoned in v1.10.0 for feature parity with the shared web UI. This decision revisits that trade in the other direction — keeping the shared core, dropping the shared UI — now that `core-api` and its UniFFI bindings exist and the webview costs are well understood.
