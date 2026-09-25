#!/usr/bin/env node
// Build a static HTML gallery from a ui-harness screenshot dir.
//
//   node report.mjs <shots-dir> [--title "Enclave UI report"]
//
// Reads manifest.json (scenario results + audit issues) and every PNG in the
// dir, and writes index.html next to them. Screenshots are linked relatively,
// so the report opens straight from the filesystem.
import { readFileSync, writeFileSync, readdirSync, existsSync } from 'node:fs';
import { join, basename } from 'node:path';

const dir = process.argv[2];
if (!dir || !existsSync(dir)) {
	console.error('usage: node report.mjs <shots-dir> [--title TITLE]');
	process.exit(2);
}
const titleIdx = process.argv.indexOf('--title');
const title = titleIdx > -1 ? process.argv[titleIdx + 1] : 'Enclave UI report';

const manifest = existsSync(join(dir, 'manifest.json'))
	? JSON.parse(readFileSync(join(dir, 'manifest.json'), 'utf8'))
	: { runs: [], issues: [] };
const pngs = readdirSync(dir).filter((f) => f.endsWith('.png')).sort();

// Group by profile (the name prefix before the first dash).
const groups = new Map();
for (const file of pngs) {
	const profile = file.split('-')[0];
	if (!groups.has(profile)) groups.set(profile, []);
	groups.get(profile).push(file);
}

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const issueBadges = manifest.issues ?? [];
const failed = (manifest.runs ?? []).filter((r) => !r.ok);

const html = `<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(title)}</title>
<style>
 :root { color-scheme: dark; }
 body { margin:0; background:#101014; color:#e6e6ea; font:14px/1.5 system-ui,sans-serif; }
 header { position:sticky; top:0; background:#17171d; border-bottom:1px solid #2a2a32; padding:14px 22px; z-index:2; }
 h1 { font-size:18px; margin:0 0 4px; }
 .meta { color:#8f8f9c; font-size:12px; }
 section { padding:16px 22px; }
 h2 { font-size:14px; text-transform:uppercase; letter-spacing:.06em; color:#9d9daa; border-bottom:1px solid #23232b; padding-bottom:6px; }
 .grid { display:grid; grid-template-columns:repeat(auto-fill,minmax(300px,1fr)); gap:14px; }
 figure { margin:0; background:#17171d; border:1px solid #26262f; border-radius:10px; overflow:hidden; }
 figure img { width:100%; display:block; background:#000; }
 figcaption { padding:8px 10px; font-size:12px; color:#b9b9c6; font-family:ui-monospace,monospace; }
 .issues { background:#2a1420; border:1px solid #5c2340; border-radius:10px; padding:12px 16px; margin:0 22px 10px; }
 .issues h3 { margin:0 0 8px; font-size:13px; color:#ff9ec2; }
 .issues li { font-size:12px; font-family:ui-monospace,monospace; color:#f3c6d8; word-break:break-word; }
 .fail { background:#2a1c10; border-color:#5c3f23; }
 .fail h3 { color:#ffc98a; }
 .fail li { color:#f3ddc6; }
 .warn { color:#ffd479; font-weight:600; }
 a { color:#9db8ff; }
</style></head><body>
<header>
  <h1>${esc(title)}</h1>
  <div class="meta">${pngs.length} screenshots · ${(manifest.runs ?? []).length} scenarios · ${failed.length} failed · generated ${new Date().toISOString()}</div>
</header>
${issueBadges.length ? `<div class="issues"><h3>Audit findings (${issueBadges.length})</h3><ul>${issueBadges.map((i) => `<li>${esc(i.profile)}/${esc(i.scenario)} — ${esc(i.name)}: ${esc(i.detail)}</li>`).join('')}</ul></div>` : ''}
${failed.length ? `<div class="issues fail"><h3>Failed scenarios (${failed.length})</h3><ul>${failed.map((f) => `<li>${esc(f.profile)}/${esc(f.scenario)}: ${esc(f.error)}</li>`).join('')}</ul></div>` : ''}
${[...groups.entries()].map(([profile, files]) => `<section><h2>${esc(profile)} <span class="meta">(${files.length})</span></h2><div class="grid">${files.map((f) => `<figure><a href="${esc(f)}" target="_blank"><img loading="lazy" src="${esc(f)}" alt="${esc(basename(f))}"></a><figcaption>${esc(f.replace(/^[^-]+-/, '').replace(/\\.png$/, ''))}</figcaption></figure>`).join('')}</div></section>`).join('')}
</body></html>`;

writeFileSync(join(dir, 'index.html'), html);
console.log(`✔ ${join(dir, 'index.html')}`);
