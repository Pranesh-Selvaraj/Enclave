# Security Policy

## Reporting a Vulnerability

**Do not open a public issue.** Report vulnerabilities privately to the maintainers.

Send an email with details (steps to reproduce, affected versions, potential impact) and we'll respond within 72 hours with a plan.

Once a fix is released, we'll publish a security advisory crediting the reporter (if desired).

## Security Model

Enclave is a zero-knowledge, local-first note-taking application. The security model rests on these guarantees:

### What We Protect

- Note contents are encrypted at rest (AES-256-GCM via SQLCipher).
- Keys are derived from a 12-word BIP39 seed phrase using Argon2id (64 MiB, 3 iterations, 4 parallelism).
- Key material exists only in memory during a session. Never written to disk.
- P2P sync messages are encrypted before leaving the device.
- The app makes zero outbound network requests. All communication is LAN-only.

### What We Don't Protect

- The operating system or filesystem itself. If an attacker has root access to your machine, Enclave cannot protect your data.
- Clipboard contents, screenshots, or other side channels.
- Metadata (document titles, creation dates, block structure) — these are stored in the encrypted database but not individually encrypted.
- The seed phrase display during vault creation. Someone looking at your screen at that moment can compromise the vault.

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
   Encrypted disk           LAN-only, encrypted transport
```

### P2P Sync Security

- Peers on the same LAN discover each other via mDNS (`_enclave._tcp.local`).
- All sync data is encrypted with a transport key derived from the vault key before leaving the device.
- Peers exchange only encrypted CRDT update blobs. A peer without the seed phrase cannot decrypt received data.
- **No authentication.** Any device on the same LAN can connect — the encryption layer is the only access control. This is by design: the threat model assumes a trusted LAN.

## Supported Versions

Only the latest release receives security patches. There are no LTS releases.

## Disclosure Timeline

1. Reporter submits vulnerability privately.
2. Maintainer acknowledges within 72 hours.
3. Fix is developed and tested.
4. Release is published with a security advisory.
5. Public disclosure 30 days after fix, or sooner by mutual agreement.
