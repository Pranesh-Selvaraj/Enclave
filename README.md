# Enclave

> 🔒 **Secure, local-first, zero-knowledge note-taking with P2P sync over local Wi-Fi.**
>
> No cloud. No servers. No internet required.

## Architecture

Enclave is built on three core principles:

### 1. Local-First
All data lives on your device first. The application functions fully offline — create, edit, and organize notes without ever connecting to a network. Sync is an enhancement, not a requirement.

### 2. Zero-Knowledge Encryption
Every note is encrypted with **AES-256-GCM** before it touches persistent storage. Key derivation uses **Argon2id** with strong defaults. The application never sees your plaintext keys — they're derived from credentials that never leave your device.

### 3. Peer-to-Peer Sync
When devices are on the same local network, they discover each other via **mDNS** (Multicast DNS) and sync encrypted notes directly over **WebSocket / WebRTC data channels**. No relay servers, no cloud routing — just device-to-device communication within your Wi-Fi boundary.

## Tech Stack

| Layer | Technology | Status |
|-------|-----------|--------|
| **Desktop Shell** | Tauri v2 (Rust) | ✅ |
| **Frontend** | SvelteKit (static adapter) + Tailwind CSS | ✅ |
| **Editor** | TipTap (ProseMirror) + custom Svelte 5 wrapper | ✅ |
| **Storage** | SQLite + sqlcipher (encrypted at rest) | ✅ |
| **Data Model** | Document + Block model with fractional indexing | ✅ |
| **Network Discovery** | mDNS (multicast DNS) | Planned |
| **Transport** | WebSockets / WebRTC data channels | Planned |
| **Sync Engine** | Yjs (CRDT for conflict-free text merging) | Planned |
| **Key Derivation** | Argon2id | Planned |

## Monorepo Structure

```
enclave/
├── apps/
│   └── desktop/                  # Tauri desktop app (SvelteKit + static adapter)
│       ├── src/
│       │   ├── app.html          # Root HTML shell
│       │   ├── app.css           # Global styles + Tailwind
│       │   ├── lib/              # App-specific utilities
│       │   └── routes/           # SvelteKit pages
│       │       ├── +layout.svelte  # App shell + command palette
│       │       ├── +layout.ts
│       │       ├── +page.svelte    # Home / recent pages
│       │       └── [id]/           # Document page (dynamic route)
│       │           └── +page.svelte  # Editor + slash menu + bubble menu
│       ├── package.json
│       ├── svelte.config.js
│       ├── vite.config.ts
│       └── tsconfig.json
├── packages/
│   ├── editor/                   # TipTap Svelte 5 wrapper + block chrome
│   │   ├── src/
│   │   │   ├── TipTapEditor.svelte  # Core editor component
│   │   │   ├── reactivity.ts        # Svelte 5 ↔ TipTap reactivity
│   │   │   ├── extensions/          # SlashCommand, WikiLink (future)
│   │   │   ├── blocks/              # SlashMenu, BubbleMenu
│   │   │   └── index.ts
│   │   └── package.json
│   └── ui/                       # Shared Svelte component library
│       ├── src/
│       │   ├── components/       # Button, NoteCard, Sidebar, EmptyState
│       │   ├── types.ts          # Document, Block interfaces
│       │   └── index.ts
│       └── package.json
├── src-tauri/                    # Rust backend (Tauri v2)
│   ├── crates/
│   │   └── core-db/              # Encrypted storage engine
│   │       ├── Cargo.toml        # rusqlite + sqlcipher
│   │       └── src/lib.rs        # Note struct, query helpers, init_db()
│   ├── src/
│   │   ├── main.rs               # Binary entry point
│   │   └── lib.rs                # Tauri commands + app builder
│   ├── Cargo.toml                # Rust workspace root
│   ├── tauri.conf.json
│   └── icons/
├── package.json                  # npm workspace root
└── README.md
```

### Planned (not yet scaffolded)

```
apps/web/          # PWA deployment target
apps/mobile/       # Capacitor / Tauri mobile
packages/crypto/   # Client-side AES-256-GCM + Argon2id (TS)
packages/sync-engine/  # Yjs CRDT + P2P mesh (TS)
src-tauri/crates/core-network/  # mDNS + WebSocket transport (Rust)
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

### 1. Clone & Install

```bash
git clone <repo-url> enclave
cd enclave

# Install all workspace dependencies
npm install
```

### 2. Development

**Frontend only** (browser):

```bash
npm run dev
# Opens at http://127.0.0.1:5173
```

**Full Tauri desktop app**:

```bash
npx tauri dev
# Compiles the Rust backend and opens the desktop window
```

### 3. Production Build

```bash
npx tauri build
# Produces platform-specific binaries in src-tauri/target/release/bundle/
```

## Security Model

- **At rest**: All notes encrypted via AES-256-GCM. Keys derived via Argon2id (memory-hard, resistant to GPU/ASIC attacks).
- **In transit**: P2P connections use encrypted WebSocket channels within the local network perimeter. WebRTC data channels are DTLS-encrypted by default.
- **Zero-trust sync**: Peers exchange only encrypted blobs. The receiving device cannot read notes without the user's decryption key — even if they're on the same network.
- **No telemetry**: The application makes zero outbound network requests. All communication is strictly local-network-only.

## Network Design

```
┌──────────────┐         mDNS discovery        ┌──────────────┐
│  Laptop       │◄─────────────────────────────►│  Phone        │
│  (Tauri)     │         WS / WebRTC            │  (Tauri)     │
│              │◄─────────────────────────────►│              │
│  SQLite      │    Encrypted CRDT blobs only    │  SQLite      │
│  +sqlcipher  │                                 │  +sqlcipher  │
└──────────────┘                                 └──────────────┘
        │                                               │
        │           No internet. No cloud.               │
        │           Wi-Fi LAN only.                      │
        └───────────────────────────────────────────────┘
```

## Status

| Phase | Feature | Status |
|---|---|---|
| **1** | Monorepo scaffolding, Tauri + SvelteKit integration | ✅ |
| **2** | Encrypted SQLite storage (sqlcipher), document + block CRUD | ✅ |
| **3** | Notion-inspired block editor (TipTap), slash commands, bubble menu, command palette | ✅ |
| **4** | Argon2id key derivation, mDNS peer discovery, P2P sync | Planned |
| **5** | Obsidian-style graph view, backlinks, `[[wikilinks]]` | Planned |
| **6** | Notion-style databases (table, board, calendar views) | Planned |

## License

MIT
