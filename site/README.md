# Enclave homepage

The public homepage, served from GitHub Pages at
<https://pranesh-selvaraj.github.io/Enclave/>.

It is deliberately a hand-written static site: one main HTML page, one
privacy page, one stylesheet, two scripts, a self-contained 404 and no build
framework. There is
no analytics, no cookie, no CDN and no runtime request outside the page's own
origin — because the product makes the same promise. The reasoning and the
alternatives are in
[ADR 0009](../docs/decisions/0009-public-homepage-github-pages.md).

## Build and preview

```bash
npm run site:build                     # assembles site-dist/
python3 -m http.server 8080 -d site-dist
# then open http://localhost:8080/
```

`site-dist/` is a deploy artifact and is git-ignored. The build script copies
the brand vectors, screenshots and the Inter OFL from their product locations,
and verifies the committed Inter subset, the local asset references and the
JSON-LD CSP hash. See [Fonts](#fonts) and [Structured data](#structured-data).

## The live tour

The product section is not a screenshot gallery. It is a window with five
small, self-contained views:

| View | What you can try |
| --- | --- |
| Editor | Edit the title, insert a block, tick tasks, switch to Markdown and copy the source |
| Sync | Type an item on the phone and watch the encrypted frame reach the desktop |
| Whiteboard | Drag a sticky, double-click the canvas to add one, or press Delete to remove it |
| Graph | Hover or tap to focus a page, scroll to zoom, drag to pan |
| Local AI | Pick a sample prompt; citations open the cited page in the graph |

On screens at least 1100×700 the window pins and the tab strip becomes a step
rail that advances with scroll. Everywhere else — and whenever the visitor
prefers reduced motion — the tabs stay above the window and switch views in
place. Prev/next and a `01 / 05` counter sit under the window; `1`–`5` switch
views while the tour is on screen. Each view is addressable: `?view=editor`,
`?view=sync`, `?view=board`, `?view=graph` or `?view=ai` opens that step, and the
command palette's "Copy link to this view" copies the current one.

A **Try** row above the fine print runs one guided action per view: tick a
task, send a phone change, add a sticky, follow a graph link, ask the local
assistant. The window's status chip reflects demo actions (`Saved locally`,
`Synced · 2 devices`) and returns to `Encrypted`. These are local demo events,
not a real vault or network indicator.

Without JavaScript the tour tabs, Try row, prev/next and keyboard hint are
hidden; the first view remains visible, and every link, screenshot and
download still works.

The demos use **sample data only**. They do not open a vault, do not read or
write files, do not call a model and do not persist anything. Do not put real
secrets in them.

### Shared state and feedback

The views share one small demo state. Renaming the editor page updates the
assistant's context chip, its initial summary, the graph node and detail, and
every "Cited" label for that page. Ticking a task updates the assistant's open
task chip and the initial summary, and the next "Summarise" answer is built
from the current title and open-task count. The graph data and the editor keep
the same page title; the sync view remains its own packing-list scenario.

A visually hidden `[data-tour-live]` region announces only **explicit** view
changes and guided results. Scroll-driven changes stay silent so a reader
moving through the page is not interrupted.

The **Play the tour** button runs the five views and their Try actions as an
opt-in sequence. Any pointer, key, wheel or touch input stops it; toggling the
button stops it; it is disabled when the visitor prefers reduced motion. The
manual Try row and prev/next remain the primary controls.

## Installing help

The download section has an "Installing and verifying" disclosure with the
Linux, Windows, macOS and Android steps plus the `SHA256SUMS` verification
commands. Keep it in step with the artifacts and with the updater description
in `SECURITY.md`.

## Privacy and security.txt

[`privacy.html`](privacy.html) describes the website, the demos and the app:
no cookies, no analytics, no third-party requests, `localStorage` for the theme
only, sample data in the demos, and a local encrypted vault. The footer links
to it.

`/.well-known/security.txt` and the convenience copy at `/security.txt` point
to GitHub's private security advisory flow and `SECURITY.md`; the `Expires`
field is deliberately dated and must be renewed. RFC 9116 expects the well-known
file at the **domain root**; this repository is a project site, so its copy is
served under `/Enclave/.well-known/security.txt`. Moving to a custom domain or
publishing a user-site repository is the opportunity to place it at the root.

