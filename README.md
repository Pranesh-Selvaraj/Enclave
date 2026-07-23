# Enclave

> **Secure, local-first, zero-knowledge knowledge base with P2P sync over local Wi-Fi.**
>
> No cloud. No servers. No internet required.

## Architecture

Enclave is built on three core principles:

### 1. Local-First
All data lives on your device first. The application functions fully offline — create, edit, and organize pages without ever connecting to a network. Sync is an enhancement, not a requirement.

### 2. Zero-Knowledge Encryption
Every page is encrypted with **AES-256-GCM** before it touches persistent storage. Key derivation uses **Argon2id** with strong defaults (64 MiB, 3 iterations, 4 parallelism). A 12-word **BIP39** seed phrase unlocks the vault. The application never sees your plaintext keys — they're derived from credentials that never leave your device.

### 3. Peer-to-Peer Sync
When devices are on the same local network, they discover each other via **mDNS** (Multicast DNS) and sync encrypted pages directly over **WebSocket** data channels using **Yjs CRDTs**. No relay servers, no cloud routing — just device-to-device communication within your Wi-Fi boundary.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Desktop Shell** | Tauri v2 (Rust) |
| **Frontend** | SvelteKit (static adapter) + Svelte 5 |
| **Editor** | TipTap (ProseMirror) + custom Svelte 5 wrapper |
| **Storage** | SQLite + sqlcipher (encrypted at rest) |
| **Data Model** | Document + Block with fractional indexing |
| **Seed Phrase** | BIP39 12-word mnemonic (`@scure/bip39`) |
| **Key Derivation** | Argon2id (`hash-wasm`) |
| **Markdown I/O** | `turndown` + `markdown-it` bidirectional bridge |
| **Network Discovery** | mDNS (`mdns-sd`) |
| **Transport** | WebSocket (`tokio-tungstenite`) |
| **Sync Engine** | Yjs CRDT with encrypted transport |
| **Theming** | Light / dark with CSS custom properties |
| **CI/CD** | GitHub Actions — Windows (.msi/.exe) + Linux (.deb/.AppImage) + macOS (.dmg) |

## Supported Platforms

| Platform | Status | Bundle |
|----------|--------|--------|
| **Linux** (x86_64) | Supported | `.deb`, `.AppImage` |
| **Windows** (x86_64) | Supported | `.msi`, `.exe` (NSIS installer) |
| **macOS** (ARM + Intel) | Supported | `.dmg` |
| **Android** | Planned | — |
| **Web** | Planned | Static SPA (IndexedDB) |

## Monorepo Structure

```
enclave/
├── .github/
│   └── workflows/build.yml        # CI: Windows + Linux builds
├── apps/
│   └── desktop/                   # Tauri desktop app (SvelteKit + static adapter)
│       ├── src/
│       │   ├── app.html           # Root HTML shell
│       │   ├── app.css            # Global styles + theme variables (light/dark)
│       │   ├── lib/
│       │   │   ├── VaultGuard.svelte     # Seed phrase unlock / vault creation
│       │   │   └── SettingsPanel.svelte  # Theme toggle + keyboard shortcuts
│       │   └── routes/
│       │       ├── +layout.svelte  # App shell, sidebar, command palette, network toggle
│       │       ├── +page.svelte    # Home / recent pages
│       │       └── [id]/
│       │           └── +page.svelte  # Editor + slash menu + markdown export
│       ├── static/
│       ├── package.json
│       ├── svelte.config.js
│       ├── vite.config.ts
│       └── tsconfig.json
├── packages/
│   ├── crypto/                    # BIP39 + Argon2id + AES-256-GCM
│   │   ├── src/index.ts           # generateMnemonic, deriveMasterKey, encrypt, decrypt
│   │   └── test.ts                # Full crypto pipeline verification
│   ├── editor/                    # TipTap Svelte 5 wrapper + block chrome
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
│   │   └── test.ts                # Markdown round-trip verification
│   ├── sync-engine/               # Yjs CRDT + encrypted P2P sync
│   │   ├── src/index.ts           # SyncEngine class, per-doc Y.Doc management
│   │   └── test.ts                # Two-peer sync convergence test
│   └── ui/                        # Shared Svelte component library
│       ├── src/
│       │   ├── theme.ts           # Reactive theme store (dark/light, persisted)
│       │   ├── types.ts           # Document, Block interfaces
│       │   ├── components/
│       │   │   ├── Button.svelte
│       │   │   └── Modal.svelte
│       │   └── index.ts
│       └── package.json
├── src-tauri/                     # Rust backend (Tauri v2)
│   ├── crates/
│   │   ├── core-db/               # Encrypted SQLite storage (dynamic key)
│   │   │   ├── Cargo.toml         # rusqlite + sqlcipher
│   │   │   └── src/lib.rs         # Document/Block CRUD, vault lifecycle
│   │   └── core-network/          # mDNS + WebSocket P2P transport
│   │       ├── Cargo.toml         # mdns-sd + tokio-tungstenite
│   │       └── src/
│   │           ├── lib.rs         # NetworkState, start/stop/status
│   │           ├── mdns.rs        # _enclave._tcp.local. discovery
│   │           └── ws.rs          # WebSocket accept loop + peer relay
│   ├── src/
│   │   ├── main.rs                # Binary entry point + windows subsystem config
│   │   └── lib.rs                 # Tauri commands (vault, docs, blocks, network, export)
│   ├── Cargo.toml                 # Rust workspace root
│   ├── tauri.conf.json            # App config, CSP, NSIS installer, bundle targets
│   └── icons/
├── package.json                   # npm workspace root
└── README.md
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

**Frontend only** (browser — some features require Tauri runtime):

```bash
npm run dev -w @enclave/desktop
# Opens at http://127.0.0.1:5173
```

### 3. Production Build

```bash
npx tauri build
# Produces platform-specific binaries in src-tauri/target/release/bundle/
```

Or let CI handle it — pushes to `v*` tags trigger GitHub Actions to build all platforms and publish a release with `.msi`/`.exe` (Windows), `.deb`/`.AppImage` (Linux), and `.dmg` (macOS).

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
```

## Security Model

- **At rest**: All pages encrypted via AES-256-GCM. Keys derived via Argon2id (memory-hard, resistant to GPU/ASIC attacks).
- **In transit**: P2P connections use encrypted WebSocket channels within the local network perimeter. Messages are encrypted before leaving the device.
- **Zero-trust sync**: Peers exchange only encrypted CRDT update blobs. The receiving device cannot read pages without the user's decryption key — even if they're on the same network.
- **No telemetry**: The application makes zero outbound network requests. All communication is strictly local-network-only.
- **Key material**: The 12-word seed phrase and derived keys exist only in memory during the session. Never written to disk.

## Network Design

```
┌──────────────┐         mDNS (_enclave._tcp.local)    ┌──────────────┐
│  Laptop       │◄─────────────────────────────────────►│  Desktop       │
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

| Feature | Key |
|---------|-----|
| Headings 1–3 | `/h1`, `/h2`, `/h3` |
| Bullet / Numbered lists | `/bullet`, `/numbered` |
| Task lists (checkboxes) | `/task` |
| Block quotes | `/quote` |
| Code blocks | `/code` |
| Callouts (info/warning boxes) | `/callout` |
| Toggle blocks (collapsible) | `/toggle` |
| Horizontal divider | `/divider` |
| Text formatting | Select + bubble menu |
| Command palette | `Ctrl+K` |
| Markdown export | Button in editor toolbar |

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) — how to contribute
- [SECURITY.md](SECURITY.md) — vulnerability reporting and security model

## License

MIT
