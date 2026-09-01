// Image block — src/alt attrs, paste-to-insert from clipboard.
// The page sets editor.storage.image.docId so pasted files land in the
// right document's attachment folder.

import { Node, mergeAttributes } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { mount, unmount } from 'svelte';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type { Editor } from '@tiptap/core';
import ImageView from '../blocks/ImageView.svelte';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		image: {
			setImage: (attrs: { src: string; alt?: string; caption?: string }) => ReturnType;
		};
	}
}

async function importImage(editor: Editor, file: File) {
	try {
		const bytes = new Uint8Array(await file.arrayBuffer());
		const abs = await invoke<string>('save_attachment', {
			documentId: editor.storage.image.docId,
			filename: file.name || `pasted-${Date.now()}.png`,
			data: Array.from(bytes),
		});
		editor.chain().focus().setImage({ src: convertFileSrc(abs), alt: file.name }).run();
	} catch (e) {
		console.error('Failed to import image:', e);
	}
}

export const Image = Node.create({
	name: 'image',

	group: 'block',
	atom: true,
	selectable: true,

	addStorage() {
		return { docId: '' };
	},

	addAttributes() {
		return {
			src: { default: null },
			alt: { default: '' },
			caption: {
				default: '',
				parseHTML: (el) => (el as HTMLElement).dataset.caption ?? '',
				renderHTML: (attrs) => ({ 'data-caption': attrs.caption }),
			},
		};
	},

	parseHTML() {
		return [{ tag: 'figure[data-img]' }];
	},

	renderHTML({ HTMLAttributes }) {
		// ponytail: ZWSP child — the markdown serializer skips blank nodes
		return ['figure', mergeAttributes(HTMLAttributes, { 'data-img': '' }), '\u200b'];
	},

	addNodeView() {
		return ({ node, editor, getPos }) => {
			const dom = document.createElement('div');
			let current = { src: node.attrs.src, alt: node.attrs.alt, caption: node.attrs.caption ?? '' };
			const view = mount(ImageView, {
				target: dom,
				props: {
					...current,
					onCaptionChange: (caption: string) => {
						const pos = getPos();
						if (pos == null) return;
						current = { ...current, caption };
						editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, current));
					},
				},
			});
			return {
				dom,
				update(newNode) {
					const next = { src: newNode.attrs.src, alt: newNode.attrs.alt, caption: newNode.attrs.caption ?? '' };
					if (next.src === current.src && next.alt === current.alt && next.caption === current.caption) {
						return true;
					}
					current = next;
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

	addCommands() {
		return {
			setImage:
				(attrs) =>
				({ commands }) =>
					commands.insertContent({ type: this.name, attrs }),
		};
	},

	addProseMirrorPlugins() {
		const ext = this;
		return [
			new Plugin({
				props: {
					handlePaste(_view, event) {
						const images = Array.from(event.clipboardData?.files ?? []).filter((f) =>
							f.type.startsWith('image/')
						);
						if (images.length === 0) return false;
						event.preventDefault();
						for (const f of images) void importImage(ext.editor, f);
						return true;
					},
				},
			}),
		];
	},
});
