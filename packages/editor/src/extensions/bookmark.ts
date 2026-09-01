// Bookmark block — a link card (AFFiNE "bookmark" / Notion "bookmark").
// Paste a bare URL or use the slash command; title is editable in the card.
// ponytail: no network fetch — favicon is a letter avatar, title defaults to
// the URL. Upgrade to metadata fetching (OpenGraph) when offline-first no
// longer matters.

import { Node, mergeAttributes } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { mount, unmount } from 'svelte';
import BookmarkView from '../blocks/BookmarkView.svelte';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		bookmark: {
			setBookmark: (url: string) => ReturnType;
		};
	}
}

const URL_RE = /^https?:\/\/\S+\.\S+/i;

export const Bookmark = Node.create({
	name: 'bookmark',

	group: 'block',
	atom: true,
	defining: true,

	addAttributes() {
		return {
			url: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-url') ?? '',
				renderHTML: (attrs) => ({ 'data-url': attrs.url }),
			},
			title: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-title') ?? '',
				renderHTML: (attrs) => ({ 'data-title': attrs.title }),
			},
		};
	},

	parseHTML() {
		return [{ tag: 'div[data-bookmark]' }];
	},

	renderHTML({ HTMLAttributes }) {
		// ponytail: ZWSP child — turndown skips blank nodes (see database.ts)
		return ['div', mergeAttributes(HTMLAttributes, { 'data-bookmark': '' }), '\u200b'];
	},

	addCommands() {
		return {
			setBookmark:
				(url) =>
				({ commands }) =>
					commands.insertContent({ type: this.name, attrs: { url, title: '' } }),
		};
	},

	addNodeView() {
		return ({ node, editor, getPos }) => {
			const dom = document.createElement('div');
			let current = { url: node.attrs.url, title: node.attrs.title };
			const view = mount(BookmarkView, {
				target: dom,
				props: {
					url: current.url,
					title: current.title,
					onTitleChange: (title: string) => {
						const pos = getPos();
						if (pos == null) return;
						current = { url: current.url, title };
						editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { url: current.url, title }));
					},
				},
			});
			return {
				dom,
				update(newNode) {
					if (newNode.attrs.url === current.url && newNode.attrs.title === current.title) return true;
					current = { url: newNode.attrs.url, title: newNode.attrs.title };
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

	addProseMirrorPlugins() {
		return [
			new Plugin({
				props: {
					handlePaste(view, event) {
						const text = event.clipboardData?.getData('text/plain') ?? '';
						// Only bare-URL pastes with no HTML companion become bookmarks;
						// pasting a rich link (markdown, styled) stays a link.
						if (!text || !URL_RE.test(text.trim()) || event.clipboardData?.getData('text/html')) {
							return false;
						}
						const url = text.trim();
						event.preventDefault();
						view.dispatch(view.state.tr.replaceSelectionWith(
							view.state.schema.nodes.bookmark.create({ url, title: '' })
						));
						return true;
					},
				},
			}),
		];
	},
});
