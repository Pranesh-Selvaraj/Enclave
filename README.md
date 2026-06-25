# Athernote

> 🔒 **Secure, local-first, zero-knowledge note-taking with P2P sync over local Wi-Fi.**
>
> No cloud. No servers. No internet required.

## Architecture

Athernote is built on three core principles:

### 1. Local-First
All data lives on your device first. The application functions fully offline — create, edit, and organize notes without ever connecting to a network. Sync is an enhancement, not a requirement.

### 2. Zero-Knowledge Encryption
Every note is encrypted with **AES-256-GCM** before it touches persistent storage. Key derivation uses **Argon2id** with strong defaults. The application never sees your plaintext keys — they're derived from credentials that never leave your device.

### 3. Peer-to-Peer Sync
When devices are on the same local network, they discover each other via **mDNS** (Multicast DNS) and sync encrypted notes directly over **WebSocket / WebRTC data channels**. No relay servers, no cloud routing — just device-to-device communication within your Wi-Fi boundary.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Desktop Shell** | Tauri v2 (Rust) |
| **Frontend** | SvelteKit (static adapter / SSG) + Tailwind CSS |
| **Backend** | Rust — Tokio, Axum (local serving) |
| **Storage** | SQLite + sqlcipher (native), IndexedDB / sqlocal (web fallback) |
| **Sync Engine** | Yjs (CRDT for conflict-free text merging) |
| **Network Discovery** | mDNS (multicast DNS) |
| **Transport** | WebSockets / WebRTC data channels |
| **Encryption** | AES-256-GCM + Argon2id |

## Monorepo Structure

```
athernote/
├── src/                    # SvelteKit frontend
│   ├── app.html            # Root HTML shell
│   ├── app.css             # Global styles + Tailwind
│   ├── lib/                # Shared components & utilities
│   └── routes/             # SvelteKit pages
│       ├── +layout.svelte  # App shell (sidebar + main area)
│       ├── +layout.ts      # Prerender config
│       └── +page.svelte    # Main notes view
├── src-tauri/              # Rust backend (Tauri v2)
│   ├── Cargo.toml          # Rust dependencies
│   ├── build.rs            # Tauri build script
│   ├── tauri.conf.json     # Tauri configuration
│   ├── icons/              # App icons
│   └── src/
│       ├── main.rs         # Binary entry point
│       └── lib.rs          # App builder + commands
├── build/                  # Static SvelteKit output (gitignored)
├── package.json            # Node dependencies
├── svelte.config.js        # SvelteKit config (static adapter)
├── vite.config.ts          # Vite + Tailwind config
└── tsconfig.json           # TypeScript config
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
git clone <repo-url> athernote
cd athernote

# Install frontend dependencies
npm install

# Verify Rust toolchain
rustc --version  # should be >= 1.77
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

**Phase 1** — Monorepo scaffolding and Tauri + SvelteKit integration.  
**Next**: CRDT sync engine (Yjs), encrypted storage layer, mDNS peer discovery.

## License

MIT
