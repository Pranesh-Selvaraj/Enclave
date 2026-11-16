# Plan: dedicated native desktop + Android apps

Companion to ADR 0002. This is the execution plan for moving Enclave from the shared-web-UI architecture (ADR 0001) to two dedicated native apps on one versioned Rust core.

## Status

Live tracker: [`docs/STATUS.md`](../STATUS.md).

- **Phase 0** — the Android vertical slice is built; the desktop stack is now **fully Rust** (ADR 0013) and its editor-engine gate is open. Qt/C++ (ADR 0004) is superseded.
- **Phase 1** — complete: vault schema versioning, the block golden fixture, sync-frame golden fixtures (transport frames, app frames and handshake crypto) and [`docs/CONTRACT.md`](../CONTRACT.md). What remains is the Phase 4 interop CI that runs the fixtures from the app builds.
- **Phase 2** — the native Android app is feature-complete for a first release; the gate is verified on a real phone, the remaining hardware checklist is pending.
- **Phase 3/4** — the Rust desktop foundation is in progress; parity and rollout are not started.

## Goal

Ship a native Android app and a native desktop app that share no UI code, both built on the existing Rust core, with identical vaults and a wire-compatible sync protocol. Remove the webview from the shipping product.

## Invariants (must never change)

- The Rust core (`core-db`, `core-network`, `core-api`) is the only implementation of crypto, storage, the block schema and sync.
- The contract in ADR 0006 is versioned and tested; desktop and Android must always interoperate.
- No cloud, no accounts, no telemetry: the product's mission is unchanged.
- Users' existing vaults open in every new app; migrations are one-way and tested.

## Non-goals

- A shared UI framework or shared components between desktop and Android.
- iOS (the contract must not preclude it, but it is not in this plan).
- Rewriting the vault, crypto or sync protocol. The core is reused; the UI is replaced.

## Phases

Phases are criteria-driven, not calendar-driven: a phase ends when its exit criteria are met, and the next begins as soon as its dependencies are complete.

```mermaid
graph LR
    P0["Phase 0<br/>Spike and stack decision"] --> P1["Phase 1<br/>Core extraction and contract"]
    P1 --> P2["Phase 2<br/>Native Android app"]
    P1 --> P3["Phase 3<br/>Native desktop app"]
    P2 --> P4["Phase 4<br/>Parity, interop, rollout"]
    P3 --> P4
```

### Phase 0 — Spike and stack decision

**Deliverables**

- A fully Rust desktop foundation: `apps/desktop` consuming `core-api` directly, a vault gate, a notes list and a shell skeleton. No C ABI, no C++.
- An in-house headless block-editor prototype covering: paragraphs, headings, lists, tasks, tables, images, code blocks, undo/redo, IME, keyboard navigation, Markdown paste, and a screen-reader pass. ProseMirror JSON is the boundary.
- Packaging, signing and auto-update on Linux first, then Windows/macOS; the core's Ed25519 verifier is the shared gate.
- An Android vertical slice: Compose app + `core-api` via UniFFI, opening a vault, listing notes, editing one document, and running background sync through a foreground service; one Glance widget.
- A written measurement table: build time, binary size, cold start, editor latency, memory, and developer effort. The Avalonia/Qt evidence is kept under `~/.enclave-dev/phase0-native-apps/` for the record.

**Exit criteria**

- The Rust editor prototype loads the canonical block fixture, edits it, exports ProseMirror JSON and round-trips it through the core.
- The editor meets the usability bar: typing latency on a 2,000-block document, IME, table/task keyboard editing, and an AccessKit screen-reader pass.
- The Rust shell consumes `core-api` directly and opens a vault; no C ABI exists.
- A Linux package and the Ed25519 update-verification path are proven.
- The Android slice proves the UniFFI path end to end on a real device.
- The contract tests exist as a skeleton (vault fixture, one sync frame fixture).

### Phase 1 — Core extraction and contract

**Deliverables**

- Core moved to `core/` per ADR 0003, with the current Tauri app still building and shipping.
- A stable app boundary in `core-api`: UniFFI bindings for Android, and a direct Rust crate dependency for the desktop. There is no C ABI (ADR 0013).
- An explicit vault schema version and ordered migration chain (ADR 0006).
- Golden fixtures and interop tests: canonical vault, block-schema documents, sync frames; both app stacks build against the same core SHA.
- The compatibility matrix (app ↔ core ↔ protocol ↔ schema) published in `docs/`.

