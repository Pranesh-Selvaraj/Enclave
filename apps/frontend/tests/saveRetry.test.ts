import * as assert from 'node:assert';
import { saveWithRetry } from '../src/lib/saveRetry.ts';

let version = 0;

// 1. First-try success
let calls = 0;
let r = await saveWithRetry(
	async () => { calls++; },
	() => version,
	0,
);
assert.strictEqual(r.ok, true, 'first-try success');
assert.strictEqual(calls, 1, 'save called exactly once on success');

// 2. Transient failures retry with backoff, then succeed
calls = 0;
const started = Date.now();
r = await saveWithRetry(
	async () => { calls++; if (calls < 3) throw new Error('db busy'); },
	() => version,
	0,
);
assert.strictEqual(r.ok, true, 'recovers after transient failures');
assert.strictEqual(calls, 3, 'retried until success');
assert.ok(Date.now() - started >= 1000, 'backoff waited between attempts');

// 3. Newer save supersedes: stale retry must abort, not overwrite
calls = 0;
version = 1; // a newer save happened while we were mid-retry
r = await saveWithRetry(
	async () => { calls++; throw new Error('db busy'); },
	() => version,
	0, // our save captured version 0 — now stale
);
assert.strictEqual(r.ok, false, 'stale retry aborts');
assert.strictEqual(calls, 1, 'stale retry does not loop');

// 4. Permanent failure reports ok:false after exhausting attempts
calls = 0;
r = await saveWithRetry(
	async () => { calls++; throw new Error('disk full'); },
	() => version,
	version,
	3,
	10, // short backoff for the test
);
assert.strictEqual(r.ok, false, 'permanent failure reported');
assert.strictEqual(calls, 3, 'exhausted all attempts');

console.log('saveWithRetry: PASS');
