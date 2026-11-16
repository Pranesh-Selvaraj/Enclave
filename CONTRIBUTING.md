# Contributing to Enclave

Enclave is a native-only repository: a shared Rust core, a Rust desktop app and a Kotlin/Compose Android app. The retired Tauri/Svelte app is preserved in Git history and release tags; it is not part of `main` (ADR 0014).

## Prerequisites

- **Rust** 1.95+ — [rustup.rs](https://rustup.rs). The core builds on older stable; the desktop uses eframe/egui 0.37.
- **Linux desktop**: `libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libssl-dev`
- **Android**: JDK 21, Android SDK platform 36, build-tools 36.0.0, NDK 29.0.13846066, `cargo-ndk`; rustup targets `aarch64-linux-android` (+ `x86_64-linux-android` for emulators).
- **Node.js** 24+ only for the static homepage; there is no npm workspace or frontend build.
- **macOS/Windows**: the Rust desktop builds with the platform toolchains; packaging/signing is tracked in [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241).

## Validate

```bash
# Core
source ~/.enclave-dev/env.sh
cargo test --manifest-path core/Cargo.toml

# Rust desktop
cargo test --locked --manifest-path apps/desktop/Cargo.toml
cargo clippy --locked --manifest-path apps/desktop/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path apps/desktop/Cargo.toml --check
cargo build --locked --manifest-path apps/desktop/Cargo.toml

# Native Android (core + bindings + JVM tests + APK)
source ~/.enclave-dev/android-env.sh
./apps/android/scripts/build-core.sh
cd apps/android && ./gradlew assembleDebug testDebugUnitTest
cd -

# Public homepage
node scripts/build-site.mjs

# Release pre-flight (versions/versionCode before tagging)
# Until the native release train lands (#241) this reports the historical
# Tauri v1.15.0 versionCode as the last release; tag releases are paused.
bash scripts/release-check.sh
```

The native desktop's window smoke needs a display; see `apps/desktop/README.md` for WSL2/Xvfb notes.

## Project structure

```
enclave/
├── core/                          # The only shared code; its own Cargo workspace
│   ├── core-db/                   # SQLCipher, FTS5, sqlite-vec, sync merge
│   ├── core-network/              # mDNS + WebSocket transport and crypto
│   └── core-api/                  # Shell-agnostic API + UniFFI bindings
├── apps/
│   ├── desktop/                   # Fully Rust desktop: egui shell + headless editor engine
│   └── android/                   # Native Android: Kotlin + Jetpack Compose
├── docs/                          # Contract, ADRs, plans, status, history, brand
│   └── screenshots/               # Native Android screenshots
├── site/                          # Static homepage (no framework, no analytics)
├── scripts/                       # Site build, native Android signing, release checks
└── .github/workflows/             # Desktop, native Android and homepage CI
```

## Making changes

1. Open or find an issue. Every branch, commit, PR and release note references it.
2. Branch from `main`; never commit or release from `main`.
3. Keep the change focused. Add a test for non-trivial behaviour. The editor engine, contract fixtures and vault lifecycle are all test-gated.
4. Run the validation above. A change that touches `core/` must not break either native app.
5. Open a PR with the linked issue and the validation evidence.

## Compatibility rules

- The Rust core is the only implementation of crypto, storage, block JSON, search, sync and updates.
- `docs/CONTRACT.md` and its golden fixtures are the compatibility gate. A contract change updates the version, fixtures and docs in the same change.
- A Tauri-created vault must still open in both native apps; the retired Tauri app is a historical peer, not a build target.
- The desktop depends on `core-api` as a Rust crate. Android uses UniFFI. Do not add a C ABI or a webview.
- No cloud, accounts or telemetry. AI is opt-in; local-first by default.

## Releases

Native release pipelines are tracked in [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241). Until they land, tag releases pause; existing Tauri releases remain installable. `scripts/release-check.sh` validates the desktop version, Android `version.properties` and the CHANGELOG before tagging.

Release notes follow the standing rule: a detailed, self-contained prose description in `CHANGELOG.md` and the same text on the forge release page. Never substitute generated PR lists or commit logs for the notes.

## Security

- No `unsafe` without justification and review.
- Fuzz or property-test parsers and transforms where applicable.
- Never log vault contents, keys, recovery phrases or AI prompts.
- Report vulnerabilities through [SECURITY.md](SECURITY.md), not a public issue.
