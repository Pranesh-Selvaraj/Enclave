#!/usr/bin/env node
// Serve the built frontend and run the UI audit against it, then write an
// HTML report next to the screenshots.
//
//   npm run audit:ui                              # defaults: desktop+phone, all scenarios
//   node tools/ui-audit/run.mjs --profiles phone --scenario flows,settingsMatrix
//   node tools/ui-audit/run.mjs --out /tmp/ui-audit
//
// The frontend must be built first (`npm run build -w @enclave/frontend`);
// the npm script does that for you.
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, '..', '..');
const FRONTEND = join(REPO, 'apps', 'frontend');
const BUILD = join(FRONTEND, 'build');

if (!existsSync(join(BUILD, 'index.html'))) {
	console.error('frontend build not found — run: npm run build -w @enclave/frontend');
	process.exit(2);
}

const passthrough = process.argv.slice(2);
const outIdx = passthrough.indexOf('--out');
const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
const out = outIdx > -1 ? passthrough[outIdx + 1] : join(HERE, 'shots', stamp);
mkdirSync(out, { recursive: true });

const port = Number(process.env.UI_AUDIT_PORT ?? 4183);
const preview = spawn(
	join(FRONTEND, 'node_modules', '.bin', 'vite'),
	['preview', '--port', String(port), '--strictPort'],
	{ cwd: FRONTEND, stdio: ['ignore', 'ignore', 'inherit'] },
);
let stopped = false;
const stop = () => {
	if (stopped) return;
	stopped = true;
	preview.kill('SIGTERM');
};
process.on('exit', stop);
process.on('SIGINT', () => process.exit(130));
process.on('SIGTERM', () => process.exit(143));

async function waitForServer() {
	for (let i = 0; i < 80; i++) {
		try {
			const res = await fetch(`http://127.0.0.1:${port}/`);
			if (res.ok) return true;
		} catch {
			/* not up yet */
		}
		await new Promise((r) => setTimeout(r, 250));
	}
	return false;
}

if (!(await waitForServer())) {
	console.error(`preview server did not come up on :${port}`);
	process.exit(1);
}

// Sensible CI defaults unless the caller chose profiles/scenarios.
const args = ['--url', `http://127.0.0.1:${port}`];
if (!passthrough.includes('--profiles')) args.push('--profiles', 'desktop,phone');
if (!passthrough.includes('--scenario')) args.push('--scenario', 'all');
if (outIdx === -1) args.push('--out', out);
args.push(...passthrough);

const shot = spawn(process.execPath, [join(HERE, 'shot.mjs'), ...args], { stdio: 'inherit' });
const code = await new Promise((resolve) => shot.on('exit', (c) => resolve(c ?? 1)));

spawnSync(process.execPath, [join(HERE, 'report.mjs'), out, '--title', 'Enclave UI audit'], { stdio: 'inherit' });
stop();
console.log(`\nUI audit finished (exit ${code}) → ${out}`);
process.exit(code);
