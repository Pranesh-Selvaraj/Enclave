// Pure helpers for linked databases — node-safe (no .svelte imports) so the
// test suite can exercise the doc-walk logic. Positions are real ProseMirror
// document positions (size-aware), so they can feed setNodeMarkup directly.

import type { JSONContent } from '@tiptap/core';
import type { DBData } from './extensions/database.js';

export interface DatabaseRef {
	pos: number;
	id: string;
	name: string;
	rowCount: number;
	data: Partial<DBData>;
}

/** ProseMirror node size from JSON: text = length, block = 1 + content size. */
function sizeOf(n: JSONContent): number {
	return n.type === 'text' ? (n.text?.length ?? 0) : 1 + (n.content ?? []).reduce((s, c) => s + sizeOf(c), 0);
}

export function parseData(node: JSONContent): Partial<DBData> {
	try {
		return JSON.parse(String(node.attrs?.data ?? ''));
	} catch {
		return {};
	}
}

/** Walk a doc for database blocks, with their document positions. */
export function walkDatabases(doc: JSONContent): { node: JSONContent; pos: number }[] {
	const out: { node: JSONContent; pos: number }[] = [];
	const visit = (n: JSONContent, pos: number) => {
		if (n.type === 'database') out.push({ node: n, pos });
		let offset = 0;
		for (const c of n.content ?? []) {
			visit(c, pos + 1 + offset);
			offset += sizeOf(c);
		}
	};
	visit(doc, 0);
	return out;
}

/** Databases on the page, with names for the picker. */
export function listDatabases(doc: JSONContent): DatabaseRef[] {
	return walkDatabases(doc).map(({ node, pos }) => {
		const d = parseData(node);
		return {
			pos,
			id: d.id ?? '',
			name: d.columns?.[0]?.name ?? 'Untitled database',
			rowCount: d.rows?.length ?? 0,
			data: d,
		};
	});
}
