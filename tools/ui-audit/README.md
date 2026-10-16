# UI audit

Opt-in screenshot + DOM-audit suite for the Enclave frontend. It renders the
real SvelteKit app in headless Chromium with the Tauri IPC stubbed (fake vault:
pages, folders, tags, trash, whiteboard, sync card), walks scenarios at
desktop and phone form factors, and records layout/UX findings in
`manifest.json` (overflow, off-screen controls, clipped text, undersized
touch targets, broken settings persistence, palette search, …).

Nothing here ships in the app; it is test tooling only. There are no npm
dependencies — it drives Chrome over CDP with Node's built-in
`fetch`/`WebSocket`.

## Run

```bash
npm run audit:ui                                   # desktop+phone, all scenarios
node tools/ui-audit/run.mjs --profiles phone-small --scenario flows
node tools/ui-audit/run.mjs --profiles desktop,phone --scenario settingsMatrix,auditPages
```

`run.mjs` serves `apps/frontend/build` (run the frontend build first — the npm
script does) and writes screenshots + `index.html` to
`tools/ui-audit/shots/<timestamp>/`. The command exits non-zero when any
scenario fails or an audit assertion fails.

Chrome/Edge is discovered from `~/.cache/ms-playwright`, `~/.cache/puppeteer`,
`$CHROME_PATH`, or the system `chromium`/`google-chrome`/Chrome/Edge. The
harness runs on Linux, macOS and Windows (the Windows CI job uses Edge).

## Profiles

| name | size | notes |
|---|---|---|
| `desktop` | 1440×900 | laptop |
| `desktop-wide` | 1920×1080 | large monitor |
| `windows-125` | 1440×900 @ dpr 1.25 | Windows at 125% display scaling |
| `windows-150` | 1440×900 @ dpr 1.5 | Windows at 150% display scaling |
| `phone` | 412×915 | Pixel 8 class |
| `phone-small` | 360×740 | budget 360dp class |
| `fold` | 673×841 | unfolded inner screen |
| `tablet` | 800×1280 | tablet |

Mobile profiles also spoof an Android WebView user agent so Android-only UI
(widget settings, share surfaces) is rendered. Every profile starts from a
cleared localStorage, so appearance changes made by an earlier profile cannot
scale later measurements (issues #126/#134).

## Scenarios

Tours: `welcome unlock home sidebar doc whiteboard graph palette drawer sheet
split capture widget` or `all`.

Audit scenarios: `settingsMobile settingsMatrix settingsScales overlaysXlarge
settingsAI settingsUpdates settingsBackup settingsLock flows vaultFlow
appearanceMatrix auditPages graphPan`.

See `shot.mjs` for what each one asserts. Useful extras:

- `--theme dark|light` — emulate `prefers-color-scheme`.
- `--setting k=v` — seed `enclave-settings` localStorage (e.g.
  `--setting trueBlack=true --setting uiScale=xlarge`).
- `--scrollbars` — keep the browser's scrollbars visible instead of passing
  `--hide-scrollbars` (useful when checking scrollbar-related layout shifts).
- `--out DIR`, `--chrome PATH`, `--verbose`.

## Local demo (click through the UI)

The same stub powers an interactive browser demo — handy for reviewing the UI
without the Tauri toolchain:

```bash
npm run build -w @enclave/frontend   # once
node tools/ui-audit/demo.mjs          # http://localhost:4300
```

It serves `apps/frontend/build` with the stub injected, opens straight into
the workspace, and supports the whole interface (pages, editor, whiteboard,
graph, settings, themes/accents). State is the demo vault and resets on
reload; `/?fresh` shows the vault creation flow. Add `--port <n>` to change
the port. This is dev tooling, not a product feature.

## CI

`.github/workflows/ui-audit.yml` runs a desktop+phone audit on
`workflow_dispatch` and weekly, then uploads the screenshots + report as a
build artifact. A second job runs the same tour on `windows-latest` with the
DPI profiles on Edge/Chrome. Failures block the run but never a release (it is
a separate, opt-in workflow).
