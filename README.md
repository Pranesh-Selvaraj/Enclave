# Enclave

> **Secure, local-first, zero-knowledge note-taking with P2P sync over local Wi-Fi.**
>
> No cloud. No servers. No internet required.

## Architecture

Enclave is built on three core principles:

### 1. Local-First
All data lives on your device first. The application functions fully offline — create, edit, and organize notes without ever connecting to a network. Sync is an enhancement, not a requirement.

### 2. Zero-Knowledge Encryption
Every note is encrypted with **AES-256-GCM** before it touches persistent storage. Key derivation uses **Argon2id** with strong defaults (64 MiB, 3 iterations, 4 parallelism). A 12-word **BIP39** seed phrase unlocks the vault. The application never sees your plaintext keys — they're derived from credentials that never leave your device.

### 3. Peer-to-Peer Sync
When devices are on the same local network, they discover each other via **mDNS** (Multicast DNS) and sync encrypted notes directly over **WebSocket** data channels using **Yjs CRDTs**. No relay servers, no cloud routing — just device-to-device communication within your Wi-Fi boundary.

## Tech Stack

| Layer | Technology | Status |
|-------|-----------|--------|
| **Desktop Shell** | Tauri v2 (Rust) | Done |
| **Frontend** | SvelteKit (static adapter) + Svelte 5 | Done |
| **Editor** | TipTap (ProseMirror) + custom Svelte 5 wrapper | Done |
| **Storage** | SQLite + sqlcipher (encrypted at rest) | Done |
| **Data Model** | Document + Block with fractional indexing | Done |
| **Seed Phrase** | BIP39 12-word mnemonic (`@scure/bip39`) | Done |
| **Key Derivation** | Argon2id (`hash-wasm`) | Done |
| **Markdown I/O** | `turndown` + `markdown-it` bidirectional bridge | Done |
| **Network Discovery** | mDNS (`mdns-sd`) | Done |
| **Transport** | WebSocket (`tokio-tungstenite`) | Done |
| **Sync Engine** | Yjs CRDT with encrypted transport | Done |
| **Theming** | Light / dark with CSS custom properties | Done |

## Monorepo Structure

```
enclave/
├── apps/
│   └── desktop/                  # Tauri desktop app (SvelteKit + static adapter)
│       ├── src/
│       │   ├── app.html          # Root HTML shell
│       │   ├── app.css           # Global styles + theme variables (light/dark)
│       │   ├── lib/
│       │   │   ├── VaultGuard.svelte  # Seed phrase unlock / vault creation
│       │   │   └── SettingsPanel.svelte  # Theme toggle + keyboard shortcuts
│       │   └── routes/
│       │       ├── +layout.svelte  # App shell, sidebar, command palette
│       │       ├── +page.svelte    # Home / recent pages
│       │       └── [id]/
│       │           └── +page.svelte  # Editor + slash menu + export button
│       ├── package.json
│       ├── svelte.config.js
│       ├── vite.config.ts
│       └── tsconfig.json
├── packages/
│   ├── crypto/                   # BIP39 + Argon2id + AES-256-GCM
│   │   ├── src/index.ts          # generateMnemonic, deriveMasterKey, encrypt, decrypt
│   │   └── test.ts               # Full crypto pipeline verification
│   ├── editor/                   # TipTap Svelte 5 wrapper + block chrome
│   │   ├── src/
│   │   │   ├── TipTapEditor.svelte   # Core editor (task lists, callouts, toggles)
│   │   │   ├── reactivity.ts         # Svelte 5 ↔ TipTap reactivity bridge
│   │   │   ├── markdown.ts           # HTML ↔ Markdown serialization
│   │   │   ├── extensions/
│   │   │   │   ├── slash-command.ts  # "/" trigger detection
│   │   │   │   ├── callout.ts        # Colored info/warning blocks
│   │   │   │   └── toggle-block.ts   # Collapsible sections
│   │   │   ├── blocks/
│   │   │   │   ├── SlashMenu.svelte  # Slash command palette
│   │   │   │   └── BubbleMenu.svelte # Text selection formatting
│   │   │   └── index.ts
│   │   └── test.ts               # Markdown round-trip verification
│   ├── sync-engine/              # Yjs CRDT + encrypted P2P sync
│   │   ├── src/index.ts          # SyncEngine class, per-doc Y.Doc management
│   │   └── test.ts               # Two-peer sync convergence test
│   └── ui/                       # Shared Svelte component library
│       ├── src/
│       │   ├── theme.ts          # Reactive theme store (dark/light, persisted)
│       │   ├── types.ts          # Document, Block interfaces
│       │   ├── components/
│       │   │   ├── Button.svelte
│       │   │   └── Modal.svelte
│       │   └── index.ts
│       └── package.json
├── src-tauri/                    # Rust backend (Tauri v2)
│   ├── crates/
│   │   ├── core-db/              # Encrypted SQLite storage (dynamic key)
│   │   │   ├── Cargo.toml        # rusqlite + sqlcipher
│   │   │   └── src/lib.rs        # Document/Block CRUD, vault lifecycle
│   │   └── core-network/         # mDNS + WebSocket P2P transport
│   │       ├── Cargo.toml        # mdns-sd + tokio-tungstenite
│   │       └── src/
│   │           ├── lib.rs        # NetworkState, start/stop/status
│   │           ├── mdns.rs       # _enclave._tcp.local. discovery
│   │           └── ws.rs         # WebSocket accept loop + peer relay
│   ├── src/
│   │   ├── main.rs               # Binary entry point
│   │   └── lib.rs                # Tauri commands (vault, docs, network, export)
│   ├── Cargo.toml                # Rust workspace root
│   ├── tauri.conf.json
│   └── icons/
├── package.json                  # npm workspace root
└── README.md
```

