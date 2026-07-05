// Markdown round-trip verification.
// Exercises htmlToMarkdown (turndown) + markdownToHtml (markdown-it).

import { htmlToMarkdown, markdownToHtml } from './src/markdown.ts';
import * as assert from 'node:assert';

const original = `# Hello World

This is a **bold** statement with *italic* text and \`inline code\`.

## Section Two

- Bullet item one
- Bullet item two
  1. Nested ordered
  2. Second nested

> A blockquote with **strong** text

\`\`\`
const x = 42;
console.log(x);
\`\`\`

End of document.
`;

console.log('=== Markdown Round-Trip Verification ===\n');

// 1. MD → HTML
const html = markdownToHtml(original);
assert.ok(html.includes('<h1>'), 'should have h1');
assert.ok(html.includes('<strong>'), 'should have strong');
assert.ok(html.includes('<ul>'), 'should have ul');
assert.ok(html.includes('<blockquote>'), 'should have blockquote');
assert.ok(html.includes('<code>'), 'should have code');
console.log('[1/3] Markdown → HTML: PASS');

// 2. HTML → MD
const roundTripped = htmlToMarkdown(html);
assert.ok(roundTripped.includes('# Hello World'), 'should preserve heading');
assert.ok(roundTripped.includes('**bold**'), 'should preserve bold');
assert.ok(roundTripped.includes('*italic*'), 'should preserve italic');
assert.ok(roundTripped.includes('`inline code`'), 'should preserve inline code');
	assert.ok(roundTripped.includes('-'), 'should preserve bullet marker');
assert.ok(roundTripped.includes('> A blockquote'), 'should preserve blockquote');
assert.ok(roundTripped.includes('```'), 'should preserve code block');
console.log('[2/3] HTML → Markdown: PASS');

// 3. Double round-trip: MD → HTML → MD → HTML → MD
const html2 = markdownToHtml(roundTripped);
const md2 = htmlToMarkdown(html2);
assert.ok(md2.includes('# Hello World'), 'double round-trip should preserve heading');
assert.ok(md2.includes('**bold**'), 'double round-trip should preserve bold');
console.log('[3/3] Double round-trip: PASS\n');

console.log('Original:');
console.log(original);
console.log('---');
console.log('Round-tripped:');
console.log(roundTripped);
console.log('\n=== All markdown checks passed ===');
