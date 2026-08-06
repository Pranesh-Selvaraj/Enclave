import * as assert from 'node:assert';
import { parseSSE, cosineSimilarity } from '../src/lib/ollama';

const { deltas, rest } = parseSSE(
	'data: {"choices":[{"delta":{"content":"Hel"}}]}\ndata: {"choices":[{"d',
);
assert.deepStrictEqual(deltas, ['Hel'], 'complete data: lines yield their deltas');
assert.strictEqual(rest, 'data: {"choices":[{"d', 'truncated tail is kept for the next chunk');
const next = parseSSE('data: {"choices":[{"d' + 'elta":{"content":"lo"}}]}\n');
assert.deepStrictEqual(next.deltas, ['lo'], 'split token reassembles');
assert.strictEqual(next.rest, '', 'buffer drains');
const { deltas: none } = parseSSE(
	'event: message\n\nevent: done\n\ndata: [DONE]\ndata: {"choices":[{"delta":{}}]}\n',
);
assert.deepStrictEqual(none, [], 'non-data lines, [DONE] and empty deltas are skipped');

const v = [1, 0, 0];
assert.strictEqual(cosineSimilarity(v, [1, 0, 0]), 1, 'identical vectors');
assert.strictEqual(cosineSimilarity(v, [0, 1, 0]), 0, 'orthogonal vectors');
assert.ok(cosineSimilarity(v, [2, 0, 0]) > 0.999, 'parallel vectors ~ 1');
assert.strictEqual(cosineSimilarity([], []), 0, 'empty vectors are neutral');
assert.strictEqual(cosineSimilarity(v, [1, 1]), 0, 'mismatched lengths are neutral');

console.log('ollama parseSSE + cosineSimilarity: PASS');
