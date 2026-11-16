# 0014. Native-only repository: retire the Tauri/Svelte app

- **Status:** Accepted — native-only `main` (2026-11-16)
- **Date:** 2026-11-16
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** the Tauri transition policy in 0001/0002/0013 (the architecture decisions themselves stay for the record)
- **Superseded by:** —

## Context

Enclave has been shipping a Tauri v2 + SvelteKit application on desktop and Android while the dedicated native apps were built. The repository now contains both implementations: the legacy webview app (`apps/frontend`, `packages/*`, `src-tauri`) and the native apps (`apps/android`, `apps/desktop`) on one Rust core.

The desktop direction is now fully Rust (ADR 0013): a Rust UI shell and an in-house headless block editor engine, with no C ABI and no webview. The Android direction is Kotlin + Jetpack Compose (ADR 0005). Keeping the Svelte/Tauri app in `main` has three costs:

1. Every UI/editor change can be made twice and drift.
2. The repository continues to carry a browser engine, Node/JS toolchains and a Tauri release pipeline that the product is trying to remove.
3. The feature baseline for the new desktop is spread across the Svelte app, its packages and the Tauri shell instead of being a single migration checklist.

The old app is still useful as a **feature reference** and as the compatibility baseline for vaults created by released versions. That is preserved by Git history, the release tags and the contract fixtures — not by keeping it buildable from `main`.

## Decision

`main` becomes **native-only**:

- Keep:
  - `core/` — the Rust core, the only shared implementation of vault, crypto, storage, search, sync and updates.
  - `apps/desktop/` — the fully Rust desktop.
  - `apps/android/` — the Kotlin + Jetpack Compose native Android app.
  - `docs/`, `site/`, brand assets, mobile screenshots and native tooling.
- Remove from `main`:
  - `apps/frontend/` (SvelteKit frontend).
  - `packages/` (Svelte/TypeScript editor, UI, crypto and sync-engine packages).
  - `src-tauri/` (Tauri desktop and Tauri Android shell, including `gen/android`).
  - Tauri/Node CI workflows and Tauri-specific release scripts, screenshots and generated assets.
  - The root `package.json` / npm workspace, once the site build no longer needs it.
- Preserve:
  - The historical code in Git commits and release tags.
  - Historical Tauri release artifacts and the v1.15.0 feature baseline.
  - ADRs, CHANGELOG and HISTORY. Do not rewrite history; mark relationships in new records.
- Replace “parity with the Tauri app” as an implicit target with an explicit artifact:
  - `docs/plans/desktop-feature-parity.md` lists every retired-desktop feature and its status in the Rust desktop.
  - Every not-yet-delivered item is linked to an issue. The checklist is the migration contract.
- The native apps must still open and sync Tauri-created vaults. The vault schema, block JSON and sync protocol are unchanged (ADR 0006); Tauri is a historical peer, not a build target.
- Release pipelines are re-established for the native apps. Until the native release jobs exist, tag releases pause; the existing Tauri release tags remain installable.

## Consequences

**Easier / stronger**

- One desktop implementation and one Android implementation; no webview, Node runtime or Svelte toolchain in the product tree.
- The repository’s shape matches the product: core, native Android, native desktop.
- The feature migration is auditable through the parity checklist rather than tribal knowledge.
- CI and contributor setup become Rust/Android-first; the site build is plain Node.

**Harder / accepted costs**

- **Release gap.** Deleting `src-tauri` removes the current Tauri Android build and the tag release workflow. Native Android signing/publishing and Rust desktop packaging/updates must be delivered before the next release. Existing releases are not affected.
- **Reference loss in the working tree.** The Svelte implementation is no longer browsable in `main`; use the historical commits/tags (`v1.15.0` and earlier) and the parity checklist. This is deliberate: `main` is not an archive.
- **Migration effort is now explicit.** Whiteboard, databases, graph, split view, AI, widgets, import/export, platform integration and accessibility are parity items, not assumptions.
- **Docs and site copy must stop implying Tauri remains current.** The site keeps historical download links but labels them as legacy until native releases exist.
- **No Tauri interop CI.** Native apps must still open canonical Tauri vault fixtures; the interop matrix is native apps plus historical fixtures, not a third live build.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Keep Tauri desktop and Svelte code in `main` | Two desktop implementations and a browser engine; contradicts ADR 0013 and keeps the migration ambiguous. |
| Keep only Tauri Android / Svelte frontend temporarily | The frontend and shell are shared; a temporary mobile-only fork would be significant surgery and still leaves a webview/Node stack in the product tree. The native Android app is the planned product. |
| Move the Tauri app to a `legacy/` directory | `main` would still carry a buildable browser-engine app and its dependencies; historical tags already preserve it. |
| Delete the history too | Loses the feature reference, vault compatibility evidence and the decision trail. ADRs/CHANGELOG/HISTORY are product memory. |
| Cut a final Tauri release first | Release and tagging are separate authorized steps; the existing tags remain installable. A final release can still be cut from an old tag if the user chooses. |

## Prior art / what this replaces

ADR 0002 kept Tauri shipping until the native apps reached parity; ADR 0013 moved the desktop to Rust but retained Tauri as the shipping fallback. This record removes that fallback from `main`. The safety net is now Git history plus the explicit parity checklist, not a second buildable app. The target state remains the one the project already chose: **no browser engine in the product**, one audited core, native desktop and Android.
