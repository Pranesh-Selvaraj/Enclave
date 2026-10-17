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

# Live dev run on an emulator/device (Vite HMR + native rebuilds)
cd src-tauri && npx tauri android dev

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
# Editor — markdown round-trip + jsdom insert/toolbar integration
npm test -w @enclave/editor
npm run test:insert -w @enclave/editor

# Crypto pipeline (BIP39 → Argon2id → AES-GCM)
npx tsx packages/crypto/test.ts

# Sync engine — two-peer convergence + 3+ peer lossy/slow stress
npx tsx packages/sync-engine/test.ts
npx tsx packages/sync-engine/stress.test.ts

# Frontend unit tests (AI client, CSP, graph links, save retries, whiteboard layout)
npx tsx apps/frontend/tests/ai.test.ts
npx tsx apps/frontend/tests/csp.test.ts
npx tsx apps/frontend/tests/graphLinks.test.ts
npx tsx apps/frontend/tests/saveRetry.test.ts
npx tsx apps/frontend/tests/wbLayout.test.ts

# Rust type-check + workspace tests
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path core/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend type-check (svelte-check)
npm run check -w @enclave/frontend

# Android — JVM unit tests (widget cache, note JSON)
cd src-tauri/gen/android && ./gradlew :app:testUniversalDebugUnitTest && cd -

# Android — regenerate the UniFFI Kotlin bindings and check for drift
# (CI runs the same check; see docs in core/core-api/src/ffi.rs)
cd src-tauri && cargo run -p core-api --features uniffi,cli --bin uniffi-bindgen -- \
  generate --library target/debug/libcore_api.so --language kotlin \
  --out-dir gen/android/app/src/main/java && git diff --exit-code -- gen/android/app/src/main/java/uniffi/ && cd -

# UI audit (screenshots + DOM checks, desktop + phone profiles)
npm run audit:ui
```

CI runs all of the above plus the platform builds; a PR that fails any check won't merge.

## Project Structure

```
enclave/
├── .github/workflows/build.yml   # CI: tests + Windows/Linux/macOS builds + releases
├── apps/frontend/                 # Shared SvelteKit frontend — desktop + Android shells
│   ├── src/
│   │   ├── lib/                  # backend.ts (Tauri IPC bridge), ai.ts (OpenAI-compatible
│   │   │                         #   client + vault settings), importExport.ts, graphLinks.ts,
│   │   │                         #   wbLayout.ts, updates.ts, haptics.ts, ActionSheet,
│   │   │                         #   VaultGuard, Settings, Whiteboard, UpdateDialog
│   │   └── routes/               # +layout, home notes grid, [id] editor, split, graph,
│   │                             #   capture, widget
│   └── tests/                    # ai, csp, graphLinks, saveRetry, wbLayout unit tests
├── packages/
│   ├── crypto/                   # BIP39, Argon2id, AES-256-GCM (TypeScript)
│   ├── editor/                   # Tiptap 3 Svelte 5 wrapper + extensions + markdown + templates
│   ├── sync-engine/              # Standalone Yjs CRDT library. NOT used for app sync —
│   │                             #   the desktop app syncs via Rust snapshot merge.
│   └── ui/                       # Shared Svelte components, theme store, types
├── src-tauri/
│   ├── crates/
│   │   ├── core-db/              # SQLite + SQLCipher storage, FTS5, embeddings, sync merge (Rust)
│   │   ├── core-network/         # mDNS + WebSocket P2P (Rust)
│   │   └── core-api/             # Shell-agnostic API + UniFFI bindings for Android
│   ├── src/                      # Tauri command bridge + sync protocol + tray + updater
│   └── tauri.conf.json           # App config, CSP, bundle targets
├── docs/                         # README screenshots
├── tools/ui-audit/               # Screenshot + DOM audit suite (desktop + phone profiles)
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
- `core/core-db/src/lib.rs` — encrypted storage, PRAGMA key handling, sync merge, FTS/embedding queries
- `core/core-network/` — peer discovery, plaintext WebSocket transport, redial
- `apps/frontend/src/lib/VaultGuard.svelte` — password/seed handling, vault.key read/write
- `apps/frontend/src/lib/backend.ts` — the Tauri IPC bridge; the single place the frontend invokes Rust commands
- `apps/frontend/src/lib/ai.ts` — sends page content to the configured LLM endpoint when AI is enabled; reads/writes vault settings incl. the API key
- `src-tauri/src/embed.rs` — built-in offline embedding model (fastembed/ONNX)
- `apps/frontend/src/routes/[id]/+page.svelte` — editor UI; rebuilds embeddings from page content on save

Notes on trust boundaries:

- **AI content handling**: when the AI assistant is enabled, page content is sent to the endpoint in `enclave-ai` (vault settings, encrypted at rest) — local servers by default; any https endpoint once an API key is added. With **offline embeddings** on, retrieval content never leaves the device.

Rules for crypto/non-trivial changes: leave one runnable check behind. For security code, show the test passing in the PR description.

