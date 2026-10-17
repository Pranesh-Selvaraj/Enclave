#!/usr/bin/env node
// Regenerate every platform icon from the brand sources in docs/brand/.
//
// Replaces scripts/generate-icons.py, which redrew the mark from scratch with Pillow —
// a second, hand-synced copy of the geometry — and wrote the Android icons into
// src-tauri/gen/android, a path the repository restructure left behind. The native app
// moved to apps/android, so Android had stopped receiving icons entirely.
//
// Sources of truth:
//   docs/brand/logo.svg             the app icon: ink tile + keyhole, on Apple's icon grid
//   docs/brand/icon-foreground.svg  Android adaptive foreground: the keyhole alone, 62 % of
//                                   the canvas, inside Android's 66/108 safe zone
//   docs/brand/icon-background.svg  Android adaptive background: solid ink
//
// `tauri icon` does the rasterising and writes .ico/.icns correctly, so the geometry stays
// in one place and this script only places the output.
//
// Usage: node scripts/generate-icons.mjs

import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, cpSync, readdirSync, statSync, rmSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const BRAND = join(ROOT, 'docs/brand');
const ICONS = join(ROOT, 'src-tauri/icons');
const ANDROID_RES = join(ROOT, 'apps/android/app/src/main/res');
const DENSITIES = ['mdpi', 'hdpi', 'xhdpi', 'xxhdpi', 'xxxhdpi'];

const TMP = mkdtempSync(join(tmpdir(), 'enclave-icons-'));
const OUT = join(TMP, 'out');

// tauri icon takes a manifest; paths in it are relative to the manifest's own location.
const manifest = join(TMP, 'manifest.json');
writeFileSync(
	manifest,
	JSON.stringify(
		{
			default: join(BRAND, 'logo.svg'),
			bg_color: '#16130F',
			android_bg: join(BRAND, 'icon-background.svg'),
			android_fg: join(BRAND, 'icon-foreground.svg'),
			android_fg_scale: 100,
		},
		null,
		2
	) + '\n'
);

console.log('→ tauri icon');
execFileSync('npx', ['tauri', 'icon', manifest, '-o', OUT], { cwd: ROOT, stdio: 'inherit' });

// Desktop: every file at the top level of the output, plus the iOS set.
let desktop = 0;
for (const entry of readdirSync(OUT)) {
	const from = join(OUT, entry);
	if (statSync(from).isDirectory()) continue;
	cpSync(from, join(ICONS, entry));
	desktop++;
}
cpSync(join(OUT, 'ios'), join(ICONS, 'ios'), { recursive: true });

// Android: replace the raster launcher set. ic_launcher_background is deliberately not
// copied — the adaptive icon in mipmap-anydpi-v26 takes its background from
// @color/ic_launcher_background, so a background PNG would be dead weight.
let android = 0;
for (const d of DENSITIES) {
	const to = join(ANDROID_RES, `mipmap-${d}`);
	mkdirSync(to, { recursive: true });
	for (const name of ['ic_launcher.png', 'ic_launcher_round.png', 'ic_launcher_foreground.png']) {
		cpSync(join(OUT, 'android', `mipmap-${d}`, name), join(to, name));
		android++;
	}
}

rmSync(TMP, { recursive: true, force: true });
console.log(`\n✓ ${desktop} desktop files → src-tauri/icons`);
console.log(`✓ ${android} Android files → apps/android/app/src/main/res/mipmap-*`);
console.log('✓ iOS set → src-tauri/icons/ios');
console.log('\nThe Android adaptive background colour lives in');
console.log('apps/android/app/src/main/res/values/ic_launcher_background.xml — keep it ink (#16130F).');
