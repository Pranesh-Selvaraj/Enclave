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
- **Android**: JDK 21+, Android SDK (platform 36, build-tools 36.0.0), NDK 29.0.13846066, rustup targets `aarch64-linux-android` (+ `x86_64-linux-android` for emulators) — see the [Android section](README.md#android-beta) of the README.

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
```

### Android builds & signing

```bash
# Prereqs (see README "Android (Beta)"): JDK 21, Android SDK + NDK, rustup targets
export ANDROID_HOME=~/Android/Sdk NDK_HOME=$ANDROID_HOME/ndk/29.0.13846066 JAVA_HOME=<jdk21>

# Build the release APK + AAB (arm64)
cd src-tauri && npx tauri android build --target aarch64 && cd ..

# Sign both artifacts
scripts/android-sign.sh
```

**Keystore** (never commit it):

```bash
keytool -genkeypair -v -keystore ~/Android/keystore/enclave-release.jks \
  -alias enclave -keyalg RSA -keysize 2048 -validity 10000 \
  -storepass "<random>" -keypass "<random>" -dname "CN=Enclave, OU=Mobile, O=Enclave, C=US"
echo "<random>" > ~/Android/keystore/enclave-release.jks.password && chmod 600 ~/Android/keystore/enclave-release.jks.password
```

The script reads `KEYSTORE_PATH`/`KEYSTORE_B64` + passwords from env (CI uses secrets);
without a keystore it warns and exits 0 so fork PRs stay green. CI signing uses
`ANDROID_KEYSTORE_B64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`,
`ANDROID_KEY_PASSWORD` secrets (see `.github/workflows/android.yml`).

### Running Tests

```bash
# Crypto pipeline (BIP39 → Argon2id → AES-GCM)
npx tsx packages/crypto/test.ts

# Markdown round-trip (hand-rolled HTML → MD + marked MD → HTML)
npx tsx packages/editor/test.ts

# Sync engine — two-peer convergence + 3+ peer lossy/slow stress
npx tsx packages/sync-engine/test.ts
npx tsx packages/sync-engine/stress.test.ts

# Frontend unit tests (AI client, graph links, whiteboard layout)
npx tsx apps/frontend/tests/ai.test.ts
npx tsx apps/frontend/tests/graphLinks.test.ts
npx tsx apps/frontend/tests/wbLayout.test.ts

# Rust type-check
cargo check --manifest-path src-tauri/Cargo.toml

# Rust unit tests (per crate — run each crate's manifest directly)
cargo test --manifest-path src-tauri/crates/core-db/Cargo.toml
cargo test --manifest-path src-tauri/crates/core-network/Cargo.toml

# Frontend type-check (svelte-check)
npm run check -w @enclave/frontend
```

CI runs all of the above plus the platform builds; a PR that fails any check won't merge.

## Project Structure

```
enclave/
├── .github/workflows/build.yml   # CI: tests + Windows/Linux/macOS builds + releases
├── apps/frontend/                 # Tauri desktop app (SvelteKit static adapter)
│   ├── src/
│   │   ├── lib/                  # backend.ts (Tauri IPC bridge), ai.ts (OpenAI-compatible
│   │   │                         #   client + vault settings), importExport.ts, graphLinks.ts,
│   │   │                         #   wbLayout.ts, VaultGuard, Settings, Whiteboard
│   │   └── routes/               # +layout, home, [id] editor, capture, graph
│   └── tests/                    # ai, graphLinks, wbLayout unit tests
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
- `apps/frontend/src/lib/VaultGuard.svelte` — password/seed handling, vault.key read/write
- `apps/frontend/src/lib/backend.ts` — the Tauri IPC bridge; the single place the frontend invokes Rust commands
- `apps/frontend/src/lib/ai.ts` — sends page content to the configured LLM endpoint when AI is enabled; reads/writes vault settings incl. the API key
- `src-tauri/src/embed.rs` — built-in offline embedding model (fastembed/ONNX)
- `apps/frontend/src/routes/[id]/+page.svelte` — editor UI; rebuilds embeddings from page content on save

Notes on trust boundaries:

- **AI content handling**: when the AI assistant is enabled, page content is sent to the endpoint in `enclave-ai` (vault settings, encrypted at rest) — local servers by default; any https endpoint once an API key is added. With **offline embeddings** on, retrieval content never leaves the device.

Rules for crypto/non-trivial changes: leave one runnable check behind. For security code, show the test passing in the PR description.

## Issue Labels

- `bug` — something's broken
- `enhancement` — new feature or improvement
- `security` — vulnerability or security hardening
- `documentation` — docs only
- `good first issue` — small, self-contained, good for new contributors

## License

MIT. By contributing, you agree that your contributions will be licensed under the MIT License.

## Releasing

1. **Bump the version** to `<new>` across `package.json`, `apps/frontend/package.json`,
   `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` (Cargo.lock updates on the
   next `cargo check`).
2. **Changelog** — add a `[<new>]` entry under `## [Unreleased]` in `CHANGELOG.md`
   (Keep a Changelog format) and update the compare links at the bottom.
3. **Verify** — `cargo test --workspace --manifest-path src-tauri/Cargo.toml`,
   `npm run check -w @enclave/frontend`, and (for Android) a release build:
   `cd src-tauri && npx tauri android build --target aarch64` with
   `ANDROID_HOME`/`NDK_HOME`/`JAVA_HOME` set — confirms the Android config
   (versionCode auto-increment, manifests) is valid.
4. **Never push directly to `main`** — branch protection requires a PR,
   and the agent must honor that even when bypass rights exist. Commit on a
   release branch, open a PR with the changelog entry in the description,
   merge it, then tag + publish:

   ```bash
   git tag -a v<new> -m "v<new> — <summary>"
   git push origin main --tags
   gh release create v<new> --title "Enclave v<new> — <summary>" --notes "$(changelog body)"
   ```

Android release signing runs in CI (`.github/workflows/android.yml`) with the
`ANDROID_KEYSTORE_*` secrets — the keystore itself stays out of the repo and must
be backed up off-machine (losing it makes installed copies unupdatable).
