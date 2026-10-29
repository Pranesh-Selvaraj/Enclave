# 0009. Public homepage as a static GitHub Pages site

- **Status:** Accepted
- **Date:** 2026-10-29
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

The repository is Enclave's public identity, but it is written for people who
have already decided to care: a long README, release assets, ADRs and a plan.
Someone arriving from a link has no front door that says what Enclave is, shows
what it looks like, states the security posture in a few lines and sends them
to the right download. The GitHub repository page cannot be that front door
without turning the README into marketing copy.

The homepage has to satisfy constraints that come from the product itself:

- **No backend.** A site that needs a server, a database or an edge function
  adds an account, a bill and an attack surface for a product whose central
  claim is that there is no server.
- **No third-party requests.** No CDN, hosted fonts, analytics, cookie banner
  or embedded widgets. A privacy product whose own homepage leaks visitor IPs
  to several vendors is not credible.
- **No second copy of the brand.** The lockups, mark, favicons, social card,
  screenshots and Inter font already exist at their product paths, and
  `docs/brand/README.md` names the lockup as the asset "for the README and the
  website". A site that keeps its own copies will drift.
- **A project Pages base path.** The site is published at
  `https://pranesh-selvaraj.github.io/Enclave/`, not at a domain root, so
  root-absolute asset paths break.
- **Cheap to keep alive during the native-app rebuild.** The product is
  mid-migration and the page must not become a second application to maintain.
- **Show, don't just tell.** A visitor deciding whether to trust a notes app
  should be able to try the editor, see a sync happen and understand the
  security model without installing anything. Static screenshots and a feature
  list do not do that.

The `site/` directory, a small build script and a Pages workflow are the result.

## Decision

- **Hand-written static files in `site/`.** One `index.html`, one `styles.css`,
  a theme-boot script and a progressive-enhancement script. No framework, no
  templating, no npm dependencies for the site itself. It is still a document,
  not a SPA: the only application-like part is a five-view product tour
  (Editor, Sync, Whiteboard, Graph, Local AI) built from the same HTML, CSS and
  vanilla JS. On wide and tall screens the window pins and a step rail advances
  with scroll; elsewhere, and under reduced motion, the tabs switch views in
  place. The demos use fixed sample data; they do not open a vault, fetch or
  persist anything.
- **The tour is guided and stateful.** A Try row performs one action per view;
  prev/next, a step counter and the `1`–`5` keys move through the views. The
  editor has an editable title, a block menu and a live Markdown view; the
  phone sync accepts typed items; the whiteboard supports double-click and
  delete; the graph zooms, pans and focuses; and AI answers cite pages that
  open in the graph. State stays in the page; the Markdown copy touches the
  clipboard only on an explicit click.
- **The roadmap is a rendering of the plan, not a promise list.** A small
  Now/Next/Then panel pulls its features and exit criteria from
  `docs/plans/dedicated-native-apps.md` and `docs/STATUS.md`, and a
  "Deliberately not planned" row records the plan's non-goals. It adds no
  dates, durations or features of its own.
- **A command palette is the page's keyboard layer.** `/` or `Ctrl/⌘K` (or the
  header search button) opens a filtered list of sections, tour views and the
  theme. It drives the existing links and tab buttons, so it adds no content or
  network requests. It is progressive enhancement; without scripting the
  roadmap's first phase is still readable, while the palette trigger and the
  phase tabs are hidden rather than left inert.
- **Shared assets are copied at build time, never duplicated in git.**
  `scripts/build-site.mjs` assembles `site-dist/` from `site/` plus
  `docs/brand/*`, `docs/screenshots/*.png` and
  `apps/frontend/static/fonts/InterVariable.woff2` (with its OFL licence). It
  fails if a required source is missing or if `index.html` references a local
  asset that was not produced.
- **`site-dist/` is a disposable artifact.** It is git-ignored. The workflow
  builds it on every relevant push and uploads it with the official
  `upload-pages-artifact` action; `deploy-pages` publishes it.
- **Deployment is a GitHub Actions Pages deployment from `main`.** The
  workflow runs on pushes that touch the site, brand, screenshots, fonts or the
  build script, and on manual dispatch. It requests only `contents: read`,
  `pages: write` and `id-token: write`, and does not cancel an in-flight
  deployment.
- **One-time repository setting: Pages source = GitHub Actions.** This is a
  manual setting and is documented in `site/README.md`; without it the deploy
  job fails rather than silently publishing nothing.
- **The site is self-contained and private by construction.** Fonts are
  self-hosted, screenshots and vectors are local, there is no analytics, no
  cookies and no runtime fetch. The tour is sample data only and persists
  nothing. A `Content-Security-Policy` meta tag sets `default-src 'none'` and
  allows only same-origin images, styles, scripts and fonts. The theme follows
  the system and can be toggled; the choice is stored in `localStorage` only.
- **Download links target `releases/latest`.** The page never hard-codes a
  version, so a release does not require a homepage change. The migration
  notice and the Android "Beta" label keep the page honest while `main` is
  rebuilt.
