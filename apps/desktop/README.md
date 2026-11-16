# Enclave desktop — fully Rust

This is the native desktop app. It is a Rust binary that depends on the shared
`core-api` **as a Rust crate** — there is no C ABI, no C++ and no webview. Android
stays Kotlin + Jetpack Compose; the Rust core is the only shared implementation
of vault, crypto, storage, search and sync.

**Status:** foundation. The app opens a vault, lists notes and proves the direct
core boundary. The next gate is the in-house headless editor engine. Tauri
remains the shipping desktop until this app passes a parity checklist.

Tracking issue: [#233](https://github.com/Pranesh-Selvaraj/Enclave/issues/233).
Architecture: [ADR 0013](../../docs/decisions/0013-fully-rust-desktop.md).

## Run

```bash
# The core needs its usual SQLCipher/OpenSSL build prerequisites.
cargo run --manifest-path apps/desktop/Cargo.toml
```

The app uses the same app-data directory as the Tauri build
(`com.enclave.app`), so it can open an existing vault during the transition.
Do not run both apps against the same vault at the same time. Override the
directory for development or tests:

```bash
ENCLAVE_APP_DIR=/tmp/enclave-desktop-dev cargo run --manifest-path apps/desktop/Cargo.toml
```

On WSL2/WSLg, the default wgpu adapter may fail to initialise. Force the
OpenGL/software path (or Vulkan if your driver supports it):

```bash
WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 cargo run --manifest-path apps/desktop/Cargo.toml
# or
WGPU_BACKEND=vulkan cargo run --manifest-path apps/desktop/Cargo.toml
```

Under a bare X server, run it inside `xvfb-run` and unset `WAYLAND_DISPLAY`; the
app has no display-independent GUI test yet, so the window smoke is manual.

## Test

```bash
cargo test --manifest-path apps/desktop/Cargo.toml
cargo clippy --manifest-path apps/desktop/Cargo.toml --all-targets -- -D warnings
```

The tests cover the direct core path (create vault → create document → list) and
the first editor slice: the canonical ProseMirror fixture
(`core/core-db/tests/fixtures/block-schema-v1.json`) parses, validates and
round-trips semantically, and unknown node types/attributes survive.

## What is here

- `src/main.rs` — eframe entry point (initial shell candidate; replaceable).
- `src/app.rs` — vault gate, notes list, placeholder reading pane.
- `src/core_worker.rs` — one `EnclaveCore` on a worker thread; typed events to
  the UI. Vault creation/unlock (Argon2id) stays off the UI thread.
- `src/editor.rs` — headless ProseMirror model (first editor-engine slice).

## Next

1. Headless editor engine: transactions, selection, history, commands, layout.
2. Desktop editor view: IME, AccessKit, tables/tasks, fixture round-trip.
3. Daily-driver shell: attachments, search, folders, settings, sync.
4. AI service: local embeddings + retrieval, opt-in remote, read-only agent.
5. Sync v4: deltas, compression, attachments, settings/templates.

The editor engine is designed to become a separate MIT/Apache-2.0 project once
the desktop slice proves it; Enclave remains the first consumer.