## Prerequisites

- **Rust** (1.77+) — [rustup.rs](https://rustup.rs)
- **Node.js** (22+) — [nvm](https://github.com/nvm-sh/nvm) recommended
- **System dependencies** (Linux):

  ```bash
  sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
    librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```

- **macOS**: Xcode Command Line Tools
- **Windows**: Microsoft Visual Studio C++ Build Tools + WebView2

## Getting Started

### 1. Install

```bash
git clone <repo-url> enclave
cd enclave
npm install
```

### 2. Development

**Full Tauri desktop app:**

```bash
npx tauri dev
# Compiles the Rust backend and opens the desktop window
```

**Frontend only** (browser — some features require Tauri runtime):

```bash
npm run dev
# Opens at http://127.0.0.1:5173
```

### 3. Production Build

```bash
npx tauri build
# Produces platform-specific binaries in src-tauri/target/release/bundle/
```

## Running Tests

```bash
# Crypto pipeline (BIP39, Argon2id, AES-GCM)
npx tsx packages/crypto/test.ts

# Markdown round-trip (MD → HTML → MD)
npx tsx packages/editor/test.ts

# Sync engine (two-peer Yjs convergence)
npx tsx packages/sync-engine/test.ts

# Rust type-check
cargo check --manifest-path src-tauri/Cargo.toml

# Frontend type-check
npm run check -w @enclave/desktop
```

## Vault & Security

### First Launch
1. App prompts you to **create a new vault**
2. A 12-word BIP39 English seed phrase is generated client-side
3. You must save this phrase — it's the **only** way to unlock your vault
4. Re-enter the phrase to confirm, then your encrypted vault is created

### Returning User
1. Enter your 12-word seed phrase to unlock
2. The phrase is validated then run through **Argon2id** (64 MiB, 3 iterations, 4 parallelism) to derive the 256-bit master key
3. The master key decrypts the **SQLCipher** database

### Crypto Flow
```
12-word mnemonic
       │
       ▼
  Argon2id(password=mnemonic, salt="enclave-vault-master-key-v1")
       │
       ▼
  256-bit master key ────► SQLCipher PRAGMA key (encrypt-at-rest DB)
       │
       └───► Transport key (future: HKDF-derived, encrypts P2P sync messages)
```

## Security Model

- **At rest**: All notes encrypted via AES-256-GCM. Keys derived via Argon2id (memory-hard, resistant to GPU/ASIC attacks).
- **In transit**: P2P connections use encrypted WebSocket channels within the local network perimeter. Messages are encrypted before leaving the device.
- **Zero-trust sync**: Peers exchange only encrypted CRDT update blobs. The receiving device cannot read notes without the user's decryption key — even if they're on the same network.
- **No telemetry**: The application makes zero outbound network requests. All communication is strictly local-network-only.
- **Key material**: The 12-word seed phrase and derived keys exist only in memory during the session. Never written to disk.

## Network Design

```
┌──────────────┐         mDNS (_enclave._tcp.local)    ┌──────────────┐
│  Laptop       │◄─────────────────────────────────────►│  Phone        │
│  (Tauri)     │         WebSocket (encrypted)          │  (Tauri)     │
│              │◄─────────────────────────────────────►│              │
│  SQLite      │    Encrypted Yjs CRDT update blobs     │  SQLite      │
│  +sqlcipher  │                                        │  +sqlcipher  │
└──────────────┘                                        └──────────────┘
        │                                                      │
        │              No internet. No cloud.                   │
        │              Wi-Fi LAN only.                          │
        └──────────────────────────────────────────────────────┘
```

### Network Lifecycle
1. **Start Sync** — begins mDNS advertising + WebSocket listener on an OS-assigned LAN port
2. **Discovery** — peers on the same Wi-Fi discover each other via `_enclave._tcp.local.`
3. **Connect** — WebSocket handshake establishes encrypted channel between peers
4. **Sync** — Yjs CRDT update blobs flow bidirectionally, encrypted with the transport key
5. **Stop Sync** — shuts down mDNS + WebSocket, clears peer list

## Editor Features

| Feature | Key | Status |
|---------|-----|--------|
| Headings 1–3 | `/h1`, `/h2`, `/h3` | Done |
| Bullet / Numbered lists | `/bullet`, `/numbered` | Done |
| **Task lists** (checkboxes) | `/task` | Done |
| Block quotes | `/quote` | Done |
| Code blocks | `/code` | Done |
| **Callouts** (info/warning boxes) | `/callout` | Done |
| **Toggle blocks** (collapsible) | `/toggle` | Done |
| Horizontal divider | `/divider` | Done |
| Text formatting | Select + bubble menu | Done |
| Command palette | `Ctrl+K` | Done |
| Markdown export | Button in editor toolbar | Done |

## Status

| Phase | Feature | Status |
|-------|---------|--------|
| **1** | Monorepo scaffolding, Tauri + SvelteKit integration | Done |
| **2** | BIP39 seed phrase, Argon2id key derivation, vault lock/unlock | Done |
| **3** | Markdown import/export bridge, block-based data layer | Done |
| **4** | mDNS peer discovery, WebSocket transport, Yjs CRDT sync engine | Done |
| **5** | Theme system, task lists, callouts, toggle blocks, settings panel | Done |

## License

MIT
