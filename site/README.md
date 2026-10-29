# Enclave homepage

The public homepage, served from GitHub Pages at
<https://pranesh-selvaraj.github.io/Enclave/>.

It is deliberately a hand-written static site: one HTML page, one stylesheet,
two small scripts and no build framework. The page makes no requests outside
its own origin — no CDN, no analytics, no cookies — because the product makes
the same promise. The reasoning and the alternatives are in
[ADR 0009](../docs/decisions/0009-public-homepage-github-pages.md).

## Build and preview

```bash
npm run site:build                     # assembles site-dist/
python3 -m http.server 8080 -d site-dist
# then open http://localhost:8080/
```

`site-dist/` is a deploy artifact and is git-ignored. The build script copies
the brand vectors, screenshots and the Inter font from their product locations,
then fails if `index.html` references a local asset that is missing.

## What lives where

| Path | Purpose |
| --- | --- |
| `index.html` | The homepage: hero, features, screenshots, security, downloads, docs |
| `404.html` | The Pages fallback; uses absolute URLs because Pages serves it at arbitrary paths |
| `styles.css` | Design tokens shared with the app, layout, light/dark themes, lightbox |
| `theme-boot.js` | Synchronous in `<head>` so the first paint has the right theme |
| `main.js` | Theme toggle, screenshot lightbox and footer year; everything degrades without it |
| `robots.txt`, `sitemap.xml` | Crawler hints for the single public page |

Brand vectors and screenshots are **not** copied into this directory. Edit
them in `docs/brand/` and `docs/screenshots/`; the build copies them into
`assets/`.

## Deployment

The workflow is [`.github/workflows/pages.yml`](../.github/workflows/pages.yml).
It runs on pushes to `main` that touch the site or its assets, and on manual
dispatch.

One-time setup in the repository:

1. **Settings → Pages → Build and deployment → Source: GitHub Actions.**
2. Merge to `main`, or run **Actions → Homepage → Run workflow**.

If the Pages source is left on "Deploy from a branch", the deploy job fails
because no `gh-pages` branch exists. The site is published as a project site
under `/Enclave/`, so every local asset path must stay relative (`assets/...`).
The absolute URLs in `404.html`, the canonical link and the Open Graph tags
are tied to that project-site URL; update them if the repository is renamed or
a custom domain is added.

## Editing rules

- **Do not add tracking.** No analytics, no external fonts, no embeds, no
  runtime API calls. The Content-Security-Policy meta tag should keep
  `default-src 'none'`.
- **Link to `releases/latest`**, never a pinned version, so the download
  section does not go stale between releases.
- **Mirror the product docs.** Security claims come from
  [`README.md`](../README.md) and [`SECURITY.md`](../SECURITY.md); update the
  page in the same change when those facts move.
- **Keep the platform status honest.** Android is beta; say so.
- **Keep the base path relative.** A leading `/` breaks the project site.
- **Run `npm run site:build` before pushing.** It catches missing assets.
