# 0005. Android UI stack: Kotlin + Jetpack Compose

- **Status:** Accepted
- **Date:** 2026-10-17
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

Decision 0002 calls for a dedicated native Android app that shares no UI code with desktop and links the Rust core. Android is the platform where the webview hurts most: background sync, battery, widgets, share targets and file access are all native APIs that the Tauri bridge only approximates. The project has prior art here — the v1.9.0 "Keep-style" shell was native Compose — and the `core-api` UniFFI bindings already exist and are proven on Android.

The app targets Android 7.0+ (`minSdk 24`, `targetSdk 36`) to cover the widest useful device range while keeping modern platform APIs.

## Decision

- **Kotlin + Jetpack Compose (Material 3)** for the entire Android UI, using the current Compose BOM, AndroidX, and Kotlin coroutines. No XML view layouts, no webview.
- **Rust core via `core-api`**: UniFFI-generated Kotlin bindings plus the JNI bridge for the parts that must cross into Android services. One core, one vault, one sync loop — the same code the desktop app links.
- **Native integrations built as platform code, not bridges**: a foreground sync service with a live notification, Jetpack Glance home-screen widgets (notes list, pinned note, quick capture), share target, launcher shortcuts, Quick Settings tile, Keystore-backed widget cache, SAF for import/export/backup, and real window-inset handling.
- **Architecture**: unidirectional data flow, a repository layer over the core bindings, immutable UI state; ViewModels per screen. Android-specific dependencies stay out of the core.
- **Distribution**: signed APK/AAB, monotonic `versionCode`, verified in-place updates (Ed25519), Play-compatible artefacts.

## Consequences

**Wins**

- First-class background execution: sync survives backgrounding and restarts, with a real foreground service.
- Native widgets and share/shortcut integration, not a bridge.
- Better battery and memory than a webview app, and a smaller attack surface on a device that holds the vault.
- A proven path: UniFFI → Kotlin already works, and the team has shipped Compose before.

**Costs**

- The editor and document surfaces are rebuilt in Compose; Compose text input/IME and rich text require careful work.
- Two Android UI implementations exist during the transition (Tauri web UI and the native app) until parity.
- Compose's text/accessibility tooling is good but not equivalent to the web's; accessibility needs testing with TalkBack.
- Android fragmentation: device coverage testing (especially widgets, SAF and mDNS/Bonjour alternatives) is ongoing.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Compose Multiplatform UI shared with desktop | Contradicts 0002 (no shared UI) and the "native on each platform" goal. |
| XML views + Fragments | Legacy approach; slower to build and less expressive than Compose. |
| Flutter (one Dart UI for Android + desktop) | New language, custom-rendered UI (not native), weaker platform integration, and it would not solve the desktop editor problem either. |
| Keep Tauri mobile | Bridged background/widgets/file access, webview battery and attack surface; the status quo this decision replaces. |
| React Native | Adds a JavaScript bridge to a product that has no need for one; weaker background/widget integration than native. |

## Prior art / what this replaces

The v1.9.0 native Compose shell was replaced by the shared Tauri UI in v1.10.0 because it lagged the desktop feature set and duplicated UI work. That experience is preserved in 0001: it failed on *parity within a shared-UI strategy*, not on Compose's suitability. This decision reuses the same core and the same UI toolkit, but commits to full parity as a dedicated app rather than dropping the native path.
