// Data-format matrix: insert every element the editor can produce, serialize
// the document to Markdown, then parse it back. Reports anything that loses
// its node type or content, so export/import gaps are visible instead of
// silently dropping data.
//
// Run: npm run test:insert -w @enclave/editor   (builds both entrypoints)
import { JSDOM } from 'jsdom';
import * as assert from 'node:assert';

const dom = new JSDOM('<div id="root"></div>', { pretendToBeVisual: true });
(globalThis as Record<string, unknown>).window = dom.window;
(globalThis as Record<string, unknown>).document = dom.window.document;
Object.defineProperty(globalThis, 'navigator', { value: dom.window.navigator, configurable: true });
for (const name of [
	'Element', 'Node', 'Text', 'HTMLElement', 'SVGElement', 'DocumentFragment', 'Comment',
	'Event', 'CustomEvent', 'MouseEvent', 'KeyboardEvent', 'FocusEvent', 'InputEvent', 'UIEvent',
	'Range', 'MutationObserver', 'getSelection', 'requestAnimationFrame', 'cancelAnimationFrame',
	'DOMRect', 'DOMTokenList', 'CSSStyleDeclaration', 'getComputedStyle',
]) {
	const v = (dom.window as unknown as Record<string, unknown>)[name];
	if (typeof v === 'function' || typeof v === 'object') {
		(globalThis as Record<string, unknown>)[name] = v;
	}
}

import { Editor } from '@tiptap/core';
import type { JSONContent } from '@tiptap/core';
import { editorExtensions } from './src/extensions.ts';
import { jsonToMarkdown, markdownToJson } from './src/markdown.ts';

const root = dom.window.document.getElementById('root')!;
const ed = new Editor({ element: root as unknown as HTMLElement, extensions: editorExtensions() });

function reset() {
	ed.commands.setContent({ type: 'doc', content: [{ type: 'paragraph' }] });
}

function nodeTypes(json: unknown, out = new Set<string>()): Set<string> {
	if (!json || typeof json !== 'object') return out;
	if (Array.isArray(json)) {
		for (const c of json) nodeTypes(c, out);
		return out;
	}
	const j = json as JSONContent;
	if (typeof j.type === 'string') out.add(j.type);
	if (j.marks) for (const m of j.marks) if (m.type) out.add(m.type);
	if (j.content) nodeTypes(j.content, out);
	return out;
}

interface Row {
	name: string;
	inserted: boolean;
	md: string;
	markdownHas: boolean;
	roundTripHas: boolean;
	notes: string[];
}
const rows: Row[] = [];
let insertErrors = 0;

async function probe(
	name: string,
	want: string,
	insert: (ed: Editor) => void,
	mdToken?: string,
): Promise<void> {
	reset();
	const notes: string[] = [];
	try {
		insert(ed);
	} catch (e) {
		insertErrors++;
		rows.push({ name, inserted: false, md: `INSERT ERROR: ${e}`, markdownHas: false, roundTripHas: false, notes: ["insert threw: " + (e as Error).message] });
		return;
	}
	const json = ed.getJSON();
	const inserted = nodeTypes(json).has(want);
	if (!inserted) notes.push(`node ${want} missing after insert`);
	let md = '';
	try {
		md = await jsonToMarkdown(json);
	} catch (e) {
		notes.push(`jsonToMarkdown threw: ${e}`);
	}
	const markdownHas = mdToken ? md.includes(mdToken) : true;
	if (!markdownHas) notes.push(`markdown missing "${mdToken}"`);
	let roundTripHas = false;
	try {
		const back = await markdownToJson(md);
		roundTripHas = nodeTypes(back).has(want);
	} catch (e) {
		notes.push(`markdownToJson threw: ${e}`);
	}
	if (!roundTripHas) notes.push(`round-trip lost ${want}`);
	rows.push({ name, inserted, md, markdownHas, roundTripHas, notes });
}

// ── Serialization-only probes ────────────────────────────────────────────────
// Nodes with Svelte node views cannot mount under jsdom, so build their JSON
// directly and test the data-format path (their UI is exercised in the browser
// by the audit's editorElements scenario).
async function probeJson(name: string, want: string, node: JSONContent, mdToken?: string): Promise<void> {
	const notes: string[] = [];
	let md = '';
	try {
		md = await jsonToMarkdown({ type: 'doc', content: [node] });
	} catch (e) {
		notes.push(`jsonToMarkdown threw: ${e}`);
	}
	const markdownHas = mdToken ? md.includes(mdToken) : true;
	if (!markdownHas) notes.push(`markdown missing "${mdToken}"`);
	let roundTripHas = false;
	try {
		const back = await markdownToJson(md);
		roundTripHas = nodeTypes(back).has(want);
	} catch (e) {
		notes.push(`markdownToJson threw: ${e}`);
	}
	if (!roundTripHas) notes.push(`round-trip lost ${want}`);
	rows.push({ name, inserted: true, md, markdownHas, roundTripHas, notes });
}

