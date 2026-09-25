# @enclave/sync-engine (standalone)

**This package is not used by the Enclave app.** It is an isolated Yjs CRDT
experiment.

The shipped sync is entirely in Rust:

| Concern | Where |
|---|---|
| Storage + doc-level LWW merge | `src-tauri/crates/core-db` |
| mDNS discovery + authenticated/encrypted WebSocket transport | `src-tauri/crates/core-network` |
| Sync orchestration (snapshots, incremental digest diff) | `src-tauri/src/lib.rs` |

Nothing imports `@enclave/sync-engine`; it is not a dependency of the
frontend. Keep it out of production code unless the architecture changes
deliberately — see CONTRIBUTING.md ("Standalone Yjs CRDT library. NOT used for
app sync").

Tests (manual; run from the repo root):

```bash
npx tsx packages/sync-engine/test.ts
npx tsx packages/sync-engine/stress.test.ts
```