**Exit criteria**

- `cargo test` plus interop fixtures pass for core + both app stubs.
- The Tauri app still reads and writes the same vaults, unchanged, against the new core layout.
- Contract documentation is complete enough for an app developer to build against without reading core internals.

### Phase 2 — Native Android app

**Deliverables**

- Compose app: unlock/create vault, notes layouts (List, Gallery, Cards, Icons), sort, search, folders, tags, editor (the Phase 0 prototype hardened), settings, sync UI.
- Native integration: foreground sync service, Glance widgets (notes list, pinned note, quick capture), share target, launcher shortcuts, Quick Settings tile, Keystore-backed widget cache, SAF import/export/backup, insets.
- Migrate an existing v1.13.0 vault without data loss; sync against the current Tauri app.

**Exit criteria**

- Feature parity with the current Android build of the Tauri app on a published checklist.
- Background sync survives restart and screen-off on a real device; battery measurements recorded.
- TalkBack pass on primary flows; widgets render while locked.

### Phase 3 — Fully Rust desktop app (can run alongside Phase 2 once Phase 1 is complete)

**Deliverables**

- Rust shell: vault flow, sidebar and folder tree, editor, whiteboard, database views, graph, split view, command palette, settings, tray/wallpaper widget, updates.
- Headless editor engine: ProseMirror JSON model, transactions/selection/history, layout/display list, IME and AccessKit, with a desktop view adapter. The engine has no UI-framework dependency and is the open-source candidate.
- AI service: local embeddings and retrieval, Q&A with citations, opt-in remote providers, and a read-only agent with app-enforced tool permissions.
- Platform integration per OS: menus, file dialogs, drag-and-drop, notifications, global shortcuts, signing/notarisation, installers (`.deb`/`.AppImage`, `.msi`/NSIS, `.dmg`).
- Migration of an existing vault; sync against both the Tauri app and the native Android app.

**Exit criteria**

- Feature parity with the desktop Tauri app on a published checklist.
- Screen-reader pass on primary flows; keyboard-only navigation works; IME works across CJK and combining sequences.
- Auto-update path verified on all three OSes, including signature verification.
- No C ABI and no C++ in the desktop product.

### Phase 4 — Parity, interoperability, rollout

**Deliverables**

- Cross-app interop matrix run in CI: Tauri ↔ native Android ↔ native desktop, all combinations, for the sync protocol and vault format.
- Release pipelines for both native apps (signing, versioning, changelog, release notes).
- Rollout: native apps ship alongside Tauri; Tauri is deprecated and then removed once telemetry-free crash/issue reports show parity.

## Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Editor rebuild proves larger than planned | Highest — blocks both apps | Phase 0 prototype with a hard go/no-go; treat the editor as its own subproject with its own milestones. |
| Contract drift between apps | Sync breakage, data loss | ADR 0006: versioned surfaces, golden fixtures, interop CI against one core SHA. |
| The Rust editor engine proves larger than planned | Highest — blocks the desktop | Phase 0 fixture/IME/accessibility gate before the shell grows; treat the engine as its own subproject; canvist is a measured fallback/reference (ADR 0013). |
| Rust UI ecosystem churn | Rework in the shell | Keep the editor engine and core UI-framework-independent; pin and isolate the shell; prefer stable crates. |
| Vault migration failure | Data loss | One-way migrations tested against canonical fixtures from every released version; never ship a migration without a round-trip test. |
| Parity gap (whiteboard, graph, database) | Native apps feel incomplete | Publish a parity checklist per app; do not deprecate Tauri until it is fully green. |
| Three apps during transition | Maintenance load, confusion | Bound the transition by parity criteria; keep `main` as the source of truth and release Tauri from its own tags. |
| Android background/battery restrictions | Sync reliability | Foreground service with visible state; measure on real devices; fall back to manual sync when restricted. |
| Editor engine maintenance and open-source support | Long-term load | Budget maintainers, fuzzers and API stability; extract only after the desktop editor slice passes (ADR 0013). |

## Success criteria

- Native Rust desktop and native Android apps pass parity checklists and ship signed releases.
- A v1.13.0 vault opens in both new apps and syncs with an unmodified Tauri app.
- The Tauri app can be retired with no loss of features or data.
- Cross-device sync is proven by CI interop tests, not by manual testing alone.
- No browser engine, no C ABI and no C++ ship in the product.
