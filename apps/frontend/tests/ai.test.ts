import * as assert from 'node:assert';
import { parseSSE } from '../src/lib/ai';

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


console.log('ai parseSSE: PASS');
