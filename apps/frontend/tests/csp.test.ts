// Config guard: vault creation runs Argon2id via hash-wasm, which compiles a
// WASM module at runtime. The Tauri CSP must allow that, or every vault
// creation fails with "WebAssembly.compile() violates CSP". Regression test
// for the directive that fixes it.
import * as assert from 'node:assert';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const conf = JSON.parse(
	readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../../../src-tauri/tauri.conf.json'), 'utf8'),
);
const csp: string = conf.app.security.csp;

assert.ok(csp.includes("script-src 'self' 'wasm-unsafe-eval'"), 'CSP must allow WASM (hash-wasm Argon2id)');
assert.ok(!csp.includes("'unsafe-eval'"), 'CSP must not allow general eval');

// Sanity: the rest of the surface stays intact.
for (const d of ["default-src 'self'", "style-src 'self' 'unsafe-inline'", "connect-src 'self' https:"]) {
	assert.ok(csp.includes(d), `CSP missing: ${d}`);
}

console.log('csp config: PASS (wasm-unsafe-eval present, unsafe-eval absent)');
