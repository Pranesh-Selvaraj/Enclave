import { htmlToMarkdown, rewriteTaskItems } from './src/markdown.ts';
import * as assert from 'node:assert';
import { marked } from 'marked';
import { listDatabases, walkDatabases } from './src/dbLink.ts';

// markdownToJson's generateJSON half needs a DOM, so check the marked half
// (the same transform) here in node.

const html = `<h1>Hello World</h1>
<p>This is a <strong>bold</strong> statement with <em>italic</em> text and <code>inline code</code>.</p>
<h2>Section Two</h2>
<ul>
<li>Bullet item one</li>
<li>Bullet item two</li>
</ul>
<blockquote>
<p>A blockquote with <strong>strong</strong> text</p>
</blockquote>
<pre><code>const x = 42;
console.log(x);
</code></pre>
<p>End of document.</p>`;

console.log('=== HTML → Markdown Test ===\n');

const md = htmlToMarkdown(html);
assert.ok(md.includes('# Hello World'), 'should have heading');
assert.ok(md.includes('**bold**'), 'should have bold');
assert.ok(md.includes('*italic*'), 'should have italic');
assert.ok(md.includes('`inline code`'), 'should have inline code');
assert.ok(md.includes('-'), 'should have bullet marker');
assert.ok(md.includes('> A blockquote'), 'should have blockquote');
assert.ok(md.includes('```'), 'should have code block');
console.log('HTML → Markdown: PASS\n');

// Database block → Markdown table
const dbData = {
	columns: [
		{ id: 'c1', name: 'Task', type: 'text' },
		{ id: 'c2', name: 'Priority', type: 'number' },
	],
	rows: [
		{ id: 'r1', cells: { c1: 'Ship database', c2: '1' } },
		{ id: 'r2', cells: { c1: 'Write docs', c2: '2' } },
	],
};
const dbHtml = `<div data-database="${JSON.stringify(dbData).replace(/"/g, '&quot;')}">\u200b</div>`;
const dbMd = htmlToMarkdown(dbHtml);
assert.ok(dbMd.includes('| Task | Priority |'), 'table header row');
assert.ok(dbMd.includes('| --- | --- |'), 'table separator row');
assert.ok(dbMd.includes('| Ship database | 1 |'), 'table data row');
assert.ok(dbMd.includes('| Write docs | 2 |'), 'table second row');
assert.ok(!htmlToMarkdown('<div data-database="not-json"></div>').includes('|'), 'bad json yields no table');
console.log('Database export: PASS\n');

// Markdown import pipeline (marked + task-item rewrite)
const importMd = `# Title

- [ ] todo one
- [x] done one
- [ ] *italic task*

\`\`\`js
const a = 1;
\`\`\`

| A | B |
| - | - |
| 1 | 2 |`;
const importHtml = rewriteTaskItems(marked.parse(importMd, { async: false }) as string);
assert.ok(importHtml.includes('<li data-type="taskItem" data-checked="false">'), 'unchecked task item');
assert.ok(importHtml.includes('<li data-type="taskItem" data-checked="true">'), 'checked task item');
assert.ok(importHtml.includes('<em>italic task</em>'), 'inline markdown inside task');
assert.ok(importHtml.includes('<pre><code class="language-js">'), 'fenced code with language');
assert.ok(importHtml.includes('<table>'), 'markdown table');
assert.ok(!/<li><input /.test(importHtml), 'no bare checkbox remains');
console.log('Markdown import pipeline: PASS\n');

// Bookmark block → markdown link
const bmHtml = `<div data-bookmark="" data-url="https://example.com/guide" data-title="The Guide">\u200b</div>`;
const bmMd = htmlToMarkdown(bmHtml);
assert.ok(bmMd.includes('[The Guide](<https://example.com/guide>)'), 'bookmark as link');
assert.ok(!htmlToMarkdown('<div data-bookmark="" data-url="">\u200b</div>').includes('['), 'empty bookmark yields nothing');
console.log('Bookmark export: PASS\n');

