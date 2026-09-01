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

import { Editor, Node } from '@tiptap/core';
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
		console.log(`ok   ${c.label}${viewError ? ' (doc changed; node view mount skipped: ' + String(viewError).slice(0,120) + ')' : ''}`);
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

// The TableMenu shows when isActive('table') — verify the caret lands in a
// cell right after insertTable.
ed.commands.setContent('<p>x</p>');
clickAt(1);
ed.chain().focus().insertTable({ rows: 2, cols: 2, withHeaderRow: true }).run();
assert.ok(ed.isActive('table'), 'caret inside inserted table → TableMenu visible');
console.log('ok   table menu visibility (isActive("table") after insertTable)');

// ── Interaction: task checkbox click toggles the checked attribute ──
ed.commands.setContent('<p>todo</p>');
clickAt(1);
ed.chain().focus().toggleTaskList().run();

ed.commands.setContent('<p>todo</p>');
clickAt(1);
ed.chain().focus().toggleTaskList().run();
const taskInput = ed.view.dom.querySelector('ul[data-type="taskList"] li input[type="checkbox"]') as HTMLInputElement | null;
assert.ok(taskInput, 'task item renders a checkbox input');
taskInput!.checked = true;
taskInput!.dispatchEvent(new dom.window.Event('change', { bubbles: true }));
let json = JSON.stringify(ed.getJSON());
assert.ok(json.includes('"checked":true'), `checkbox click should check the task: ${json.slice(0, 160)}`);
taskInput!.checked = false;
taskInput!.dispatchEvent(new dom.window.Event('change', { bubbles: true }));
json = JSON.stringify(ed.getJSON());
assert.ok(!json.includes('"checked":true'), 'second click unchecks the task');
console.log('ok   task checkbox click toggles checked');

// ── Interaction: toggle chevron click collapses/expands and persists ──
ed.commands.setContent('<p>x</p>');
clickAt(1);
ed.chain().focus().setToggleBlock().run();
const chevron = ed.view.dom.querySelector('.toggle-block .toggle-chevron') as HTMLButtonElement | null;
assert.ok(chevron, 'toggle block renders a chevron button');
chevron!.click();
json = JSON.stringify(ed.getJSON());
assert.ok(json.includes('"collapsed":true'), `chevron click should collapse the toggle: ${json.slice(0, 160)}`);
chevron!.click();
json = JSON.stringify(ed.getJSON());
assert.ok(!json.includes('"collapsed":true'), 'second chevron click expands the toggle');
console.log('ok   toggle chevron click collapses/expands (state persists in doc)');


// ── Backspace deletes an empty toggle (the user-reported bug) ──
function pressBackspace() {
	const ev = new dom.window.KeyboardEvent('keydown', { key: 'Backspace', bubbles: true, cancelable: true });
	ed.view.dom.dispatchEvent(ev);
}


// Empty toggle: place the caret in the empty body paragraph, Backspace
// deletes the whole toggle (the user-reported bug).
ed.commands.setContent('<p>before</p><div data-toggle><div data-toggle-summary>Toggle</div><p></p></div>');
let toggleBodyPos = 0;
ed.state.doc.descendants((node, pos) => {
	if (node.type.name === 'toggleBlock') {
		toggleBodyPos = pos + 1 + node.child(0).nodeSize + 1; // inside the first body block
		return false;
	}
	return true;
});
ed.commands.setTextSelection(toggleBodyPos);
pressBackspace();
json = JSON.stringify(ed.getJSON());
assert.ok(!json.includes('toggleBlock'), `Backspace at empty toggle body start should delete it: ${json.slice(0, 200)}`);
console.log('ok   backspace deletes an empty toggle');

// ── Backspace at the summary start unwraps, keeping the body content ──
ed.commands.setContent('<p>before</p><div data-toggle><div data-toggle-summary>Summary</div><p>Body text</p></div>');
let summaryPos = 0;
ed.state.doc.descendants((node, pos) => {
	if (node.type.name === 'toggleSummary') {
		summaryPos = pos + 1;
		return false;
	}
	return true;
});
ed.commands.setTextSelection(summaryPos);
pressBackspace();
json = JSON.stringify(ed.getJSON());
assert.ok(!json.includes('toggleBlock'), `Backspace at summary start should unwrap the toggle: ${json.slice(0, 200)}`);
assert.ok(json.includes('Body text'), 'unwrapped toggle keeps its body content');
console.log('ok   backspace at summary start unwraps the toggle (content kept)');


// ── Events from inside a node view must not reach ProseMirror ──
// The database/bookmark/image views contain real inputs. Without stopEvent,
// a keypress while the PM selection is a NodeSelection on the block
// REPLACES the block with the typed character — the "block disappears when
// I type" bug. Reproduce with a minimal atom node + input, then verify the
// stopEvent version keeps the doc intact.
function makeAtomEditor(withStopEvent: boolean) {
	const atom = Node.create({
		name: 'testAtom',
		group: 'block',
		atom: true,
		parseHTML: () => [{ tag: 'div[data-test-atom]' }],
		renderHTML: () => ['div', { 'data-test-atom': '' }] as never,
		addNodeView() {
			return () => {
				const dom = document.createElement('div');
				dom.setAttribute('data-test-atom', '');
				const input = document.createElement('input');
				input.type = 'text';
				dom.appendChild(input);
				const view = { dom };
				if (withStopEvent) {
					(view as { stopEvent?: () => boolean }).stopEvent = () => true;
				}
				return view;
			};
		},
	});
	const e = new Editor({ element: root as unknown as HTMLElement, extensions: [...editorExtensions(), atom] });
	e.commands.setContent('<p>a</p><div data-test-atom></div><p>b</p>');
	return e;
}

function pressKeyInInput(e: { view: { dom: HTMLElement } }) {
	const input = e.view.dom.querySelector('input')!;
	const kp = new dom.window.KeyboardEvent('keypress', { bubbles: true, cancelable: true, key: 'x' });
	Object.defineProperty(kp, 'charCode', { value: 120 });
	Object.defineProperty(kp, 'keyCode', { value: 120 });
	input.dispatchEvent(kp);
}

// Without stopEvent: the atom is replaced by the typed character.
const edNoStop = makeAtomEditor(false);
let atomPosNoStop = 0;
edNoStop.state.doc.descendants((n, pos) => {
	if (n.type.name === 'testAtom') { atomPosNoStop = pos; return false; }
	return true;
});
edNoStop.commands.setNodeSelection(atomPosNoStop);
pressKeyInInput(edNoStop);
json = JSON.stringify(edNoStop.getJSON());
assert.ok(!json.includes('testAtom'), `without stopEvent typing must delete the block: ${json.slice(0, 120)}`);
edNoStop.destroy();

// With stopEvent: the doc is untouched, the input keeps the keystroke.
const edStop = makeAtomEditor(true);
let atomPosStop = 0;
edStop.state.doc.descendants((n, pos) => {
	if (n.type.name === 'testAtom') { atomPosStop = pos; return false; }
	return true;
});
edStop.commands.setNodeSelection(atomPosStop);
pressKeyInInput(edStop);
json = JSON.stringify(edStop.getJSON());
assert.ok(json.includes('testAtom'), `with stopEvent the block must survive typing: ${json.slice(0, 120)}`);
console.log('ok   stopEvent keeps blocks intact when typing in node-view inputs');
edStop.destroy();

ed.destroy();
if (failed > 0) {
	console.log(`\n${failed} insert check(s) FAILED`);
	process.exit(1);
}
console.log('\nAll insert checks passed');
