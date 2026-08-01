// Database block — typed columns, rows, sort/filter (AFFiNE-style).
// Persists as a single atom node; the table JSON lives in the node's
// `data` attribute and saves with the regular doc content flow.

import { Node } from '@tiptap/core';
import { mount, unmount } from 'svelte';
import DatabaseView from '../blocks/DatabaseView.svelte';

export interface DBColumn {
	id: string;
	name: string;
	type: 'text' | 'number' | 'checkbox' | 'date';
}

export interface DBRow {
	id: string;
	cells: Record<string, string | boolean>;
}

export interface DBData {
	columns: DBColumn[];
	rows: DBRow[];
}

function uid(): string {
	return Math.random().toString(36).slice(2, 10);
}

export function defaultDatabaseData(): DBData {
	return {
		columns: [{ id: uid(), name: 'Name', type: 'text' }],
		rows: [{ id: uid(), cells: {} }],
	};
}

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		database: {
			setDatabase: () => ReturnType;
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
					onData: (json: string) => {
						const pos = getPos();
						if (pos == null) return;
						current = json;
						editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { data: json }));
					},
				},
			});
			return {
				dom,
				update(newNode) {
					if (newNode.attrs.data === current) return true;
					current = newNode.attrs.data;
					(view as unknown as { $set: (p: Record<string, unknown>) => void }).$set({ data: current });
					return true;
				},
				destroy() {
					unmount(view);
				},
			};
		};
	},
});
