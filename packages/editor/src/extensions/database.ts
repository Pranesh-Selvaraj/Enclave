// Database block — typed columns, rows, sort/filter, views, linked databases
// (AFFiNE-style). Persists as a single atom node; the table JSON lives in the
// node's `data` attribute and saves with the regular doc content flow.
// A linked database is a node with `sourceId` set; its `data` holds a mirror
// of the source's DBData, refreshed from the source on every transaction.

import { Node } from '@tiptap/core';
import { mount, unmount } from 'svelte';
import DatabaseView from '../blocks/DatabaseView.svelte';

export const DB_TYPES = [
	'text',
	'number',
	'checkbox',
	'date',
	'select',
	'multiSelect',
	'url',
	'email',
	'progress',
	'createdAt',
	'updatedAt',
	'relation',
] as const;
export type DBType = (typeof DB_TYPES)[number];

export interface DBColumn {
	id: string;
	name: string;
	type: DBType;
	options?: string[];
}

export interface DBRow {
	id: string;
	cells: Record<string, string | boolean | string[]>;
	createdAt?: string;
	updatedAt?: string;
}

export interface DBData {
	id?: string;
	columns: DBColumn[];
	rows: DBRow[];
	view?: 'table' | 'kanban' | 'list' | 'gallery' | 'timeline';
	groupBy?: string | null;
	sort?: { colId: string; dir: 'asc' | 'desc' } | null;
	filters?: Record<string, string>;
	/// Row density — persisted with the block so it survives reloads.
	density?: 'comfortable' | 'compact';
}

function uid(): string {
	return Math.random().toString(36).slice(2, 10);
}

export function defaultDatabaseData(): DBData {
	const now = new Date().toISOString();
	return {
		id: uid(),
		columns: [{ id: uid(), name: 'Name', type: 'text' }],
		rows: [{ id: uid(), cells: {}, createdAt: now, updatedAt: now }],
		density: 'comfortable',
	};
}

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		database: {
			setDatabase: () => ReturnType;
			setLinkedDatabase: (sourceId: string, data: DBData) => ReturnType;
		};
	}
}

export const Database = Node.create({
	name: 'database',

	group: 'block',
	atom: true,
	defining: true,

	addAttributes() {
		return {
			data: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-database') || '',
				renderHTML: (attrs) => ({ 'data-database': attrs.data }),
			},
			sourceId: {
				default: '',
				// ponytail: not rendered to HTML — a linked db round-trips as a
				// plain mirror through copy/markdown. Re-link after export.
				renderHTML: () => ({}),
			},
		};
	},

	parseHTML() {
		return [{ tag: 'div[data-database]' }];
	},

	renderHTML({ HTMLAttributes }) {
		// ponytail: ZWSP text child — turndown short-circuits blank nodes to
		// blankRule, so a childless div would never reach the database export
		// rule. The node view replaces this DOM in the editor, so it's invisible.
		return ['div', HTMLAttributes, '\u200b'];
	},

	addCommands() {
		return {
			setDatabase:
				() =>
				({ commands }) =>
					commands.insertContent({
						type: this.name,
						attrs: { data: JSON.stringify(defaultDatabaseData()) },
					}),
			setLinkedDatabase:
				(sourceId, data) =>
				({ commands }) =>
					commands.insertContent({
						type: this.name,
						attrs: { sourceId, data: JSON.stringify(data) },
					}),
		};
	},

	addNodeView() {
		return ({ node, editor, getPos }) => {
			const dom = document.createElement('div');
			let current = node.attrs.data;
			const view = mount(DatabaseView, {
				target: dom,
				props: {
					data: current,
					readOnly: Boolean(node.attrs.sourceId),
					onDeleteBlock: () => {
						const pos = getPos();
						if (pos == null) return;
						const size = editor.state.doc.nodeAt(pos)?.nodeSize ?? node.nodeSize;
						editor.view.dispatch(editor.state.tr.deleteRange(pos, pos + size));
					},
					onData: (json: string) => {
						const pos = getPos();
						if (pos == null) return;
						current = json;
						editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { data: json }));
					},
				},
			});

			const mirrorSource = () => {
				if (!node.attrs.sourceId) return;
				const sourceId = node.attrs.sourceId;
				let sourceData: DBData | null = null;
				editor.state.doc.descendants((n) => {
					if (n.type.name !== 'database') return true;
					if (n.attrs.sourceId) return true; // linked node, not a source
					let d: Partial<DBData> = {};
					try {
						d = JSON.parse(n.attrs.data ?? '');
					} catch {
						d = {};
					}
					if (d.id === sourceId) {
						sourceData = d as DBData;
						return false;
					}
					return true;
				});
				if (!sourceData) return;
				const json = JSON.stringify(sourceData);
				if (json === current) return;
				const pos = getPos();
				if (pos == null) return;
				current = json;
				editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { data: json }));
			};

			if (node.attrs.sourceId) {
				editor.on('transaction', mirrorSource);
			}

			return {
				dom,
				// All events from inside the widget are UI, not editor edits —
				// without this, typing in a cell bubbles to ProseMirror, which
				// replaces the database node with the typed text (the block
				// "disappears").
				stopEvent: () => true,
				update(newNode) {
					if (newNode.attrs.data === current) return true;
					current = newNode.attrs.data;
					// Svelte 5: no $set — components expose update functions.
					(view as unknown as { applyData: (d: string) => void }).applyData(current);
					return true;
				},
				destroy() {
					editor.off('transaction', mirrorSource);
					unmount(view);
				},
			};
		};
	},
});
