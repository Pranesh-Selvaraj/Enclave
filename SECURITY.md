# Security Policy

## Reporting a Vulnerability

**Do not open a public issue.** Report vulnerabilities privately to the maintainers.

Send an email with details (steps to reproduce, affected versions, potential impact) and we'll respond within 72 hours with a plan.

Once a fix is released, we'll publish a security advisory crediting the reporter (if desired).

## Security Model

Enclave is a zero-knowledge, local-first knowledge base. The products on `main` are the native Rust desktop and the native Android app, both on the shared Rust core. The retired Tauri/Svelte app is historical (ADR 0014) and is not part of the current attack surface.

### What We Protect

- **At rest**: page contents are stored in a SQLCipher-encrypted SQLite database — AES-256-CBC with HMAC-SHA512, the SQLCipher defaults. Embeddings, the vec0 ANN index and app settings live in the same encrypted file. Keys are derived from a 12-word BIP39 seed phrase using Argon2id (64 MiB, 3 iterations, 4 parallelism).
- **Key material**: the master key exists only in memory during a session. The only on-disk copy of key material is `vault.key`: the seed phrase re-encrypted with Argon2id (fresh random 32-byte salt) + AES-256-GCM under the password. It is unusable without the password and can be reset by deleting it (unlock with the seed phrase instead).
- **UI boundary**: the desktop consumes `core-api` as a Rust crate; Android consumes it through UniFFI. No webview, no C ABI, no JavaScript runtime. The core validates untrusted document content before it reaches a UI model where practical.

### What We Don't Protect

- The operating system or filesystem itself. If an attacker has root access to your machine, Enclave cannot protect your data.
- Clipboard contents, screenshots or other side channels.
- The seed phrase display during vault creation. Someone looking at your screen at that moment can compromise the vault.
- **Sync traffic on the wire** — fixed: P2P sync is authenticated and encrypted (see below).

### Trust Boundaries

```
Native UI (Rust desktop / Kotlin Compose Android)
        │
        ├── desktop: direct Rust `core-api` dependency
        └── Android: UniFFI/JNA typed boundary
        │
        ▼
Rust core (trusted, same process)
        │
        ├── core-db: SQLCipher at rest
        └── core-network: mDNS + WebSocket, authenticated + sealed
```

The core is the only implementation of crypto, storage, the block schema and sync. Shells own windows, input and presentation only.

## P2P Sync Security

- Peers on the same LAN discover each other via mDNS (`_enclave._tcp.local`) and connect over WebSocket (`ws://`).
- **Mutual authentication before anything else:** every session starts with a challenge-response handshake. Both sides prove knowledge of the vault-derived sync key (HKDF of the vault key, which itself derives from the BIP39 seed phrase) with HMAC-SHA256 proofs over fresh random challenges + peer ids. A peer that can't prove the key is disconnected at the handshake — **no data, not even the hello, is exchanged**.
- **Transport encryption:** after auth, every frame is sealed with XChaCha20-Poly1305 (AEAD) under a per-session key (HKDF of the sync key + both challenges). Random 24-byte nonce per frame; forged or tampered frames fail the MAC and kill the session.
- **Ownership = the seed phrase.** Two devices with the same vault (same BIP39 seed phrase) derive the same sync key and can sync. Devices with a different vault are rejected — even though they share the Wi-Fi and can see the traffic, they only see ciphertext. mDNS still advertises "an Enclave is here", but discovery alone yields nothing.
- **Replay protection:** challenges are fresh per connection, so a recorded handshake cannot be replayed.
- Remaining caveat: sync assumes a LAN you can *reach* — mDNS and the WebSocket port are visible to the network, so an attacker with full control of the router could mount a MITM at the TCP layer. Do not sync across an untrusted network.

## AI Data Boundary

AI is **not yet shipped in the native apps**. When it lands, the design is:

- opt-in and off by default;
- local embeddings/retrieval by default, with remote providers requiring explicit per-provider consent;
- the exact context sent to a remote provider shown to the user;
- the AI service never receives the vault key; the app executes agent tools after policy checks and approvals;
- no network, shell or filesystem tools for the model; no automatic writes;
- no telemetry and no logging of prompts, document contents or keys.

Until that implementation ships, do not assume any AI data path exists in the native apps. The retired Tauri AI client is not part of the current product.

## What This Means in Practice

Enclave protects data **at rest** and keeps it **off the cloud** — there is no Enclave server, no telemetry and no account. The remaining exposure is the LAN during sync and whatever endpoint a future explicit AI opt-in chooses. Both are operator choices; keep sync on networks you control and point AI only at endpoints you trust.

## Update Integrity

Auto-updates are strictly opt-in and never run unattended. The native apps will use the core's verifier: the asset is checked against GitHub's reported size and SHA-256 digest, and every release asset also carries a detached Ed25519 signature (`<asset>.sig`) verified against the pinned public key in `core/core-api/src/updater.rs`. A missing or invalid signature deletes the download and refuses installation. The private signing key lives in the `UPDATE_SIGNING_KEY` CI secret; `scripts/sign-release.mjs` signs the assets and publishes `SHA256SUMS` + `SHA256SUMS.sig` for manual installs.

Native release pipelines are being rebuilt ([#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241)). Until then, no new release is published; historical Tauri assets are not covered by the native update policy.

## Known Dependency Advisories

### `glib` < 0.20.0 — GHSA-wrw7-89jp-8q8g (historical)

This advisory affected the retired Tauri Linux build transitively through `tauri → gtk → glib`. ADR 0014 removed the Tauri shell and `gtk`/`webkit2gtk`/`wry` from `main`, so the advisory is no longer in the current dependency graph. Any alert against an old Tauri tag is historical and not a current product issue.

The native desktop uses `eframe/egui/winit/wgpu`; the native Android app uses Compose and the Rust core. New dependency advisories are evaluated against those graphs.

## Supported Versions

Only the latest native release receives security patches. Historical Tauri releases are not supported. There are no LTS releases.

## Disclosure Timeline

1. Reporter submits vulnerability privately.
2. Maintainer acknowledges within 72 hours.
3. Fix is developed and tested.
4. Release is published with a security advisory.
5. Public disclosure 30 days after fix, or sooner by mutual agreement.
