# Contributing to Enclave

Thanks for contributing. Enclave is a security-sensitive local-first app — every change that touches crypto, storage, or networking needs extra care.

## Before You Write Code

- **Open an issue first.** Describe what you want to build or fix. This prevents wasted work on something that may not fit the project direction.
- For bugs, include reproduction steps. For features, explain the problem it solves.
- If you're unsure if something belongs, ask in the issue. YAGNI is a rule here — we avoid speculative features.

## Development Setup

### Prerequisites

- **Rust** 1.77+ — [rustup.rs](https://rustup.rs)
- **Node.js** 24+ (26 recommended) — [nvm](https://github.com/nvm-sh/nvm) recommended
- **Linux**: `sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev`
- **macOS**: Xcode Command Line Tools
- **Windows**: Microsoft Visual Studio C++ Build Tools + WebView2

### Getting Started

```bash
git clone git@github.com:Pranesh-Selvaraj/Enclave.git
cd Enclave
npm install
```

### Running the App

```bash
# Full Tauri desktop app
npx tauri dev

# Frontend-only (browser SPA — Tauri-only features are stubbed, see backend.ts)
npm run dev -w @enclave/desktop
```

### Running Tests

```bash
# Crypto pipeline (BIP39 → Argon2id → AES-GCM)
npx tsx packages/crypto/test.ts

# Markdown round-trip (hand-rolled HTML → MD + marked MD → HTML)
npx tsx packages/editor/test.ts

# Sync engine — two-peer convergence + 3+ peer lossy/slow stress
npx tsx packages/sync-engine/test.ts
npx tsx packages/sync-engine/stress.test.ts

# Frontend unit tests (AI client, IndexedDB web store, graph links, whiteboard layout)
npx tsx apps/desktop/tests/ollama.test.ts
npx tsx apps/desktop/tests/webStore.test.ts
npx tsx apps/desktop/tests/graphLinks.test.ts
npx tsx apps/desktop/tests/wbLayout.test.ts

# Rust type-check
cargo check --manifest-path src-tauri/Cargo.toml

# Rust unit tests (per crate — run each crate's manifest directly)
cargo test --manifest-path src-tauri/crates/core-db/Cargo.toml
cargo test --manifest-path src-tauri/crates/core-network/Cargo.toml

# Frontend type-check (svelte-check)
npm run check -w @enclave/desktop
```

CI runs all of the above plus the platform builds; a PR that fails any check won't merge.

## Project Structure

```
enclave/
├── .github/workflows/build.yml   # CI: tests + Windows/Linux/macOS builds + releases
├── apps/desktop/                 # Tauri desktop app + web SPA (SvelteKit static adapter)
│   ├── src/
│   │   ├── lib/                  # backend.ts (invoke bridge), webStore.ts (IndexedDB),
│   │   │                         #   ollama.ts (local-LLM client), importExport.ts,
│   │   │                         #   graphLinks.ts, wbLayout.ts, VaultGuard, Settings, Whiteboard
│   │   └── routes/               # +layout, home, [id] editor, capture, graph
│   └── tests/                    # ollama, webStore, graphLinks, wbLayout unit tests
├── packages/
│   ├── crypto/                   # BIP39, Argon2id, AES-256-GCM (TypeScript)
│   ├── editor/                   # TipTap Svelte 5 wrapper + extensions + markdown serializer
│   ├── sync-engine/              # Standalone Yjs CRDT library. NOT used for app sync —
│   │                             #   the desktop app syncs via Rust snapshot merge.
│   └── ui/                       # Shared Svelte components, theme store, types
├── src-tauri/
│   ├── crates/
│   │   ├── core-db/              # SQLite + SQLCipher storage, FTS5, embeddings, sync merge (Rust)
│   │   └── core-network/         # mDNS + WebSocket P2P (Rust)
│   ├── src/                      # Tauri command bridge + sync protocol + tray
│   └── tauri.conf.json           # App config, CSP, bundle targets
├── tsconfig.base.json            # Shared TypeScript base config
└── package.json                  # npm workspace root
```

## Making Changes

1. Open an issue describing the change.
2. Create a branch from `main`. Work happens on feature or `stage/N-<slug>` branches; each stage is merged to `main` via PR.
3. Make your change. Follow existing code style — match what's already there, don't introduce new patterns.
4. Write a test if the change is non-trivial. Existing tests are plain `test.ts` files run with `tsx` — follow that pattern.
5. Run type checks and tests (see above).
6. Open a PR referencing the issue.

### Code Style

- **TypeScript**: existing patterns in `packages/*/src/` and `apps/*/src/`. Use Svelte 5 runes (`$state`, `$effect`, `$derived`), not Svelte 4 stores. Runes must be in `.svelte.ts`/`.svelte.js` files — plain `.ts` files cannot use them.
- **Rust**: `cargo fmt` + `cargo clippy`. No `unsafe` without justification.
- **No new dependencies** unless there's a strong reason. Prefer stdlib, existing deps, or a few lines of code.
- **No abstractions without at least two callers.** An interface for one implementation, a factory for one product, or a config value that never changes — these get deleted in review.
- Mark deliberate simplifications with a `ponytail:` comment.

## Security-Sensitive Areas

Changes to these files need extra scrutiny — tag them clearly in the PR description:

- `packages/crypto/src/index.ts` — key derivation, encryption/decryption
- `src-tauri/crates/core-db/src/lib.rs` — encrypted storage, PRAGMA key handling, sync merge, FTS/embedding queries
- `src-tauri/crates/core-network/` — peer discovery, plaintext WebSocket transport, redial
- `apps/desktop/src/lib/VaultGuard.svelte` — password/seed handling, vault.key read/write
- `apps/desktop/src/lib/backend.ts` — the invoke bridge; decides which commands reach Tauri vs. the web store
- `apps/desktop/src/lib/webStore.ts` — IndexedDB + WebCrypto storage (the web build's encryption boundary)
- `apps/desktop/src/lib/ollama.ts` — sends page content to the configured LLM endpoint when AI is enabled
- `apps/desktop/src/routes/[id]/+page.svelte` — editor UI; rebuilds embeddings from page content on save

Notes on trust boundaries:

- **AI content handling**: when the AI assistant is enabled, page content is sent to the endpoint in `enclave-ai` (localStorage, plaintext) — the desktop CSP restricts that to `localhost:*`; the web build can reach any URL.
- **Web build**: the SPA stores encrypted records in IndexedDB with WebCrypto AES-256-GCM. It has no Rust backend — P2P sync and native file dialogs are unavailable there.

Rules for crypto/non-trivial changes: leave one runnable check behind. For security code, show the test passing in the PR description.

## Issue Labels

- `bug` — something's broken
- `enhancement` — new feature or improvement
- `security` — vulnerability or security hardening
- `documentation` — docs only
- `good first issue` — small, self-contained, good for new contributors

## License

MIT. By contributing, you agree that your contributions will be licensed under the MIT License.