- **The homepage uses relative asset paths; the 404 is self-contained.** The
  homepage works under the `/Enclave/` project base path with no `<base>` tag.
  GitHub Pages serves `404.html` at arbitrary depths, so that one file inlines
  its CSS and gets the shared lockup injected at build time, with
  `style-src 'unsafe-inline'` and no scripts. Its links to the site are
  absolute.
- **The palette follows the app's tokens.** Background, surfaces, borders,
  text, accent and status colours mirror `apps/frontend/src/app.css`, including
  the warm charcoal dark theme and paper light theme, so the homepage and the
  app do not look like different products.

## Consequences

- Publishing is one command locally (`npm run site:build`) and automatic on
  merge. There is no site build dependency to audit and no framework upgrade to
  track.
- Brand and screenshot updates flow to the site automatically; a change to the
  logo or a new screenshot does not need a second commit in `site/`.
- The page's prose, security claims and platform status are still maintained by
  hand and can drift from `README.md`/`SECURITY.md`. The edit rules in
  `site/README.md` make that an explicit review obligation, not an accident.
- The live tour is real client-side code: it needs interaction, keyboard and
  reduced-motion testing, and its sample content can drift from the app. It is
  deliberately scoped to five thin demos, not a second editor and not a real
  vault.
- The guided actions, Markdown generator, graph zoom and citations are demo
  code. Adding a block type, graph node or Try action means updating the
  matching generator or id, and the keyboard and no-JS paths need retesting.
- The roadmap can drift from the plan and STATUS; those documents stay
  canonical and the panel links to them. Adding a section or a tour view means
  updating the palette's action map as well as the markup.
- `404.html` is the only page allowed inline CSS. Its styles are a small,
  deliberate duplicate so the fallback cannot break on a deep path or in local
  preview; the lockup itself is still injected from the one brand source.
- A CSP delivered in a meta tag cannot use `frame-ancestors` and cannot set
  other response headers; GitHub Pages does not let this repository set them.
  The site has no forms, accounts or state-changing actions, so the residual
  clickjacking/framing risk is accepted. Serving it behind a proxy only to add
  headers would contradict the no-backend decision.
- The canonical URL, Open Graph image and the 404's homepage link are tied to
  the `pranesh-selvaraj.github.io/Enclave/` origin. Renaming the repository or
  moving to a custom domain requires updating them.
- There is no traffic data, by design. Questions about what visitors read
  cannot be answered from analytics; they have to come from GitHub issues.
- The homepage is not versioned with the app. It can describe `main` while the
  supported release is older; the migration notice and the "latest stable"
  links must stay accurate during that period.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Markdown + a static-site generator (Hugo, Astro, SvelteKit) | A toolchain, dependency tree and theme to maintain for a single page. The content has no repeating collection, the wordmark must stay a vector (not typeset text), and a generator would add a build step without removing the hand-written layout. |
| Publish `/docs` with Jekyll from `main` | Mixes product documentation, ADRs and a marketing page in one tree; Jekyll's implicit processing makes the deployed output harder to reason about; and the page would inherit a documentation layout rather than the brand. |
| A `gh-pages` branch with committed build output | A second copy of the site that goes stale, needs manual rebuilds and makes review of the actual output harder. Deploying the artifact from `main` keeps one source of truth. |
| Netlify, Vercel or Cloudflare Pages | Adds an account, a third party and a request log for a static page that GitHub Pages already hosts. It buys build previews and headers this page does not need, and weakens the no-cloud posture. |
| A SvelteKit site inside the monorepo | The most familiar stack, but it pulls the frontend build and its dependency surface into the homepage, and turns a copy edit into an application deploy. |
| A screenshot gallery only | Static images go stale, cannot show sync and cannot let a visitor tick a task; the homepage becomes a brochure. The live tour costs no video weight because the views are HTML/CSS/JS. |
| A video walkthrough | Heavy, not interactive, not searchable, needs captions and a player, and would be the only large media asset on a page that promises no third-party requests. |
| Embed the real app | Requires shipping or loading the actual vault/editor: a backend or a large wasm bundle, plus a real key-handling surface on a marketing page. The tour uses sample data on purpose. |
| Keep the README as the front door | The README is a contributor and reference document. It is long by necessity, and it cannot show the product or route a first-time visitor to the right installer without becoming a worse README. |

## Prior art / what this replaces

This is the first public website; there is no earlier site to replace. It
extends the brand work in `docs/brand/` (which already reserved the lockups for
the website) and follows the repository's CI conventions: pinned major action
versions, npm scripts for build entry points, and an explicit security posture
rather than a vendor default. The homepage is a summary and a router; the
record stays in `README.md`, `CHANGELOG.md`, `SECURITY.md` and
`docs/decisions/`.

The first version of this site was a static screenshot gallery. It was replaced
before merge by the live tour: the screenshots were already going stale and
could not show the one thing that matters — what it feels like to use the app.
The build, hosting, brand and no-third-party decisions did not change.