## Structured data

`index.html` carries inline `SoftwareApplication` JSON-LD. Because the main CSP
does not allow inline scripts, the exact schema text is allowed by a SHA-256
hash in `script-src`; the build recomputes the hash and fails if the CSP copy is
stale. The page also allows `connect-src 'self'` so same-origin audits (for
example Lighthouse reading `robots.txt`) are not blocked; the page itself makes
no fetch.

## Fonts

The site uses a 115 KB Latin subset of Inter generated from the app's 344 KB
variable font. The subset lives at `assets/fonts/InterVariable-site.woff2`; the
OFL is copied from `apps/frontend/static/fonts/Inter-OFL.txt` at build time.
Regenerate it after adding new punctuation or symbols to the page:

```bash
pyftsubset apps/frontend/static/fonts/InterVariable.woff2 \
  --unicodes='U+0020-007E,U+00A0-00FF,U+2000-206F,U+20A0-20CF,U+2100-214F,U+2190-2199,U+21B5,U+2200-22FF,U+2318,U+2726' \
  --layout-features='*' --flavor=woff2 \
  --output-file=site/assets/fonts/InterVariable-site.woff2
```

User-typed characters outside the subset fall back to the system font; the page
copy itself is covered.

## Roadmap and command palette

The **roadmap** is a rendering of the product plan, not a separate promise
list. Its Now/Next/Then panels and exit criteria come from
[`docs/plans/dedicated-native-apps.md`](../docs/plans/dedicated-native-apps.md)
and [`docs/STATUS.md`](../docs/STATUS.md); the "Deliberately not planned" row
records the plan's non-goals. It contains no dates or durations — a phase ends
when its exit criteria are met.

The **command palette** opens with `/`, `Ctrl/⌘K`, or the search button in the
header. It filters sections, the five live views, the theme and a **Copy link
to this view** action; arrow keys navigate, Enter runs, Escape closes. It is a
keyboard layer over existing links and tab buttons, so it adds no new content
or network requests. Without JavaScript, the first roadmap phase stays
readable; the palette trigger and the phase tabs are hidden because they would
be inert.

## What lives where

| Path | Purpose |
| --- | --- |
| `index.html` | Hero, live tour, facts, features, Android, security, roadmap, FAQ, downloads and docs |
| `privacy.html` | The site/demo/app privacy page, linked from the footer |
| `404.html` | Self-contained Pages fallback: inline CSS and the shared lockup injected at build, so it renders at any path and in local preview |
| `styles.css` | Design tokens shared with the app, layout, tour and demos, editor Markdown/graph controls, roadmap, command palette, themes and lightbox |
| `theme-boot.js` | Synchronous in `<head>` so the first paint has the right theme |
| `main.js` | Theme, mobile menu, scroll tour, guided Try actions, editor blocks/Markdown, sync, whiteboard, graph zoom/focus, AI citations, roadmap tabs, command palette, OS detection, reveals and lightbox |
| `robots.txt`, `sitemap.xml` | Crawler hints; the sitemap lists the homepage and privacy page |
| `.well-known/security.txt`, `security.txt` | Security contact and policy; the well-known path is the canonical one |
| `assets/fonts/` | The site Inter subset and the copied OFL |

Brand vectors and screenshots are **not** copied into this directory. Edit
them in `docs/brand/` and `docs/screenshots/`; the build copies them into
`assets/`. The Inter subset is site-owned (the app font remains the full
variable font); the build copies only its OFL.

## Deployment

The workflow is [`.github/workflows/pages.yml`](../.github/workflows/pages.yml).
It runs on pushes to `main` that touch the site or its assets, and on manual
dispatch.

One-time setup in the repository:

1. **Settings → Pages → Build and deployment → Source: GitHub Actions.**
2. Merge to `main`, or run **Actions → Homepage → Run workflow**.

If the Pages source is left on "Deploy from a branch", the deploy job fails
because no `gh-pages` branch exists. The site is published as a project site
under `/Enclave/`, so every local asset path in `index.html` must stay relative
(`assets/...`). The canonical URL and Open Graph tags are absolute and tied to
that project-site URL; update them if the repository is renamed or a custom
domain is added. The privacy page and `security.txt` are built as part of the
same artifact; the sitemap lists the privacy page.

