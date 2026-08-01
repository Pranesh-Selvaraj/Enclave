import * as assert from 'node:assert';
import { extractLinks } from '../src/lib/graphLinks.ts';

const titleToId = new Map([
	['Wiki', 'doc-2'],
	['Me', 'doc-1'],
]);

const content = {
	type: 'doc',
	content: [
		{
			type: 'paragraph',
			content: [
				{ type: 'text', text: 'See ', marks: [{ type: 'link', attrs: { href: '/doc/doc-2' } }] },
				{ type: 'text', text: ' here [[Wiki]] and self [[Me]]' },
			],
		},
		{ type: 'pageEmbed', attrs: { docId: 'doc-3' } },
		{
			type: 'paragraph',
			content: [{ type: 'text', text: 'plain' }],
		},
	],
};

const links = extractLinks(content, titleToId, 'doc-1').sort((a, b) => a.target.localeCompare(b.target));
assert.deepEqual(links, [
	{ source: 'doc-1', target: 'doc-2' },
	{ source: 'doc-1', target: 'doc-3' },
], 'link mark, wiki title (deduped), and page embed resolve; self-link and plain text are skipped');
console.log('extractLinks: PASS', JSON.stringify(links));
