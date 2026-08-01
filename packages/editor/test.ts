import { htmlToMarkdown, rewriteTaskItems } from './src/markdown.ts';
import * as assert from 'node:assert';
import { marked } from 'marked';

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

console.log('Result:');
console.log(md);
console.log('\n=== All checks passed ===');
