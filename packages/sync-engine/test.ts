// Sync engine verification — simulates two peers syncing a Yjs document
// with encrypted message exchange.

import { SyncEngine } from './src/index.ts';
import * as Y from 'yjs';
import * as assert from 'node:assert';

// Dummy crypto that passes bytes through (no real encryption).
// This tests the sync protocol, not the crypto.
function dummyCrypto() {
	return {
		encrypt: async (plain: Uint8Array) => plain,
		decrypt: async (cipher: Uint8Array) => cipher,
	};
}

console.log('=== Sync Engine Verification ===\n');

let aliceSends: string[] = [];
let bobSends: string[] = [];

const alice = new SyncEngine(
	(docId, payload) => aliceSends.push(payload),
	dummyCrypto(),
);

const bob = new SyncEngine(
	(docId, payload) => bobSends.push(payload),
	dummyCrypto(),
);

const DOC_ID = 'test-doc-1';

// 1. Get/create a doc on both peers
const aliceDoc = alice.getDoc(DOC_ID);
const bobDoc = bob.getDoc(DOC_ID);
const aliceText = aliceDoc.getText('content');
const bobText = bobDoc.getText('content');
assert.ok(aliceText instanceof Y.Text);
assert.ok(bobText instanceof Y.Text);
console.log('[1/4] Doc creation: PASS');

// 2. Alice makes a change — should trigger an outgoing message
aliceText.insert(0, 'Hello from Alice');
assert.strictEqual(aliceText.toString(), 'Hello from Alice');
// Wait for async encrypt + send to complete
await new Promise((r) => setTimeout(r, 100));
assert.ok(aliceSends.length > 0, 'alice should have outgoing messages');
console.log('[2/4] Local change triggers sync: PASS');

// 3. Relay Alice's message to Bob
for (const msg of aliceSends) {
	await bob.handleIncoming(DOC_ID, msg);
}
aliceSends = [];
console.log('Bob sees:', bobText.toString());
assert.strictEqual(bobText.toString(), 'Hello from Alice');
console.log('[3/4] Peer sync convergence: PASS');

// 4. Bob makes a change — Alice receives it
bobText.insert(bobText.length, ' and Bob');
assert.strictEqual(bobText.toString(), 'Hello from Alice and Bob');
await new Promise((r) => setTimeout(r, 100));
assert.ok(bobSends.length > 0, 'bob should have outgoing messages');

for (const msg of bobSends) {
	await alice.handleIncoming(DOC_ID, msg);
}
bobSends = [];

assert.strictEqual(aliceText.toString(), 'Hello from Alice and Bob');
assert.strictEqual(bobText.toString(), 'Hello from Alice and Bob');
console.log('[4/4] Bidirectional sync: PASS');

// Cleanup
alice.destroy();
bob.destroy();

console.log('\n=== All sync engine checks passed ===');
