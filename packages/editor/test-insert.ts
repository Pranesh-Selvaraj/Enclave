// Headless check of the editor's insert commands (the same actions the
// context-menu Insert flyout and the slash menu run). Creates a real Editor
// with the production extensions under jsdom, moves the caret the way the
// context menu does (setTextSelection at the click point), then runs each
// action and asserts the doc changed as expected.
import { JSDOM } from 'jsdom';
import * as assert from 'node:assert';

const dom = new JSDOM('<div id="root"></div>', { pretendToBeVisual: true });
// ProseMirror reads these globals at import time.
(globalThis as Record<string, unknown>).window = dom.window;
(globalThis as Record<string, unknown>).document = dom.window.document;
Object.defineProperty(globalThis, 'navigator', { value: dom.window.navigator, configurable: true });
// DOM globals the svelte client runtime and ProseMirror reference directly.
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
import { editorExtensions } from './src/extensions.ts';

const root = dom.window.document.getElementById('root')!;
const ed = new Editor({ element: root as unknown as HTMLElement, extensions: editorExtensions() });

// The context-menu flow: right-click moves the caret via setTextSelection
// (unless the click lands inside an existing selection), then the action
// runs with chain().focus().
function clickAt(pos: number) {
	const { from, to } = ed.state.selection;
	if (from >= pos || pos >= to) ed.commands.setTextSelection(pos);
}

interface Case {
	label: string;
	pos: number; // where the "right-click" lands
	setup?: () => void;
	action: (e: typeof ed) => void;
	check: (json: { type: string; content?: unknown[] }) => boolean;
}

const cases: Case[] = [
	{
		label: 'markdown table',
		pos: 1,
		action: (e) => e.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
		check: (json) => JSON.stringify(json).includes('"table"'),
	},
	{
		label: 'bullet list',
		pos: 1,
		action: (e) => e.chain().focus().toggleBulletList().run(),
		check: (json) => JSON.stringify(json).includes('bulletList'),
	},
	{
		label: 'numbered list',
		pos: 1,
		action: (e) => e.chain().focus().toggleOrderedList().run(),
		check: (json) => JSON.stringify(json).includes('orderedList'),
	},
	{
		label: 'task list',
		pos: 1,
		action: (e) => e.chain().focus().toggleTaskList().run(),
		check: (json) => JSON.stringify(json).includes('taskList'),
	},
	{
		label: 'toggle block',
		pos: 1,
		action: (e) => e.chain().focus().setToggleBlock().run(),
		check: (json) => JSON.stringify(json).includes('toggleBlock'),
	},
	{
		label: 'quote',
		pos: 1,
		action: (e) => e.chain().focus().toggleBlockquote().run(),
		check: (json) => JSON.stringify(json).includes('blockquote'),
	},
	{
		label: 'callout',
		pos: 1,
		action: (e) => e.chain().focus().toggleCallout().run(),
		check: (json) => JSON.stringify(json).includes('callout'),
	},
	{
		label: 'divider',
		pos: 1,
		action: (e) => e.chain().focus().setHorizontalRule().run(),
		check: (json) => JSON.stringify(json).includes('horizontalRule'),
	},
	{
		label: 'code block',
		pos: 1,
		action: (e) => e.chain().focus().toggleCodeBlock().run(),
		check: (json) => JSON.stringify(json).includes('codeBlock'),
	},
	{
		label: 'paragraph (turn into)',
		pos: 1,
		setup: () => ed.commands.setContent('<h2>alpha beta gamma</h2>'),
		action: (e) => e.chain().focus().setParagraph().run(),
		check: (json) => JSON.stringify(json).includes('"paragraph"'),
	},
	{
		label: 'heading 2 (turn into)',
		pos: 1,
		action: (e) => e.chain().focus().setHeading({ level: 2 }).run(),
		check: (json) => JSON.stringify(json).includes('heading'),
	},
	{
		label: 'bold (format)',
		pos: 3,
		setup: () => ed.commands.setTextSelection({ from: 1, to: 5 }),
		action: (e) => e.chain().focus().toggleBold().run(),
		check: (json) => JSON.stringify(json).includes('bold'),
	},
];

let failed = 0;
for (const c of cases) {
	ed.commands.setContent('<p>alpha beta gamma</p>');
	c.setup?.();
	clickAt(c.pos);
	const before = JSON.stringify(ed.getJSON());
	let viewError: unknown = null;
	try {
		c.action(ed);
	} catch (e) {
		// Node views (code block etc.) need real layout; their mount errors
		// are a jsdom limitation, not a command failure — the doc change is
		// what we assert.
		viewError = e;
	}
	const after = JSON.stringify(ed.getJSON());
	const ok = after !== before && c.check(JSON.parse(after) as never);
	if (!ok) {
		failed++;
		console.log(`FAIL ${c.label}: before=${before} after=${after} ${viewError ? '(view error: ' + String(viewError).slice(0, 80) + ')' : ''}`);
	} else {
		console.log(`ok   ${c.label}${viewError ? ' (doc changed; node view mount skipped)' : ''}`);
	}
}

// Table manipulation commands (the missing table menu) must exist and work.
ed.commands.setContent('<p>x</p>');
clickAt(1);
ed.chain().focus().insertTable({ rows: 2, cols: 2, withHeaderRow: true }).run();
const rowsBefore = (JSON.stringify(ed.getJSON()).match(/"tableRow"/g) ?? []).length;
ed.chain().focus().addRowAfter().run();
const rowsAfter = (JSON.stringify(ed.getJSON()).match(/"tableRow"/g) ?? []).length;
assert.ok(rowsAfter === rowsBefore + 1, `addRowAfter: ${rowsBefore} → ${rowsAfter}`);
const colsBefore = (JSON.stringify(ed.getJSON()).match(/"tableCell"/g) ?? []).length;
ed.chain().focus().addColumnAfter().run();
const colsAfter = (JSON.stringify(ed.getJSON()).match(/"tableCell"/g) ?? []).length;
assert.ok(colsAfter === colsBefore + 2, `addColumnAfter: ${colsBefore} → ${colsAfter}`);

ed.destroy();
if (failed > 0) {
	console.log(`\n${failed} insert check(s) FAILED`);
	process.exit(1);
}
console.log('\nAll insert checks passed');

// The TableMenu shows when isActive('table') — verify the caret lands in a
// cell right after insertTable.
ed.commands.setContent('<p>x</p>');
clickAt(1);
ed.chain().focus().insertTable({ rows: 2, cols: 2, withHeaderRow: true }).run();
assert.ok(ed.isActive('table'), 'caret inside inserted table → TableMenu visible');
console.log('ok   table menu visibility (isActive("table") after insertTable)');
