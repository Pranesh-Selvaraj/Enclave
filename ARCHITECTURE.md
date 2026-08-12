# Enclave Architecture

Secure, local-first, zero-knowledge knowledge base. One codebase serves
desktop (Tauri) and Android (Tauri mobile) with the same frontend and the
same Rust core.

## Runtime topology

```
┌────────────────────────── WebView / window ──────────────────────────┐
│  SvelteKit frontend (Svelte 5, static build)                         │
│  - TipTap editor, vault guard, AI panel, sync toggle                 │
│  - @enclave/crypto: BIP39 + Argon2id + AES-256-GCM (vault.key only)  │
└──────────────┬───────────────────────────────────────────────────────┘
               │ Tauri IPC (invoke / events) — the only boundary
┌──────────────▼───────────────────────────────────────────────────────┐
│  Rust backend (src-tauri) — trusted, same process                    │
│                                                                      │
│  commands (src/lib.rs)                                               │
│   ├── vault lifecycle   init/unlock/lock, key held while unlocked    │
│   ├── document/block CRUD, FTS search, backlinks, favorites          │
│   ├── embeddings (RAG)  embed_text (gated on x86_64-android)         │
│   └── network           start/stop/status, sync message handling     │
│                                                                      │
│  core-db ──── SQLCipher (encrypted SQLite) + FTS5 + sqlite-vec       │
│  core-network ─ mDNS discovery + WebSocket sessions (auth + AEAD)    │
└──────────────────────────────────────────────────────────────────────┘
```

- The frontend never touches the database or the network directly — everything
  goes through Tauri commands (see `apps/desktop/src/lib/backend.ts`).
- `AppState` holds the open DB connection and the vault-derived sync key;
  both exist only while the vault is unlocked (`lock_vault` drops both and
  stops the network).

## Keys & encryption

```
BIP39 seed phrase (12 words — the ownership token, same on all your devices)
   │  Argon2id (64 MiB, t=3, p=4)
   ▼
vault key (256-bit) ──► SQLCipher at rest (AES-256-CBC + HMAC-SHA512)
   │
   └── HKDF-SHA256("enclave-sync-v1") ──► sync key
                                          ├─ mutual auth (HMAC-SHA256 proofs)
                                          └─ transport (XChaCha20-Poly1305)
```

- **At rest**: one SQLCipher file (`enclave.db`) holds documents, blocks,
  settings (incl. AI API keys), embeddings and the ANN index — one encryption
  boundary.
- **vault.key**: the seed phrase re-encrypted with `Argon2id(password) +
  AES-256-GCM` so later unlocks only need the password. Deleting it falls
  back to seed-phrase unlock.
- **In transit** (LAN only): challenge-response handshake over the sync key —
  wrong key = disconnected before any data flows — then every frame sealed
  with XChaCha20-Poly1305 under a per-session key (HKDF of sync key + both
  challenges). Fresh random challenges per connection (replay-safe).
  See `crates/core-network/src/crypto.rs`.

## Data model

- `documents` (id, title, icon, timestamps, archived, favorite) and `blocks`
  (id, doc, type, content, `sort_order`) — a block tree per document.
- `updated_at` + a monotonic per-document edit counter are the **LWW clock**
  for sync; deletes are tombstones.
- FTS5 index for full-text search; `vec0` tables for the ANN embedding index.

## P2P sync protocol

1. **Discovery** — mDNS advertises `_enclave._tcp.local.` on the LAN;
   devices dial every discovered peer.
2. **Session** (per connection, `core-network/src/ws.rs`):
   `auth` (challenge) → `auth_proof` (HMAC over both challenges + peer id,
   constant-time verified) → `hello` (peer id + device name) → app payloads.
   All frames after the handshake are encrypted binary frames.
3. **App payloads** (`src/lib.rs`):
   - `snapshot` — full JSON of docs + blocks (sent on hello; merged with
     doc-level LWW)
   - `ack` — peer confirms a merge; both sides update "last synced at"
   - sync progress is emitted to the UI as `sync-done` events.
4. **Resilience** — dead sessions are redialed every 3 s from the last known
   host:port (mDNS doesn't reliably re-fire for a known service).

## RAG pipeline

- Offline embeddings: `fastembed` (ONNX `all-MiniLM-L6-v2`, ~25 MB model
  downloaded once into the app data dir; inference runs in-process).
- Vectors land in per-dimension `vec0` tables inside the encrypted DB;
  retrieval uses ANN candidates with a recall buffer, re-ranked by exact
  cosine; exact-scan fallback for dimensions without an index.
- Chat (optional, off by default) goes to a user-configured OpenAI-compatible
  endpoint; settings incl. API keys live encrypted in the vault.
- **Platform gate**: ONNX Runtime ships no prebuilt binaries for
  `x86_64-linux-android` (emulator-only), so local embeddings compile out
  there via target cfg — arm64 phones and desktops keep RAG.

## Platforms

| Platform | Shell | Notes |
|---|---|---|
| Linux / Windows / macOS | Tauri v2 (wry) | system tray + quick-capture window (`cfg(desktop)`) |
| Android (arm64) | Tauri mobile (system WebView) | full feature set incl. RAG |
| Android emulator (x86_64) | same | RAG gated out (see above) |

Platform-specific Rust: tray/menu code is `#[cfg(desktop)]`; SQLCipher links
system OpenSSL on desktop and vendored OpenSSL on Android; TLS is rustls
everywhere (no OpenSSL in the Rust dependency graph).

## Release engineering

- CI: `.github/workflows/build.yml` (tests + desktop bundles),
  `.github/workflows/android.yml` (arm64 APK/AAB, signed with
  `ANDROID_KEYSTORE_*` secrets, artifacts uploaded).
- Signing: `scripts/android-sign.sh` (zipalign + apksigner / jarsigner);
  keystore lives outside the repo and must be backed up off-machine.
- Android `versionCode` auto-increments per build (`autoIncrementVersionCode`,
  counter tracked in `tauri.properties`).
- Full release checklist: see **Releasing** in CONTRIBUTING.md.
- Issue log + mobile build guide: `docs/android-mobile.md`.
