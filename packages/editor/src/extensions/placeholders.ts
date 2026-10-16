import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';

/**
 * Per-block empty hints (issue #155).
 *
 * The stock Placeholder extension only decorates a top-level textblock, so an
 * empty callout, quote, toggle body, table header or code block looked like a
 * stray artifact with no indication of what to type. This plugin decorates the
 * empty textblock that holds the caret — including nested ones — with a
 * `data-placeholder` attribute that the editor CSS paints via ::before.
 */
const PLACEHOLDER_KEY = new PluginKey('blockPlaceholders');

function hintFor(type: string, parent: string, attrs: Record<string, unknown>): string | null {
	if (type === 'heading') return `Heading ${attrs.level}`;
	if (type === 'codeBlock') return 'Code…';
	if (parent === 'callout') return 'Callout text…';
	if (parent === 'toggleBlock') return 'Toggle content…';
	if (parent === 'blockquote') return 'Quote…';
	if (parent === 'tableHeader') return 'Header';
	if (parent === 'tableCell') return null;
	return null;
}

export interface BlockPlaceholdersOptions {
	/** Hint for a plain empty paragraph (callers can pass their own). */
	fallback: string;
}

export const BlockPlaceholders = Extension.create<BlockPlaceholdersOptions>({
	name: 'blockPlaceholders',

	addOptions() {
		return { fallback: 'Write something…' };
	},

	addProseMirrorPlugins() {
		const fallback = this.options.fallback;
		return [
			new Plugin({
				key: PLACEHOLDER_KEY,
				props: {
					decorations(state) {
						const { selection } = state;
						if (!selection.empty) return null;
						const { $from } = selection;
						let depth = $from.depth;
						while (depth > 0 && !$from.node(depth).isTextblock) depth--;
						if (depth === 0) return null;
						const node = $from.node(depth);
						if (node.content.size > 0) return null;
						const parent = depth > 1 ? $from.node(depth - 1).type.name : 'doc';
						const text = hintFor(node.type.name, parent, node.attrs);
						if (text === '') return null;
						const label = text ?? fallback;
						const pos = $from.before(depth);
						return DecorationSet.create(state.doc, [
							Decoration.node(pos, pos + node.nodeSize, { 'data-placeholder': label, class: 'is-empty' }),
						]);
					},
				},
			}),
		];
	},
});
