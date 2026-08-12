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
- ~~**Sync traffic on the wire.**~~ **Fixed:** P2P sync is now authenticated + encrypted — see [P2P Sync Security](#p2p-sync-security).

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
   AES-256-CBC at rest     AUTH + XChaCha20-Poly1305 in transit
```

## P2P Sync Security

- Peers on the same LAN discover each other via mDNS (`_enclave._tcp.local`) and connect over WebSocket (`ws://`).
- **Mutual authentication before anything else:** every session starts with a challenge-response handshake. Both sides prove knowledge of the vault-derived sync key (HKDF of the vault key, which itself derives from the BIP39 seed phrase) with HMAC-SHA256 proofs over fresh random challenges + peer ids. A peer that can't prove the key is disconnected at the handshake — **no data, not even the hello, is exchanged**.
- **Transport encryption:** after auth, every frame is sealed with XChaCha20-Poly1305 (AEAD) under a per-session key (HKDF of the sync key + both challenges). Random 24-byte nonce per frame; forged or tampered frames fail the MAC and kill the session.
- **Ownership = the seed phrase.** Two devices with the same vault (same BIP39 seed phrase) derive the same sync key and can sync. Devices with a different vault are rejected — even though they share the Wi-Fi and can see the traffic, they only see ciphertext. mDNS still advertises "an Enclave is here", but discovery alone yields nothing.
- **Replay protection:** challenges are fresh per connection, so a recorded handshake cannot be replayed.
- Remaining caveat: sync still assumes a LAN you can *reach* — mDNS and the WebSocket port are visible to the network, so an attacker with full control of the router could mount a MITM at the TCP layer. Do not sync across an untrusted network.

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
