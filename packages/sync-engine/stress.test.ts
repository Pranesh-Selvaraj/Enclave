// Sync engine stress test — multi-peer convergence under a lossy/slow channel.
//
// Yjs incremental updates are NOT delivery-guaranteed: if an update is dropped
// the content in it is never re-sent (CRDTs only carry the delta since the
// sender's last state vector). So a flaky LAN segment loses data unless peers
// occasionally exchange full state snapshots — which is exactly what a
// reconnect + hello + snapshot exchange does in the real transport.
//
// Control (section 1) proves loss alone breaks convergence. Section 2 runs the
// full mesh WITH periodic resync (the reconnect-recovery path) and asserts every
// peer converges to identical content with no chunk lost.
//
// Run: npx tsx packages/sync-engine/stress.test.ts

import { SyncEngine } from './src/index.ts';
import * as Y from 'yjs';
import * as assert from 'node:assert';

// Dummy crypto that passes bytes through (tests the protocol, not crypto).
function dummyCrypto() {
	return {
		encrypt: async (plain: Uint8Array) => plain,
		decrypt: async (cipher: Uint8Array) => cipher,
	};
}

// Deterministic PRNG (mulberry32) so failures are reproducible.
function mulberry32(seed: number) {
	return function () {
		seed |= 0;
		seed = (seed + 0x6d2b79f5) | 0;
		let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const DOC_ID = 'stress-doc';
const N = 4;

function makePeer(sends: { docId: string; payload: string }[]) {
	return new SyncEngine((docId, payload) => sends.push({ docId, payload }), dummyCrypto());
}

console.log('=== Sync Engine Stress Test ===\n');

// ── 1. Control: a single dropped update is unrecoverable without resync ─────

{
	const sends: { docId: string; payload: string }[] = [];
	const alice = makePeer(sends);
	const bob = new SyncEngine(() => {}, dummyCrypto());
	const a = alice.getDoc(DOC_ID).getText('content');
	const b = bob.getDoc(DOC_ID).getText('content');

	// Alice writes, Bob receives nothing (a dropped update on a lossy link).
	a.insert(0, 'alpha');
	await sleep(10);
	// Simulate the link dropping this message entirely.
	sends.length = 0;
	// Alice keeps editing but nothing is relayed.
	a.insert(a.length, '-beta');
	await sleep(10);

	// Even though Bob never sees the first update, send Bob the *next* one —
	// it only carries the new delta, so 'alpha' is lost forever.
	for (const m of sends) await bob.handleIncoming(DOC_ID, m.payload);
	assert.notStrictEqual(b.toString(), a.toString(), 'control should diverge');
	await alice.destroy();
	bob.destroy();
	console.log('[1/3] Control (loss breaks convergence): PASS');
}

// ── 2. Full lossy mesh with periodic resync — must converge ──────────────────

{
	const sends = Array.from({ length: N }, () => [] as { docId: string; payload: string }[]);
	const peers = sends.map((s) => makePeer(s));
	const texts = peers.map((p) => p.getDoc(DOC_ID).getText('content'));

	const rand = mulberry32(0x5eed);
	const DROP = 0.35; // per-message drop probability
	const MAX_DELAY = 15; // ms — reorders + delays propagation

	// Route one peer's queued messages to every other peer through the lossy
	// channel. Deliveries run concurrently, so application order is shuffled.
	const inflight: Promise<void>[] = [];
	function flush() {
		for (let from = 0; from < N; from++) {
			for (const m of sends[from]) {
				for (let to = 0; to < N; to++) {
					if (to === from) continue;
					inflight.push(
						(async () => {
							if (rand() < DROP) return; // packet loss
							await sleep(Math.floor(rand() * MAX_DELAY));
							await peers[to].handleIncoming(DOC_ID, m.payload);
						})(),
					);
				}
			}
			sends[from] = [];
		}
	}

	// Every peer pushes a full snapshot to every other peer — the reconnect
	// recovery path (hello → snapshot) that heals dropped incremental updates.
	function resync() {
		const r: Promise<void>[] = [];
		for (let from = 0; from < N; from++) {
			r.push(
				(async () => {
					const snap = await peers[from].snapshot(DOC_ID);
					for (let to = 0; to < N; to++) {
						if (to === from) continue;
						await peers[to].handleIncoming(DOC_ID, snap);
					}
				})(),
			);
		}
		return r;
	}

	const CHUNKS = 12;
	for (let c = 0; c < CHUNKS; c++) {
		// All peers edit concurrently — forces multi-master CRDT convergence.
		for (let i = 0; i < N; i++) texts[i].insert(texts[i].length, `P${i}C${c};`);
		await sleep(10); // let the async encrypt+send handlers fire
		flush();
		// Reconnect/re-sync periodically (simulates dropped-then-redialed links).
		if (c % 4 === 0) inflight.push(...(await Promise.all(resync())));
	}
	inflight.push(...(await Promise.all(resync()))); // final heal
	await Promise.all(inflight);
	await sleep(50); // settle

	const final = texts[0].toString();
	assert.ok(final.length > 0, 'final doc should not be empty');
	// No chunk may be lost, no matter how many messages the link dropped.
	for (let i = 0; i < N; i++) {
		for (let c = 0; c < CHUNKS; c++) {
			assert.ok(final.includes(`P${i}C${c};`), `lost chunk P${i}C${c}`);
		}
	}
	// Every peer must be byte-identical (CRDT convergence).
	for (let i = 1; i < N; i++) {
		assert.strictEqual(texts[i].toString(), final, `peer ${i} diverged`);
	}

	for (const p of peers) p.destroy();
	console.log(`[2/3] Lossy 4-peer mesh converged (${CHUNKS} rounds, ${DROP} drop): PASS`);
}

// ── 3. Slow-link fan-out: 3 peers, one source, chained delivery ──────────────

{
	const sends: { docId: string; payload: string }[][] = [[], [], []];
	const peers = sends.map((s) => makePeer(s));
	const texts = peers.map((p) => p.getDoc(DOC_ID).getText('content'));

	// Alice types a long document while Bob and Carol are on slow links
	// (each update arrives 30–80ms late, sometimes out of order).
	const rand = mulberry32(0xabba);
	const body = 'The quick brown fox jumps over the lazy dog. '.repeat(3);
	for (let i = 0; i < body.length; i += 5) {
		texts[0].insert(texts[0].length, body.slice(i, i + 5));
	}
	await sleep(20);
	await Promise.all(sends[0].map((m) => (async () => {
		await sleep(30 + Math.floor(rand() * 50));
		await peers[1].handleIncoming(DOC_ID, m.payload);
		await peers[2].handleIncoming(DOC_ID, m.payload);
	})()));

	// One reconnect snapshot heals anything the slow links mangled.
	await peers[1].handleIncoming(DOC_ID, await peers[0].snapshot(DOC_ID));
	await peers[2].handleIncoming(DOC_ID, await peers[0].snapshot(DOC_ID));

	assert.strictEqual(texts[1].toString(), body, 'bob diverged on slow link');
	assert.strictEqual(texts[2].toString(), body, 'carol diverged on slow link');

	for (const p of peers) p.destroy();
	console.log('[3/3] Slow chained fan-out: PASS');
}

console.log('\n=== All sync engine stress checks passed ===');
