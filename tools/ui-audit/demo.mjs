#!/usr/bin/env node
// Interactive local demo of the Enclave UI.
//
// Serves the built frontend (`apps/frontend/build`) with the same stubbed
// Tauri backend the UI audit uses, so the whole interface can be clicked
// through in a normal browser — pages, editor, whiteboard, graph, settings,
// themes and accents. State is per-page (a reload resets the demo vault).
//
//   npm run build -w @enclave/frontend   # once
//   node tools/ui-audit/demo.mjs [--port 4300]
//
// This is dev tooling, not a product feature: the real app always runs inside
// the Tauri shell (desktop) or the Android app.
import { createServer } from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { dirname, extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { stubSource } from './stub.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const BUILD = join(HERE, '..', '..', 'apps', 'frontend', 'build');
const portArg = process.argv.indexOf('--port');
const PORT = Number(process.env.DEMO_PORT || (portArg > -1 ? process.argv[portArg + 1] : 4300));

if (!existsSync(join(BUILD, 'index.html'))) {
	console.error('frontend build not found — run: npm run build -w @enclave/frontend');
	process.exit(1);
}

const TYPES = {
	'.html': 'text/html; charset=utf-8',
	'.js': 'text/javascript',
	'.mjs': 'text/javascript',
	'.css': 'text/css',
	'.json': 'application/json',
	'.svg': 'image/svg+xml',
	'.png': 'image/png',
	'.jpg': 'image/jpeg',
	'.jpeg': 'image/jpeg',
	'.gif': 'image/gif',
	'.webp': 'image/webp',
	'.ico': 'image/x-icon',
	'.woff': 'font/woff',
	'.woff2': 'font/woff2',
	'.ttf': 'font/ttf',
	'.wasm': 'application/wasm',
	'.map': 'application/json',
	'.txt': 'text/plain; charset=utf-8',
};

// The demo opens straight into the unlocked workspace; the audit keeps the
// guard on main routes so it can exercise the unlock screen. `?fresh` still
// shows the create-vault flow.
const stub = stubSource().replace(
	"return location.pathname.startsWith('/capture') || location.pathname.startsWith('/widget');",
	'return true;',
);

// Inject the stub before the app's module scripts (the static page has no CSP
// meta, so an inline script is fine here; the audit injects the same source
// through CDP). Read per request: caching it meant a rebuild during a running
// demo served stale asset hashes and the page came up blank.
function renderIndex() {
	return readFileSync(join(BUILD, 'index.html'), 'utf8').replace(
		'<script src="/theme-boot.js"></script>',
		`<script src="/theme-boot.js"></script>\n\t\t<script>${stub}</script>`,
	);
}

const server = createServer((req, res) => {
	const pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
	const safe = normalize(pathname).replace(/^(\.\.[/\\])+/, '');
	const file = join(BUILD, safe);
	if (safe && existsSync(file) && statSync(file).isFile() && extname(file) !== '.html') {
		res.writeHead(200, { 'content-type': TYPES[extname(file).toLowerCase()] ?? 'application/octet-stream' });
		res.end(readFileSync(file));
		return;
	}
	// SPA fallback: /, /doc-1, /graph, /settings, …
	res.writeHead(200, { 'content-type': TYPES['.html'] });
	res.end(renderIndex());
});

server.listen(PORT, () => {
	console.log(`\n  Enclave demo (stubbed backend) → http://localhost:${PORT}\n`);
	console.log('  Click through pages, the editor, whiteboard, graph, settings and themes.');
	console.log('  Data is the demo vault and resets on reload. Ctrl+C to stop.\n');
});