## Decision Records

Significant technical decisions are recorded in the repository, not only in chat or PR discussion. `docs/HISTORY.md` is the project timeline, `docs/decisions/` holds the Architecture Decision Records (ADRs), and `docs/decisions/README.md` is the index and decision map.

Record a decision when it changes any of: the UI stack, shell or architecture, storage engine, crypto, sync protocol, vault or block schema, FFI/contract, repository layout, platform support, or distribution/update mechanism. An ADR states what was chosen, why, the alternatives, the consequences, and — when it replaces an earlier choice — what was wrong before.

Do it in the same change that makes the decision. Never delete an old ADR: mark it `Superseded by NNNN` and add the new record. A change that only implements an already-recorded decision does not need a new ADR, but it must update `docs/HISTORY.md` if it changes an era or a contract version.

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
2. **Changelog** — add the new release to the top of `CHANGELOG.md` and
   update the compare links at the bottom. Every release uses the same layout:

   - Heading: `## [<new>] - YYYY-MM-DD` (no `v`, ISO date).
   - A one-paragraph prose summary of the release under the heading.
   - Section headings in this order, each carrying an emoji and omitted when
     empty: `### ✨ Added`, `### ⚡ Improved`, `### 🔧 Changed`,
     `### 🗑️ Removed`, `### 🐛 Fixed`, `### 🔒 Security`, `### 📝 Notes`.
   - One single-line, emoji-prefixed bullet per change, starting with a bold
     headline: `- 🔐 **Short headline.** What changed, why, and the concrete
     fix, with issue/PR/advisory references.`

   Add `[<new>]: .../compare/v<previous>...v<new>` to the reference block at
   the bottom so the heading links to its diff.

   **Release notes must be self-contained prose.** Say, in words, what was
   added, changed, improved and fixed, and explain the concrete cause and fix
   behind each item — including advisory, issue or CVE references where they
   exist. Never ship forge-generated “What's Changed” PR/branch lists, and
   never point readers at a branch, diff or commit instead of the
   description. The `CHANGELOG.md` entry and the forge release body must
   carry the same detailed text; after publishing, check the release page and
   replace generated notes if the forge injected them. Write each paragraph
   and bullet on a single line (no manual hard wrapping) — GitHub renders
   single newlines as hard breaks, so wrapped text shows as ragged lines.
3. **Verify** — `cargo test --manifest-path core/Cargo.toml` and
   `cargo test --manifest-path src-tauri/Cargo.toml`,
   `npm run check -w @enclave/frontend`, and (for Android) a release build:
   `cd src-tauri && npx tauri android build --target aarch64` with
   `ANDROID_HOME`/`NDK_HOME`/`JAVA_HOME` set — confirms the Android config
   (versionCode auto-increment, manifests) is valid.
4. **Never push directly to `main`** — branch protection requires a PR,
   and the agent must honor that even when bypass rights exist. Commit on a
   release branch, open a PR with the changelog entry in the description,
   merge it, then tag + publish:

   ```bash
   git tag -a v<new> -m "v<new>"
   git push origin main --tags
   ```

   Pushing the tag makes CI create the GitHub release: the title is the bare tag
   `v<new>`, matching the git tag and the `## [<new>]` changelog heading, and the
   body is extracted from that changelog entry, so the page and `CHANGELOG.md`
   carry the same text. The description lives in the body, never in the title.
   If the page needs correcting, edit it in place rather than re-creating it:

   ```bash
   gh release edit v<new> --title "v<new>" --notes "$(changelog body)"
   ```

Android release signing runs in CI (`.github/workflows/android.yml`) with the
`ANDROID_KEYSTORE_*` secrets — the keystore itself stays out of the repo and must
be backed up off-machine (losing it makes installed copies unupdatable).

Update signing follows the same rule (issue #83). CI signs every release asset
with the `UPDATE_SIGNING_KEY` secret — the `release` job in
`.github/workflows/build.yml` and the Android workflow — producing `<asset>.sig`
plus `SHA256SUMS`/`SHA256SUMS.sig`. The app verifies the signature against the
public key pinned in `src-tauri/src/update_signature.rs` and refuses unsigned
downloads, so a tag build whose "Sign release assets" step fails is not
installable until fixed.

The secret is a PKCS#8 Ed25519 PEM. Generate it, keep it offline and back it up
(losing it means shipping a release signed by a new key can only be installed
manually), then set it:

```bash
node --input-type=module -e "import { generateKeyPairSync } from 'node:crypto'; import { writeFileSync } from 'node:fs'; const { publicKey, privateKey } = generateKeyPairSync('ed25519'); writeFileSync('enclave-update-signing.pem', privateKey.export({ type: 'pkcs8', format: 'pem' }), { mode: 0o600 }); console.log(publicKey.export({ type: 'spki', format: 'der' }).subarray(-32).toString('hex'));"
gh secret set UPDATE_SIGNING_KEY < enclave-update-signing.pem
```

Update the pinned public key constant with the printed hex when rotating.
