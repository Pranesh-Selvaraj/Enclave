# Contributing to Enclave

Thanks for contributing. Enclave is a security-sensitive local-first app — every change that touches crypto, storage, or networking needs extra care.

## Before You Write Code

- **Open an issue first.** Describe what you want to build or fix. This prevents wasted work on something that may not fit the project direction.
- For bugs, include reproduction steps. For features, explain the problem it solves.
- If you're unsure if something belongs, ask in the issue. YAGNI is a rule here — we avoid speculative features.

## Development Setup

### Prerequisites

- **Rust** 1.77+ — [rustup.rs](https://rustup.rs)
- **Node.js** 22+ — [nvm](https://github.com/nvm-sh/nvm) recommended
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

# Frontend-only (browser, some features require Tauri)
npm run dev
```

### Running Tests

```bash
# Crypto pipeline (BIP39 → Argon2id → AES-GCM)
npx tsx packages/crypto/test.ts

# Markdown round-trip
npx tsx packages/editor/test.ts

# Sync engine (two-peer Yjs convergence)
npx tsx packages/sync-engine/test.ts

# Rust type-check
cargo check --manifest-path src-tauri/Cargo.toml

# Frontend type-check
npm run check -w @enclave/desktop
```

## Project Structure

```
enclave/
├── apps/desktop/         # Tauri + SvelteKit frontend
├── packages/
│   ├── crypto/           # BIP39, Argon2id, AES-256-GCM (TypeScript)
│   ├── editor/           # TipTap Svelte 5 wrapper + extensions
│   ├── sync-engine/      # Yjs CRDT + encrypted transport
│   └── ui/               # Shared Svelte components
├── src-tauri/
│   ├── crates/
│   │   ├── core-db/      # SQLite + sqlcipher storage (Rust)
│   │   └── core-network/ # mDNS + WebSocket P2P (Rust)
│   └── src/              # Tauri command bridge
└── package.json          # npm workspace root
```

## Making Changes

1. Open an issue describing the change.
2. Create a branch from `main`.
3. Make your change. Follow existing code style — match what's already there, don't introduce new patterns.
4. Write a test if the change is non-trivial. Existing tests are plain `test.ts` files run with `tsx` — follow that pattern.
5. Run type checks and tests (see above).
6. Open a PR referencing the issue.

### Code Style

- **TypeScript**: existing patterns in `packages/*/src/` and `apps/*/src/`. Use Svelte 5 runes (`$state`, `$effect`, `$derived`), not Svelte 4 stores.
- **Rust**: `cargo fmt` + `cargo clippy`. No `unsafe` without justification.
- **No new dependencies** unless there's a strong reason. Prefer stdlib, existing deps, or a few lines of code.
- **No abstractions without at least two callers.** An interface for one implementation, a factory for one product, or a config value that never changes — these get deleted in review.

## Security-Sensitive Areas

Changes to these files need extra scrutiny — tag them clearly in the PR description:

- `packages/crypto/src/index.ts` — key derivation, encryption/decryption
- `src-tauri/crates/core-db/src/lib.rs` — encrypted storage, PRAGMA key handling
- `src-tauri/crates/core-network/` — peer discovery, message transport
- `apps/desktop/src/routes/[id]/+page.svelte` — key material handling in the UI

Rules for crypto/non-trivial changes: leave one runnable check behind. For security code, show the test passing in the PR description.

## Issue Labels

- `bug` — something's broken
- `enhancement` — new feature or improvement
- `security` — vulnerability or security hardening
- `documentation` — docs only
- `good first issue` — small, self-contained, good for new contributors

## License

MIT. By contributing, you agree that your contributions will be licensed under the MIT License.
