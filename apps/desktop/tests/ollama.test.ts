import * as assert from 'node:assert';
import { parseChunks } from '../src/lib/ollama';

const { deltas, rest } = parseChunks(
	'{"message":{"content":"Hel"},"done":false}\n{"message":{"conten',
);
assert.deepStrictEqual(deltas, ['Hel'], 'complete lines yield their deltas');
assert.strictEqual(rest, '{"message":{"conten', 'truncated tail is kept for the next chunk');
const next = parseChunks('{"message":{"conten' + 't":"lo"},"done":false}\n');
assert.deepStrictEqual(next.deltas, ['lo'], 'split token reassembles');
assert.strictEqual(next.rest, '', 'buffer drains');
const { deltas: none } = parseChunks('   \nnot-json\n{"message":{}}\n');
assert.deepStrictEqual(none, [], 'blank, malformed and empty-content lines are skipped');
console.log('ollama parseChunks: PASS');
