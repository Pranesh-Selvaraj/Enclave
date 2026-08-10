# Security Policy

## Reporting a Vulnerability

**Do not open a public issue.** Report vulnerabilities privately to the maintainers.

Send an email with details (steps to reproduce, affected versions, potential impact) and we'll respond within 72 hours with a plan.

Once a fix is released, we'll publish a security advisory crediting the reporter (if desired).

## Security Model

Enclave is a zero-knowledge, local-first knowledge base. The security model rests on these guarantees:

### What We Protect

- **At rest**: page contents are stored in a SQLCipher-encrypted SQLite database — AES-256-CBC with HMAC-SHA512, the SQLCipher defaults. Embeddings, the vec0 ANN index, and AI settings (incl. API keys) live in the same encrypted file. Keys are derived from a 12-word BIP39 seed phrase using Argon2id (64 MiB, 3 iterations, 4 parallelism), a memory-hard KDF resistant to GPU/ASIC attacks.
- **Key material**: the master key exists only in memory during a session. The only on-disk copy of key material is `vault.key`: the seed phrase re-encrypted with Argon2id (fresh random 32-byte salt) + AES-256-GCM under your password. It is unusable without the password and can be reset by deleting it (unlock with the seed phrase instead).

### What We Don't Protect

- The operating system or filesystem itself. If an attacker has root access to your machine, Enclave cannot protect your data.
- Clipboard contents, screenshots, or other side channels.
- The seed phrase display during vault creation. Someone looking at your screen at that moment can compromise the vault.
- **Sync traffic on the wire.** P2P sync sends **plaintext** JSON snapshots over LAN WebSockets (see [P2P Sync Security](#p2p-sync-security)).

### Trust Boundaries

```
User input ──► SvelteKit frontend (trusted, same process)
                      │
                      ▼
              Tauri command bridge (IPC boundary)
                      │
                      ▼
              Rust backend (trusted, same machine)
                      │
        ┌─────────────┴─────────────┐
        ▼                           ▼
   core-db (SQLCipher)     core-network (mDNS + WebSocket)
        │                           │
        ▼                           ▼
   AES-256-CBC at rest     LAN-only, PLAINTEXT transport
```

## P2P Sync Security

- Peers on the same LAN discover each other via mDNS (`_enclave._tcp.local`) and connect over plaintext WebSocket (`ws://`).
- Each side exchanges a full JSON snapshot (`{kind:"snapshot", docs, blocks}`), merged with doc-level last-write-wins. Dropped peers are redialed every 3 s.
- **Sync data is not encrypted in transit.** Any device on the same network can connect and read synced content. **The threat model assumes a trusted LAN** — do not sync across an untrusted network (e.g. a public or hotel Wi-Fi).
- There is no peer authentication or handshake secret. A LAN device that can reach the WebSocket port can join a sync session.

## Local AI / LLM Data Boundary

The AI assistant is **opt-in** and **off by default**. When enabled in **Settings → AI assistant**:

- Page content is sent to the endpoint you configure — for embedding on save (`POST /v1/embeddings`) and as retrieval context for questions (`POST /v1/chat/completions`).
- The CSP allows `https:` plus `localhost:*` (and the app's own origin). Content only leaves the machine if you run a local LLM server (Ollama, llama.cpp, LM Studio, vLLM) or point the AI feature at a remote https endpoint with an API key.
- **Offline embeddings** (built-in ONNX model, on by default when enabled): retrieval vectors are computed on-device and the embedding step makes no network request at all — only chat still talks to the endpoint.
- Settings (URL, model, enabled, RAG, **API key**) are stored **encrypted at rest** in the vault's `settings` table (SQLCipher), not in localStorage. A pre-1.1 localStorage copy is migrated once on first load and then removed.

## What This Means in Practice

Enclave protects data **at rest** and keeps it **off the cloud** — there is no Enclave server, no telemetry, and no account. The remaining exposure is the LAN during sync, and whatever endpoint you choose to send content to if you enable the AI feature. Both are operator choices; keep sync on networks you control and keep AI pointed at endpoints you trust.

## Supported Versions

Only the latest release receives security patches. There are no LTS releases.

## Disclosure Timeline

1. Reporter submits vulnerability privately.
2. Maintainer acknowledges within 72 hours.
3. Fix is developed and tested.
4. Release is published with a security advisory.
5. Public disclosure 30 days after fix, or sooner by mutual agreement.