// ── Structural blocks ────────────────────────────────────────────────────────
await probe('paragraph', 'paragraph', (e) => { e.chain().focus().setParagraph().insertContent('Hello paragraph').run(); }, 'Hello paragraph');
await probe('heading1', 'heading', (e) => { e.chain().focus().setHeading({ level: 1 }).insertContent('Title One').run(); }, '# Title One');
await probe('heading2', 'heading', (e) => { e.chain().focus().setHeading({ level: 2 }).insertContent('Title Two').run(); }, '## Title Two');
await probe('heading3', 'heading', (e) => { e.chain().focus().setHeading({ level: 3 }).insertContent('Title Three').run(); }, '### Title Three');
await probe('bulletList', 'bulletList', (e) => { e.chain().focus().toggleBulletList().insertContent('bullet item').run(); }, '- bullet item');
await probe('orderedList', 'orderedList', (e) => { e.chain().focus().toggleOrderedList().insertContent('ordered item').run(); }, '1. ordered item');
await probe('taskList', 'taskList', (e) => { e.chain().focus().toggleTaskList().insertContent('task item').run(); }, '- [ ] task item');
await probe('blockquote', 'blockquote', (e) => { e.chain().focus().toggleBlockquote().insertContent('quoted').run(); }, '> quoted');
await probe('callout', 'callout', (e) => { e.chain().focus().toggleCallout().insertContent('callout body').run(); }, 'callout body');
await probe('toggleBlock', 'toggleBlock', (e) => { e.chain().focus().setToggleBlock().insertContent('toggle body').run(); }, 'toggle body');
await probeJson('codeBlock', 'codeBlock', {
	type: 'codeBlock',
	attrs: { language: 'js' },
	content: [{ type: 'text', text: 'const a = 1' }],
}, '```js');
await probe('horizontalRule', 'horizontalRule', (e) => { e.chain().focus().setHorizontalRule().run(); }, '---');
await probe('markdownTable', 'table', (e) => { e.chain().focus().insertTable({ rows: 3, cols: 2, withHeaderRow: true }).run(); }, '|');
await probeJson('database', 'database', {
	type: 'database',
	attrs: {
		data: JSON.stringify({
			columns: [{ id: 'c1', name: 'Task', type: 'text' }, { id: 'c2', name: 'Status', type: 'select' }],
			rows: [{ id: 'r1', cells: { c1: 'Ship database', c2: 'Doing' } }],
		}),
	},
}, 'Ship database');
await probeJson('image', 'image', {
	type: 'image',
	attrs: { src: 'https://example.com/pic.png', alt: 'A pic', caption: 'Cap' },
}, 'pic.png');
await probeJson('filePdf', 'file', {
	type: 'file',
	attrs: { path: '/tmp/report.pdf', name: 'report.pdf' },
}, 'report.pdf');
await probeJson('bookmark', 'bookmark', {
	type: 'bookmark',
	attrs: { url: 'https://example.com/guide', title: 'The Guide' },
}, 'https://example.com/guide');
await probeJson('pageEmbed', 'pageEmbed', {
	type: 'pageEmbed',
	attrs: { docId: 'doc-2', title: 'Roadmap' },
}, 'Roadmap');
// `[[` page links and `@` mentions both insert the same mention node.
await probe('pageLink(mention)', 'mention', (e) => {
	e.chain().focus().insertContent({ type: 'mention', attrs: { docId: 'doc-2', title: 'Roadmap' } }).run();
}, 'Roadmap');
await probe('mention', 'mention', (e) => {
	e.chain().focus().insertContent({ type: 'mention', attrs: { docId: 'doc-2', title: 'Roadmap' } }).run();
}, 'Roadmap');

// ── Inline marks ─────────────────────────────────────────────────────────────
await probe('bold', 'bold', (e) => { e.chain().focus().insertContent('bold text').selectAll().toggleBold().run(); }, '**bold text**');
await probe('italic', 'italic', (e) => { e.chain().focus().insertContent('italic text').selectAll().toggleItalic().run(); }, '*italic text*');
await probe('strike', 'strike', (e) => { e.chain().focus().insertContent('struck text').selectAll().toggleStrike().run(); }, '~~struck text~~');
await probe('inlineCode', 'code', (e) => { e.chain().focus().insertContent('some code').selectAll().toggleCode().run(); }, '`some code`');
await probe('link', 'link', (e) => {
	e.chain().focus().insertContent('docs link').selectAll().setLink({ href: 'https://example.com' }).run();
}, '[docs link](https://example.com)');

// ── Report ───────────────────────────────────────────────────────────────────
console.log('\n=== Editor element / data-format matrix ===\n');
let losses = 0;
for (const r of rows) {
	const clean = r.inserted && r.markdownHas && r.roundTripHas && r.notes.length === 0;
	if (!clean) losses++;
	console.log(`${clean ? 'ok  ' : 'LOSS'} ${r.name.padEnd(14)} ${r.notes.join('; ')}`);
}
console.log(`\n${rows.length - losses}/${rows.length} elements round-trip cleanly (${insertErrors} insert error(s))`);

ed.destroy();
if (insertErrors > 0) process.exit(1);
console.log('element matrix complete');
