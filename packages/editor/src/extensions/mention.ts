import { Node, mergeAttributes } from '@tiptap/core';
import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';

// ── Inline mention node (atom): renders as a chip, exports as [[title]] ─────

export const Mention = Node.create({
	name: 'mention',
	group: 'inline',
	inline: true,
	atom: true,
	selectable: true,

	addAttributes() {
		return {
			docId: { default: null },
			title: { default: '' },
		};
	},

	parseHTML() {
		return [{ tag: 'span[data-mention]' }];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-mention': '',
				'data-doc-id': node.attrs.docId,
				'data-title': node.attrs.title,
				class: 'mention-chip',
			}),
			`@${node.attrs.title}`,
		];
	},

	renderText({ node }) {
		return `@${node.attrs.title}`;
	},
});

// ── @ trigger: emits {query, range} for the mention menu ────────────────────

export interface MentionEvent {
	query: string;
	range: { from: number; to: number };
}

export const MentionPluginKey = new PluginKey<MentionEvent | null>('mention');

export const MentionTrigger = Extension.create({
	name: 'mentionTrigger',

	addProseMirrorPlugins() {
		return [
			new Plugin<MentionEvent | null>({
				key: MentionPluginKey,
				state: {
					init() { return null; },
					apply(tr, prev) {
						if (!tr.docChanged) return prev;
						const { selection } = tr;
						if (!selection.empty) return null;

						const pos = selection.$from;
						const textBefore = pos.parent.textContent.slice(0, pos.parentOffset);

						// @word — must follow start-of-line or whitespace so
						// emails and mid-word @ don't trigger.
						const match = textBefore.match(/(?:^|\s)@([^\s]*)$/);
						if (match && match[1] !== '') {
							const from = pos.start() + pos.parentOffset - match[0].length + 1;
							return {
								query: match[1],
								range: { from, to: pos.pos },
							};
						}

						return null;
					},
				},
			}),
		];
	},
});
