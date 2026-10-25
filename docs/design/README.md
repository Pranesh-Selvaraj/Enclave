# Design system and sources

This directory is the repository side of Enclave's design work: the tokens, the brand assets, the Figma replicator, and the exported screens used for documentation. Together they answer three questions without opening a chat: **what are the rules, where is the source, and how does the design get into the product?**

## Layout

| Path | Purpose |
| --- | --- |
| [`tokens.json`](tokens.json) | Canonical colours, type scale, layout and radii. The Figma plugin is generated from this file; the Compose theme mirrors it. |
| [`figma/`](figma/README.md) | Development plugin that writes the brand and current-UI pages into Figma, plus its build script. |
| [`../brand/`](../brand/README.md) | The brand SVGs: mark, lockup, icon, watermark — the source of every logo in the product. |

## Where the brand files live

- **Vectors**: `docs/brand/mark.svg`, `lockup.svg`, `lockup-dark.svg`, `logo.svg`, `watermark.svg`, `watermark-dark.svg`. These are the source of truth; the Android vector drawables are generated from the lockups, not redrawn.
- **Fonts**: Inter and JetBrains Mono are bundled (SIL OFL) in `apps/frontend/static/fonts` and converted to Android `res/font` for the native app. Running text is always Inter; code is always JetBrains Mono.
- **Watermark**: `watermark.svg` (ink at 6%) and `watermark-dark.svg` (paper at 6%) are for backgrounds and documentation, never over body text or at full contrast.

## The Figma documentation

The plugin in [`figma/`](figma/README.md) creates two pages in a Figma file:

- **Enclave — Brand**: the lockups, app icon, watermarks, palette, type scale and usage rules.
- **Enclave — Current UI**: the gate as it is implemented — intro, register PIN (closed/keypad/confirm/confirmed), password, login, seed phrase — with the keypad slide and the logo-to-form transition prototyped.

The plugin is **generated** from the repository: `build.mjs` reads `tokens.json` and the brand SVGs, then writes `figma/code.js`. Do not hand-edit `code.js`; edit the template, tokens or SVGs and rebuild.

```bash
cd docs/design/figma
node build.mjs          # rewrites code.js from tokens.json + docs/brand/*.svg
```

Then in the Figma desktop app: **Plugins → Development → Import plugin from manifest…** and pick `docs/design/figma/manifest.json`. Running it always creates new pages, so earlier pages and your edits are never overwritten.

For reading a design back, the local puller (outside the repo) exports frames and prototype transitions:

```bash
# ~/.enclave-dev/figma.env holds FIGMA_TOKEN (read-only) and FIGMA_FILE
~/.enclave-dev/figma_pull.py --list
~/.enclave-dev/figma_pull.py
```

Exports land in `~/.enclave-dev/design/`; only the curated images for documentation go into `docs/design/screens/`. Tokens and API keys never enter the repository.

## Keeping tokens honest

The intended contract is one token file for both sides:

1. `tokens.json` is the canonical value set.
2. The Figma plugin is generated from it, so the documentation cannot drift from the values.
3. `apps/android/.../ui/Theme.kt` mirrors it today; the next step is a small generator (or a test) that compares the Compose colour/type constants with `tokens.json`, so a token change fails CI until both sides agree.

Until that check exists, treat a token change as a two-file change: `tokens.json` **and** the Compose theme, in the same commit.

## Rules of thumb

- Figma is for exploration and review; the repository is the record. Export final screens, link the Figma file from the relevant PR/ADR, and keep the decisions in `docs/HISTORY.md` / `docs/decisions/`.
- Brand geometry is never retyped; the wordmark is built from rectangles and the lockup SVG is the source.
- New screens use the tokens, not ad-hoc hex values. If a value is missing, add it to `tokens.json` first.
- Documentation screenshots are optimised (phone width, reasonable file size). Working files, Figma exports and throwaway analysis images stay in `~/.enclave-dev/`.
