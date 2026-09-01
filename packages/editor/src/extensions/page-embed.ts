// Page embed — inline reference to another page (AFFiNE "linked doc").
// The card shows the page title and opens it on click; while docId is
// empty the NodeView shows a picker fed by get_page_list.

import { Node, mergeAttributes } from '@tiptap/core';
import { mount, unmount } from 'svelte';
import PageEmbedView from '../blocks/PageEmbedView.svelte';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		pageEmbed: {
			setPageEmbed: () => ReturnType;
		};
	}
}

export const PageEmbed = Node.create({
	name: 'pageEmbed',

	group: 'block',
	atom: true,
	defining: true,

	addAttributes() {
		return {
			docId: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-doc-id') ?? '',
				renderHTML: (attrs) => ({ 'data-doc-id': attrs.docId }),
			},
			title: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-title') ?? '',
				renderHTML: (attrs) => ({ 'data-title': attrs.title }),
			},
		};
	},

	parseHTML() {
		return [{ tag: 'div[data-page-embed]' }];
	},

	renderHTML({ HTMLAttributes }) {
		// ponytail: ZWSP child — turndown skips blank nodes (see database.ts)
		return ['div', mergeAttributes(HTMLAttributes, { 'data-page-embed': '' }), '\u200b'];
	},

	addCommands() {
		return {
			setPageEmbed:
				() =>
				({ commands }) =>
					commands.insertContent({ type: this.name, attrs: { docId: '', title: '' } }),
		};
	},

	addNodeView() {
		return ({ node, editor, getPos }) => {
			const dom = document.createElement('div');
			let current = { docId: node.attrs.docId, title: node.attrs.title };
			const view = mount(PageEmbedView, {
				target: dom,
				props: {
					docId: current.docId,
					title: current.title,
					onPick: (docId: string, title: string) => {
						const pos = getPos();
						if (pos == null) return;
						current = { docId, title };
						editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { docId, title }));
					},
				},
			});
			return {
				dom,
				// The page picker is UI, not an editor edit.
				stopEvent: () => true,
				update(newNode) {
					if (newNode.attrs.docId === current.docId && newNode.attrs.title === current.title) return true;
					current = { docId: newNode.attrs.docId, title: newNode.attrs.title };
					// Svelte 5: no $set — returning false lets ProseMirror
					// recreate the view with the new attrs (undo/redo/load).
					return false;
				},
				destroy() {
					unmount(view);
				},
			};
		};
	},
});
