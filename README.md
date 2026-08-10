# Enclave

> **Secure, local-first, zero-knowledge knowledge base with P2P sync over local Wi-Fi.**
>
> No cloud. No servers. No internet required (the optional local-AI assistant talks only to an endpoint you configure).

## Architecture

Enclave is built on three core principles:

### 1. Local-First
All data lives on your device first. The application functions fully offline — create, edit, and organize pages without ever connecting to a network. Sync is an enhancement, not a requirement.

### 2. Encryption at Rest
The vault database is encrypted with **SQLCipher** (AES-256-CBC with HMAC-SHA512) before it touches persistent storage. Key derivation uses **Argon2id** with strong defaults (64 MiB, 3 iterations, 4 parallelism). A 12-word **BIP39** seed phrase unlocks the vault. The application never sees your plaintext keys — they're derived from credentials that never leave your device.

### 3. Peer-to-Peer Sync
When devices are on the same local network, they discover each other via **mDNS** (Multicast DNS) and sync documents directly over **WebSocket**, merged with **doc-level last-write-wins** resolution. No relay servers, no cloud routing — just device-to-device communication within your Wi-Fi boundary. Sync travels **unencrypted on the wire**, so run it on a network you trust (see [Security Model](#security-model)).

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Desktop Shell** | Tauri v2 (Rust) |
| **Frontend** | SvelteKit (static adapter) + Svelte 5 |
| **Editor** | TipTap (ProseMirror) + custom Svelte 5 wrapper |
| **Storage** | SQLite + SQLCipher (AES-256-CBC with HMAC-SHA512) |
| **Data Model** | Document + Block with a `sort_order` column |
| **Seed Phrase** | BIP39 12-word mnemonic (`@scure/bip39`) |
| **Key Derivation** | Argon2id (`hash-wasm`) |
| **Markdown I/O** | Hand-rolled HTML → Markdown serializer + `marked` (Markdown → HTML) |
| **Network Discovery** | mDNS (`mdns-sd`) |
| **Transport** | WebSocket (`tokio-tungstenite`) |
| **Sync** | Full JSON snapshots + doc-level last-write-wins merge |
| **Local AI** | OpenAI-compatible client (`/v1`) — Ollama, llama.cpp, LM Studio, vLLM |
| **Theming** | Light / dark with CSS custom properties (6 accents, 4 fonts, 3 densities) |
| **CI/CD** | GitHub Actions — tests + Windows (.msi/.exe) + Linux (.deb/.AppImage) + macOS (.dmg) |

## Supported Platforms

| Platform | Status | Bundle |
|----------|--------|--------|
| **Linux** (x86_64) | Supported | `.deb`, `.AppImage` |
| **Windows** (x86_64) | Supported | `.msi`, `.exe` (NSIS installer) |
| **macOS** (Apple Silicon) | Supported | `.dmg` |
| **Android** | Planned | — |

## Monorepo Structure

```
enclave/
├── .github/
│   └── workflows/build.yml        # CI: tests + Windows/Linux/macOS builds + releases
├── apps/
│   └── desktop/                   # Tauri desktop app (SvelteKit static adapter)
│       ├── src/
│       │   ├── app.html           # Root HTML shell
│       │   ├── app.css            # Global styles + theme variables (light/dark)
│       │   ├── lib/
│       │   │   ├── backend.ts          # Tauri IPC bridge (invoke/listen)
│       │   │   ├── ollama.ts           # OpenAI-compatible local-LLM client (chat/embeddings)
│       │   │   ├── importExport.ts     # Markdown/HTML import + export, vault backup
│       │   │   ├── graphLinks.ts       # Link extraction for graph view + backlinks
│       │   │   ├── wbLayout.ts         # Whiteboard layout helpers
│       │   │   ├── VaultGuard.svelte   # Vault creation / unlock flow
│       │   │   ├── SettingsPanel.svelte# Appearance, AI assistant, backup, shortcuts
│       │   │   ├── Whiteboard.svelte   # Infinite canvas editor
│       │   │   └── Icon.svelte         # Inline SVG icon set
│       │   └── routes/
│       │       ├── +layout.svelte  # App shell: sidebar, command palette, network, sync readout
│       │       ├── +layout.ts      # prerender / SSR config
│       │       ├── +page.svelte    # Home / recent pages + daily journal
│       │       ├── [id]/+page.svelte # Editor + AI ask/RAG panel
│       │       ├── capture/        # Quick Capture window
│       │       └── graph/          # Backlink graph view
│       ├── tests/                  # ollama, graphLinks, wbLayout
│       ├── static/
│       ├── package.json
│       ├── svelte.config.js
│       ├── vite.config.ts
│       └── tsconfig.json
├── packages/
│   ├── crypto/                     # BIP39 + Argon2id + AES-256-GCM
│   │   └── src/index.ts            # generateMnemonic, deriveMasterKey, encrypt/decrypt
│   ├── editor/                     # TipTap Svelte 5 wrapper + extensions
│   │   ├── src/
│   │   │   ├── TipTapEditor.svelte # Core editor
│   │   │   ├── markdown.ts         # Hand-rolled HTML → Markdown serializer
│   │   │   ├── extensions/         # slash-command, page-link, mention, callout, toggle-block,
│   │   │   │                       #   database, image, page-embed, bookmark, drag-handle
│   │   │   ├── blocks/             # SlashMenu, BubbleMenu, DragHandleMenu, TocPanel,
│   │   │   │                       #   DatabaseView, PageEmbedView, CodeBlockView, …
│   │   │   └── index.ts
│   │   └── test.ts                 # Markdown round-trip verification
│   ├── sync-engine/                # Yjs CRDT library (standalone; the desktop app syncs via
│   │   │                           #   Rust snapshot merge, see Network Design)
│   │   ├── src/index.ts            # SyncEngine class, per-doc Y.Doc management
│   │   ├── test.ts                 # Two-peer convergence
│   │   └── stress.test.ts          # 3+ peer convergence under lossy/slow links
│   └── ui/                         # Shared Svelte components, theme store, types
├── src-tauri/                      # Rust backend (Tauri v2)
│   ├── crates/
│   │   ├── core-db/                # Encrypted SQLite (SQLCipher) + FTS5 + embeddings
│   │   │   └── src/lib.rs          # Document/Block/embedding CRUD, vault lifecycle, sync merge
│   │   └── core-network/           # mDNS + WebSocket P2P transport
│   │       ├── src/lib.rs          # NetworkState, start/stop/status, peer redial
│   │       ├── src/mdns.rs         # _enclave._tcp.local. discovery
│   │       └── src/ws.rs           # WebSocket accept loop + session relay
│   ├── src/
│   │   ├── main.rs                 # Binary entry point
│   │   └── lib.rs                  # Tauri commands + sync protocol + tray/quick capture
│   ├── Cargo.toml                  # Rust workspace root
│   ├── tauri.conf.json             # App config, CSP, NSIS installer, bundle targets
│   └── icons/
├── tsconfig.base.json              # Shared TypeScript base config
└── package.json                    # npm workspace root
```

## Prerequisites

- **Rust** (1.77+) — [rustup.rs](https://rustup.rs)
- **Node.js** (24+, 26 recommended) — [nvm](https://github.com/nvm-sh/nvm) recommended
- **System dependencies** (Linux):

  ```bash
  sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
    librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```

- **Windows**: [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + WebView2 (pre-installed on Windows 10+)

## Getting Started

### 1. Install

```bash
git clone https://github.com/Pranesh-Selvaraj/Enclave.git
cd enclave
npm install
```

### 2. Development

**Full Tauri desktop app:**

```bash
npx tauri dev
# Compiles the Rust backend and opens the desktop window
```

### 3. Production Build

```bash
npx tauri build
# Produces platform-specific binaries in src-tauri/target/release/bundle/
```

Or let CI handle it — pushes to `v*` tags (and to `main`, and PRs to `main`) trigger GitHub Actions to build all platforms and publish a release with `.msi`/`.exe` (Windows), `.deb`/`.AppImage` (Linux), and `.dmg` (macOS Apple Silicon).

## Local AI Assistant (opt-in)

Enclave ships a zero-dependency AI client that talks to any **OpenAI-compatible** local
endpoint over `/v1` — **Ollama** (default `http://localhost:11434`), **llama.cpp /
llama-server**, **LM Studio**, or **vLLM**.

Enable it in **Settings → AI assistant**:

- **Enable local AI** — turns on the Ask-AI panel.
- **Endpoint URL** — where the server lives (default `http://localhost:11434`).
- **Model** — any model the endpoint exposes (fetched from `GET /v1/models`).
- **Vault-wide answers (RAG)** — on by default (see below).

Behavior:

- **Chat**: `POST /v1/chat/completions` (SSE) streams answers into the Ask-AI panel, with
  **Stop** support.
- **Retrieval (RAG)**: pages are embedded (`POST /v1/embeddings`) into an `embeddings` table
  on save. Questions are answered with the most similar pages injected as context — retrieved
  by client-side cosine similarity with an FTS5 fallback. Answers
  list their **Sources** with links to the pages used.
- **Privacy**: when AI is enabled, page content (for embedding + context injection) is sent to
  the configured endpoint. The desktop CSP restricts it to `localhost:*`.
  See [SECURITY.md](SECURITY.md).

## Running Tests

```bash
# Crypto pipeline (BIP39, Argon2id, AES-GCM)
npx tsx packages/crypto/test.ts

# Markdown round-trip (HTML → MD)
npx tsx packages/editor/test.ts

# Sync engine — two-peer convergence + 3+ peer lossy/slow stress
npx tsx packages/sync-engine/test.ts
npx tsx packages/sync-engine/stress.test.ts

# Frontend unit tests (AI client, graph links, whiteboard layout)
npx tsx apps/desktop/tests/ollama.test.ts
npx tsx apps/desktop/tests/graphLinks.test.ts
npx tsx apps/desktop/tests/wbLayout.test.ts

# Rust type-check
cargo check --manifest-path src-tauri/Cargo.toml

# Rust unit tests (per crate — the root manifest only tests the root package)
cargo test --manifest-path src-tauri/crates/core-db/Cargo.toml
cargo test --manifest-path src-tauri/crates/core-network/Cargo.toml

# Frontend type-check
npm run check -w @enclave/desktop
```

## Vault & Security

### First Launch
1. App prompts you to **create a new vault** with a password
2. A 12-word BIP39 English seed phrase is generated client-side
3. You must save this phrase — it's the **only** way to recover the vault if you forget your password
4. The seed phrase is stored locally, encrypted with Argon2id + AES-256-GCM (`vault.key`), so later unlocks only need your password

### Returning User
1. Enter your **password** (or the 12-word seed phrase if you forgot it)
2. Credentials are run through **Argon2id** (64 MiB, 3 iterations, 4 parallelism) to derive the 256-bit master key
3. The master key decrypts the **SQLCipher** database

### Crypto Flow
```
12-word mnemonic
       │
       ▼
  Argon2id(password=mnemonic, salt="enclave-vault-master-key-v1")
       │
       ▼
  256-bit master key ────► SQLCipher PRAGMA key (encrypt-at-rest DB,
                          AES-256-CBC + HMAC-SHA512)
                                │
                                └── vault.key: seed phrase re-encrypted
                                    with Argon2id(password + random salt) + AES-256-GCM
```

## Security Model

- **At rest**: the SQLCipher database is encrypted (default AES-256-CBC, HMAC-SHA512). Keys are derived via Argon2id (memory-hard, resistant to GPU/ASIC attacks).
- **In transit**: LAN WebSockets carry **plaintext** JSON snapshots of documents and blocks, merged with doc-level last-write-wins. Enclave treats the LAN as trusted — a device on the same network can connect and read synced content. Do not sync across an untrusted network.
- **No cloud**: the app makes no requests to Enclave infrastructure. The only optional outbound traffic is the local-AI feature, which sends content to an endpoint you configure (restricted to localhost by CSP).
- **Key material**: the seed phrase and derived keys exist only in memory during the session. The only on-disk copy is `vault.key` — the seed phrase re-encrypted with Argon2id + AES-256-GCM under your password, so it's unusable without it.

## Network Design

```
┌──────────────┐        mDNS (_enclave._tcp.local)       ┌──────────────┐
│  Laptop      │◄───────────────────────────────────────►│  Desktop     │
│  (Tauri)     │    WebSocket (plaintext, LAN-only)       │  (Tauri)     │
│              │◄───────────────────────────────────────►│              │
│  SQLite      │    full JSON snapshots {docs, blocks}    │  SQLite      │
│  +sqlcipher  │    doc-level last-write-wins merge       │  +sqlcipher  │
└──────────────┘                                          └──────────────┘
        │                                                        │
        │              No internet. No cloud.                     │
        │              Wi-Fi LAN only.                            │
        └────────────────────────────────────────────────────────┘
```

### Sync Protocol
1. **Start Sync** — begins mDNS advertising + WebSocket listener on an OS-assigned LAN port
2. **Discovery** — peers on the same Wi-Fi discover each other via `_enclave._tcp.local.`
3. **Connect** — WebSocket handshake; each side sends a `hello`
4. **Snapshot** — each peer replies with a full snapshot of documents + blocks
5. **Merge** — the receiver merges doc-level (last-write-wins by revision + timestamp; tombstones survive) and replies with an `ack`
6. **Resilience** — a peer that drops is redialed every 3 s; the UI footer shows `peers online · last sync … ago`
7. **Stop Sync** — shuts down mDNS + WebSocket and clears the peer list

## Editor Features

Every page toggles between **Paper** (documents) and **Whiteboard** (infinite canvas).

### Paper mode — slash commands

| Feature | How |
|---------|-----|
| Headings 1–3, Text | `/text`, `/h1`, `/h2`, `/h3` |
| Bullet / Numbered / Task lists | `/bullet`, `/numbered`, `/task` |
| Quote, Callout, Toggle, Divider | `/quote`, `/callout`, `/toggle`, `/divider` |
| Code block (language selector) | `/code` |
| **Database** | `/database` — typed tables, 12 column types, 5 views (table/kanban/list/gallery/timeline), grouping/sort/filters |
| **Linked Database** | `/linked database` — mirror another database on this page |
| **Page embed** | `/embed page` — inline page card |
| **Image** | `/image`, or paste/drop |
| **Bookmark** | `/bookmark`, or paste a URL |
| **Template** | `/template` — Meeting Notes, Project Plan, Daily Journal, Book Notes |
| Page links / backlinks | type `[[` + page title |
| Mentions | type `@` + page title |
| Formatting | select text → bubble menu (bold/italic/strike/code) |
| Block menu | hover the drag handle → duplicate/remove/cut/copy |
| Table of contents | outline panel with scroll-spy; click to jump |

### Editor chrome

- **Command palette** — `Ctrl+K` (search pages, run actions)
- **Shortcuts** — `Ctrl+N` new page, `Ctrl+B` toggle sidebar
- **Page metadata** — emoji icon, gradient cover, tags, comments, favorites
- **Info & safety** — created/modified/word counts, delete-to-trash with Undo toast
- **Export** — per-page Markdown or HTML (native dialog); whole-vault encrypted backup
- **Import** — Markdown files
- **AI** — Ask-AI panel with RAG retrieval and sources (see [Local AI Assistant](#local-ai-assistant-opt-in))

### Whiteboard mode

Infinite canvas with **select, pan, sticky notes, rectangles, ellipses, arrows, text,
frames, page embeds, and mind maps** (Tab adds a child node). **Presentation mode** steps
through frames; **PNG export** downloads a capture. Layout is persisted per page as a
`whiteboard` block.

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) — how to contribute
- [SECURITY.md](SECURITY.md) — vulnerability reporting and security model

## License

MIT
