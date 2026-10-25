# Enclave design docs — Figma plugin

This development plugin writes the repository's design documentation into Figma as native, editable layers. It is generated from the repository, so the documentation cannot drift from the tokens or the brand vectors.

## Build

```bash
node docs/design/figma/build.mjs
```

Reads `docs/design/tokens.json` and `docs/brand/lockup.svg`, `lockup-dark.svg`, `logo.svg`, `watermark.svg`, `watermark-dark.svg`, and rewrites `code.js`. **Do not hand-edit `code.js`** — edit the template or the sources and rebuild.

## Import and run

1. Figma **desktop app** → open the target file (edit access).
2. **Plugins → Development → Import plugin from manifest…** → choose `docs/design/figma/manifest.json`.
3. **Plugins → Development → Enclave design docs**.

It creates two pages and never edits existing ones (re-running creates numbered pages, so earlier edits are safe):

- **Enclave — Brand** — lockups (paper/ink), app icon, watermarks, dark/light palettes, type scale, usage rules.
- **Enclave — Current UI** — the gate as implemented:
  - `Intro / Logo` — the official lockup centred, glow
  - `Register / PIN` — minimal form, PIN dots, Create vault, bottom note
  - `Register / PIN / Keypad` — the keypad revealed, content lifted
  - `Register / Confirm` — “Confirm your PIN”, keypad still up
  - `Register / Confirmed` — “PIN confirmed”, keypad gone, Create enabled
  - `Register / Password` — password fields
  - `Register / PIN (light)` — light palette
  - `Login / PIN`, `Login / PIN / Keypad`, `Login / Password`
  - `Seed phrase` — 2 words per row, centred

The prototype (best-effort) connects `Intro → Register / PIN` after 900 ms with smart animate, tapping the PIN label reveals the keypad, the switch moves between PIN and Password, and Create goes to the seed phrase.

## Notes

- Install **Inter** and **JetBrains Mono** in Figma for exact type parity; the plugin falls back to Roboto if Inter is unavailable.
- The seed words in the Figma frame are placeholders, not a valid phrase.
- The watermark SVGs are already at 6% opacity; they are for backgrounds and documentation only.
- The Figma API token used to pull designs back (for review/implementation) lives outside the repo in `~/.enclave-dev/figma.env`; only the documented screenshots are committed.

## Pulling a design back

```bash
~/.enclave-dev/figma_pull.py --list
~/.enclave-dev/figma_pull.py
# PNGs + manifest + prototype transitions land in ~/.enclave-dev/design/
```
