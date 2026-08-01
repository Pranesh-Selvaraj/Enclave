// Image block — src/alt attrs, paste-to-insert from clipboard.
// The page sets editor.storage.image.docId so pasted files land in the
// right document's attachment folder.

import { Node, mergeAttributes } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type { Editor } from '@tiptap/core';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		image: {
			setImage: (attrs: { src: string; alt?: string }) => ReturnType;
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
		};
	},

	parseHTML() {
		return [{ tag: 'img[src]' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['img', mergeAttributes(HTMLAttributes, { draggable: 'false' })];
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