### Why the 404 is different

GitHub Pages serves `404.html` for missing paths at any depth, so relative URLs
cannot be relied on. The file is therefore self-contained: inline CSS and the
brand lockup, injected from `docs/brand/lockup.svg` at build time. It makes no
external requests. Its only relaxed policy is `style-src 'unsafe-inline'`, and
it has no scripts. All other pages keep `style-src 'self'`; the main page's
`script-src 'self'` is widened only by the JSON-LD hash and `connect-src 'self'`
for same-origin audits.

## Editing rules

- **Do not add tracking or runtime requests.** No analytics, external fonts,
  embeds or API calls. The main page's CSP should keep `default-src 'none'`.
- **Keep the demos local.** Sample data only; no fetch, no vault access, no
  persistence. The window status chip is demo feedback, not a real sync signal.
- **Keep shared state in sync.** Title changes dispatch `enclave:demo-title` and
  task changes dispatch `enclave:demo-tasks`; the AI context/chips/cites, the
  graph node and the graph link labels listen for them. Do not hard-code
  "Roadmap Q4" in those views again.
- **Announce explicit actions only.** The `[data-tour-live]` region is written
  by `go()` and guided actions, never by scroll-driven `follow()` changes.
- **Keep auto-play guarded.** It is opt-in, stops on trusted pointer/key/wheel/
  touch input, and is disabled under `prefers-reduced-motion`. Add new views to
  the same loop rather than a second timer.
- **Update the JSON-LD hash.** Edit the schema and run `npm run site:build`; the
  build prints/fails on the expected `sha256-…` value. Never add
  `'unsafe-inline'` to the main `script-src`.
- **Renew `security.txt`.** Keep `Expires` in the future, keep the GitHub
  advisory/policy links current, and remember the project-site path caveat.
- **Regenerate the font subset** with the command above when the page gains a
  glyph outside the current ranges; keep the OFL in the build.
- **Keep guided actions wired.** Every `data-try` action targets a `tab-*` id or
  a demo selector; prev/next and the counter depend on the tab list. The editor
  Markdown generator, whiteboard double-click/delete, graph zoom/focus and AI
  `data-cite` ids all need to move with the markup.
- **Test reduced motion and no-JS.** The tour must fall back to tabs, the first
  view and the first roadmap phase must remain readable, and the palette must
  be inert without scripting.
- **Keep the roadmap honest.** Mirror the plan and STATUS; do not add dates,
  durations or features the plan does not contain, and keep the non-goals row.
- **Keep the palette in sync.** Every command maps to a section id or a `tab-*`
  id; adding a section or view means updating the list. `/` must never open the
  palette while the visitor is typing in a field.
- **Keep view links stable.** `?view=` values are derived from the tab ids
  (`tab-editor` → `editor`), and the copy command uses the current tab. Renaming
  a tab id breaks shared links; update the deep-link test and docs with it.
- **Keep the section nav honest.** The nav highlight uses the section ids and a
  viewport-line scan; adding a section means adding the matching nav link (or
  deliberately mapping it to another one).
- **Link to `releases/latest`**, never a pinned version, so the download
  section does not go stale between releases.
- **Mirror the product docs.** Security claims come from
  [`README.md`](../README.md) and [`SECURITY.md`](../SECURITY.md); update the
  page in the same change when those facts move.
- **Keep the platform status honest.** Android is beta; say so.
- **Keep the base path relative** in `index.html`. A leading `/` breaks the
  project site. `404.html` is the deliberate exception.
- **The 404 lockup is injected, not copied.** Edit `docs/brand/lockup.svg`;
  the build replaces the `INLINE_LOCKUP` marker. Never paste a second copy.
- **Refresh the Android screenshots** in `docs/screenshots/` when the phone UI
  changes; the homepage uses four of them.
- **Run `npm run site:build` before pushing.** It catches missing assets, a
  stale JSON-LD CSP hash, a missing 404 lockup marker and a missing font subset.