// Code block language round-trip (fenced style keeps the language tag)
const cbHtml = `<pre><code class="language-js">const a = 1;</code></pre>`;
const cbMd = htmlToMarkdown(cbHtml);
assert.ok(cbMd.includes('```js'), 'code fence keeps language');
assert.ok(cbMd.includes('const a = 1;'), 'code content preserved');
console.log('Code block export: PASS\n');

// Plain links, ordered lists, task items (hand-rolled serializer)
const linkMd = htmlToMarkdown('<p>See <a href="https://example.com">docs</a>.</p>');
assert.ok(linkMd.includes('[docs](https://example.com)'), 'plain link');
const olMd = htmlToMarkdown('<ol><li>First</li><li>Second</li></ol>');
assert.ok(olMd.includes('1. First') && olMd.includes('2. Second'), 'ordered list');
const taskMd = htmlToMarkdown('<ul><li data-type="taskItem" data-checked="true">done</li><li data-type="taskItem" data-checked="false">todo</li></ul>');
assert.ok(taskMd.includes('- [x] done') && taskMd.includes('- [ ] todo'), 'task item checkboxes');
const nestedMd = htmlToMarkdown('<ul><li>one<ul><li>nested</li></ul></li></ul>');
assert.ok(nestedMd.includes('- one\n  - nested'), 'nested list indentation');
console.log('Links/lists/tasks export: PASS\n');

// Round-trip through marked (mirrors the app's import→export HTML shapes)
const rtMd = `# RT

- [ ] task

1. one

**bold** *em* \`code\` [link](https://x.com)

> quote

\`\`\`js
const a = 1;
\`\`\``;
const rt = htmlToMarkdown(rewriteTaskItems(marked.parse(rtMd, { async: false }) as string));
for (const [frag, label] of [
	['# RT', 'heading'],
	['- [ ] task', 'task item'],
	['1. one', 'ordered item'],
	['**bold** *em* `code` [link](https://x.com)', 'inline'],
	['> quote', 'blockquote'],
	['```js', 'fenced code'],
	['const a = 1;', 'code body'],
] as const) {
	assert.ok(rt.includes(frag), `round-trip keeps ${label}`);
}
console.log('Round-trip through marked: PASS\n');

// Linked database doc-walk: positions must be real PM positions
const linkedDoc = {
	type: 'doc',
	content: [
		{ type: 'paragraph', content: [{ type: 'text', text: 'hello' }] }, // size 6 → pos 0, next block at 7
		{
			type: 'database',
			attrs: {
				data: JSON.stringify({
					id: 'db1',
					columns: [{ id: 'c1', name: 'Tasks', type: 'text' }],
					rows: [{ id: 'r1', cells: {} }, { id: 'r2', cells: {} }],
				}),
			},
		}, // pos 7
		{ type: 'paragraph', content: [] }, // pos 8
		{
			type: 'database',
			attrs: { data: JSON.stringify({ columns: [{ id: 'c1', name: 'Old', type: 'text' }], rows: [] }) },
		}, // pos 9, no id
	],
};
const refs = listDatabases(linkedDoc);
assert.strictEqual(refs.length, 2, 'two databases found');
assert.strictEqual(refs[0].pos, 7, 'db after text paragraph sits at pos 7');
assert.strictEqual(refs[0].name, 'Tasks', 'name from first column');
assert.strictEqual(refs[0].rowCount, 2, 'row count');
assert.strictEqual(refs[0].id, 'db1', 'id parsed');
assert.strictEqual(refs[1].pos, 9, 'second db position');
assert.strictEqual(refs[1].id, '', 'old db has no id — gets stamped on link');
assert.deepStrictEqual(
	walkDatabases(linkedDoc).map((w) => w.pos),
	[7, 9],
	'walkDatabases positions agree'
);
console.log('dbLink doc-walk: PASS\n');

console.log('Result:');
console.log(md);
console.log('\n=== All checks passed ===');
