import { htmlToMarkdown } from './src/markdown.ts';
import * as assert from 'node:assert';

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

console.log('Result:');
console.log(md);
console.log('\n=== All checks passed ===');
