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

Chrome is discovered from `~/.cache/ms-playwright`, `~/.cache/puppeteer`,
`$CHROME_PATH`, or the system `chromium`/`google-chrome`.

## Profiles

| name | size | notes |
|---|---|---|
| `desktop` | 1440×900 | laptop |
| `desktop-wide` | 1920×1080 | large monitor |
| `phone` | 412×915 | Pixel 8 class |
| `phone-small` | 360×740 | budget 360dp class |
| `fold` | 673×841 | unfolded inner screen |
| `tablet` | 800×1280 | tablet |

Mobile profiles also spoof an Android WebView user agent so Android-only UI
(widget settings, share surfaces) is rendered.

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
- `--out DIR`, `--chrome PATH`, `--verbose`.

## CI

`.github/workflows/ui-audit.yml` runs a desktop+phone audit on
`workflow_dispatch` and weekly, then uploads the screenshots + report as a
build artifact. Failures block the run but never a release (it is a separate,
opt-in workflow).
