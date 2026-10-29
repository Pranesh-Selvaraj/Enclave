#!/usr/bin/env node
/**
 * Assembles the public homepage into site-dist/.
 *
 * The page itself has no build step, but the brand vectors, screenshots and
 * the Inter font are maintained in their product locations (docs/brand,
 * docs/screenshots, apps/frontend/static/fonts). Copying them here keeps one
 * source of truth instead of a second, drifting set of assets.
 *
 * The build fails if a required source asset is missing or if index.html
 * references a local asset that was not produced, so a broken image cannot
 * reach the deployed site unnoticed.
 */
import { copyFile, cp, mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = path.join(root, 'site');
const output = path.join(root, 'site-dist');

const copies = [
  ['docs/brand/lockup.svg', 'assets/brand/lockup.svg'],
  ['docs/brand/lockup-dark.svg', 'assets/brand/lockup-dark.svg'],
  ['docs/brand/logo-tile.svg', 'assets/brand/logo-tile.svg'],
  ['docs/brand/social-preview.png', 'assets/brand/social-preview.png'],
  ['apps/frontend/static/fonts/InterVariable.woff2', 'assets/fonts/InterVariable.woff2'],
  ['apps/frontend/static/fonts/Inter-OFL.txt', 'assets/fonts/Inter-OFL.txt'],
];

if (!existsSync(source)) {
  throw new Error(`Missing site source directory: ${source}`);
}

await rm(output, { recursive: true, force: true });
await cp(source, output, {
  recursive: true,
  filter: (file) => path.basename(file) !== 'README.md',
});

async function copyInto(relativeSource, relativeDestination) {
  const from = path.join(root, relativeSource);
  const to = path.join(output, relativeDestination);

  if (!existsSync(from)) {
    throw new Error(`Missing required asset: ${relativeSource}`);
  }

  await mkdir(path.dirname(to), { recursive: true });
  await copyFile(from, to);
}

for (const [from, to] of copies) {
  await copyInto(from, to);
}

const screenshots = (await readdir(path.join(root, 'docs/screenshots')))
  .filter((file) => file.endsWith('.png'))
  .sort();

if (screenshots.length === 0) {
  throw new Error('No screenshots found in docs/screenshots');
}

for (const screenshot of screenshots) {
  await copyInto(`docs/screenshots/${screenshot}`, `assets/screenshots/${screenshot}`);
}

/* Verify every local asset referenced by the homepage actually exists. */
const html = await readFile(path.join(output, 'index.html'), 'utf8');
const references = new Set(
  [...html.matchAll(/(?:src|href)="((?:assets|styles|theme-boot|main)[^"]*)"/g)].map((match) => match[1]),
);

for (const reference of references) {
  if (!existsSync(path.join(output, reference))) {
    throw new Error(`index.html references a missing local file: ${reference}`);
  }
}

/* GitHub Pages otherwise runs Jekyll over the uploaded artifact. */
await writeFile(path.join(output, '.nojekyll'), '');

let bytes = 0;
for (const file of await readdir(output, { recursive: true })) {
  const info = await stat(path.join(output, file));
  if (info.isFile()) {
    bytes += info.size;
  }
}

console.log(
  `Built site-dist/ (${screenshots.length} screenshots, ${(bytes / 1024 / 1024).toFixed(1)} MB)`,
);
