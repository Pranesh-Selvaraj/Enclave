# Enclave desktop — fully Rust

This is the native desktop app. It is a Rust binary that depends on the shared
`core-api` **as a Rust crate** — there is no C ABI, no C++ and no webview. Android
stays Kotlin + Jetpack Compose; the Rust core is the only shared implementation
of vault, crypto, storage, search and sync.

**Status:** branded shell + editable text engine. The app opens with the
official Enclave lockup, a centred welcome/login/register gate, the Inter
typeface and the Android palette; it then lists notes, loads a note through the
core and edits it with autosave. The in-house
headless engine supports text insertion/deletion, block split/merge, headings,
marks, task toggles, toolbar list creation, list/task-item Enter, Tab/Shift+Tab
nesting, undo/redo and save through the core.
The next gate is IME, AccessKit, table navigation and a screen-reader pass. The retired Tauri app is
the feature baseline; [`docs/plans/desktop-feature-parity.md`](../../docs/plans/desktop-feature-parity.md)
tracks the transfer. It is not the shipping desktop anymore (ADR 0014).

Tracking issue: [#233](https://github.com/Pranesh-Selvaraj/Enclave/issues/233).
Architecture: [ADR 0013](../../docs/decisions/0013-fully-rust-desktop.md).
User guide: [`docs/desktop.md`](../../docs/desktop.md).

## Run

```bash
# The core needs its usual SQLCipher/OpenSSL build prerequisites.
cargo run --manifest-path apps/desktop/Cargo.toml
```

The app uses the same app-data directory as the retired Tauri build
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

There are 31 tests: the direct core path (create vault → create document → list),
the worker lifecycle (create/unlock/list/lock/save/load/rename), the headless editor
engine and a headless render of the canonical fixture. The engine tests cover
the canonical ProseMirror fixture
(`core/core-db/tests/fixtures/block-schema-v1.json`), text insertion/deletion,
paragraph merges, block splitting (including new list/task items), list wrapping/type-switching, empty-list-item Backspace, list nesting/outdenting, headings, marks, task toggles, table-cell edits and table-cell navigation, undo/redo, unknown nodes/attributes and JSON round-trips.

## What is here

- `src/main.rs` — eframe entry point (initial shell candidate; replaceable).
- `src/brand.rs` — shared palette, Inter/JetBrains Mono loading, vector lockup
  and the branded field/button primitives.
- `src/app.rs` — branded welcome/login/register gate, numbered recovery grid,
  notes list, editable title and document view, mark toolbar, debounced
  autosave and save-on-switch/lock.
- `src/core_worker.rs` — one `EnclaveCore` on a worker thread; typed events to
  the UI (load/save notes included). Vault work stays off the UI thread.
- `src/editor/model.rs` — ProseMirror contract, validation, JSON, paths.
- `src/editor/selection.rs` — single-block caret/range selection.
- `src/editor/engine.rs` — headless editing commands, history and tests.

## Next

1. IME and AccessKit passes; real CJK/combining input and a screen reader.
2. Table editing polish (row/column insertion/deletion, merged cells), keyboard
   shortcuts (undo/redo, save) and true bold typography.
3. Daily-driver shell: attachments, search, folders, settings, sync.
4. AI service: local embeddings + retrieval, opt-in remote, read-only agent.
5. Sync v4: deltas, compression, attachments, settings/templates.

## Brand assets

- `assets/brand/lockup-dark.svg` — the combined-path lockup used by the gate;
  [`docs/brand/lockup-dark.svg`](../../docs/brand/lockup-dark.svg) remains the
  geometry source of truth.
- `assets/fonts/` — Inter and JetBrains Mono, with their OFL licences. These
  match the fonts bundled by Android.

The editor engine is designed to become a separate MIT/Apache-2.0 project once
the desktop slice proves it; Enclave remains the first consumer.
