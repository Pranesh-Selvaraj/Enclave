#!/usr/bin/env node
// Builds code.js from code.template.js + docs/design/tokens.json + docs/brand/*.svg.
// Usage: node docs/design/figma/build.mjs
import { readFileSync, writeFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..', '..');
const read = (path) => readFileSync(join(root, path), 'utf8');

const tokens = JSON.parse(read('docs/design/tokens.json'));
const assets = {
  __TOKENS__: JSON.stringify(tokens),
  __SVG_LOCKUP_INK__: read('docs/brand/lockup.svg').trim(),
  __SVG_LOCKUP_PAPER__: read('docs/brand/lockup-dark.svg').trim(),
  __SVG_ICON__: read('docs/brand/logo.svg').trim(),
  __SVG_WATERMARK_INK__: read('docs/brand/watermark.svg').trim(),
  __SVG_WATERMARK_PAPER__: read('docs/brand/watermark-dark.svg').trim(),
};

let source = readFileSync(join(here, 'code.template.js'), 'utf8');
for (const [placeholder, value] of Object.entries(assets)) {
  if (!source.includes(placeholder)) {
    throw new Error(`template is missing ${placeholder}`);
  }
  if (placeholder.startsWith('__SVG')) {
    if (value.includes('`') || value.includes('${')) {
      throw new Error(`${placeholder} contains a backtick or template expression`);
    }
    source = source.replace(placeholder, '`' + value + '`');
  } else {
    source = source.replace(placeholder, value);
  }
}

const output = join(here, 'code.js');
writeFileSync(output, source);
console.log(`wrote ${output} (${Math.round(statSync(output).size / 1024)} KB)`);
