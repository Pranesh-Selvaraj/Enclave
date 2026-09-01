// File block — a stored vault attachment (PDFs today). Paste or insert a
// PDF and it lands in the document's attachment folder; the node view shows
// an inline preview where the webview supports it plus an "Open" button for
// the system viewer (Android ACTION_VIEW via tauri-plugin-opener).

import { Node, mergeAttributes } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { mount, unmount } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import type { Editor } from '@tiptap/core';
import FileView from '../blocks/FileView.svelte';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		file: {
			setFile: (attrs: { path: string; name: string }) => ReturnType;
		};
	}
}

async function importFile(editor: Editor, file: File) {
	try {
		const bytes = new Uint8Array(await file.arrayBuffer());
		const abs = await invoke<string>('save_attachment', {
			documentId: editor.storage.image.docId,
			filename: file.name || `file-${Date.now()}.pdf`,
			data: Array.from(bytes),
		});
		editor.chain().focus().setFile({ path: abs, name: file.name }).run();
	} catch (e) {
		console.error('Failed to import file:', e);
	}
}

export const File = Node.create({
	name: 'file',

	group: 'block',
	atom: true,
	selectable: true,

	addAttributes() {
		return {
			path: { default: '' },
			name: { default: '' },
		};
	},

	parseHTML() {
		return [{ tag: 'div[data-file]' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['div', mergeAttributes(HTMLAttributes, { 'data-file': '' }), '\u200b'];
	},

	addCommands() {
		return {
			setFile:
				(attrs) =>
				({ commands }) =>
					commands.insertContent({ type: this.name, attrs }),
		};
	},

	addNodeView() {
		return ({ node, editor, getPos }) => {
			const dom = document.createElement('div');
			let current = { path: node.attrs.path, name: node.attrs.name };
			const view = mount(FileView, {
				target: dom,
				props: { ...current },
			});
			return {
				dom,
				update(newNode) {
					const next = { path: newNode.attrs.path, name: newNode.attrs.name };
					if (next.path === current.path && next.name === current.name) return true;
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

	addProseMirrorPlugins() {
		const ext = this;
		return [
			new Plugin({
				props: {
					handlePaste(_view, event) {
						const files = Array.from(event.clipboardData?.files ?? []).filter((f) =>
							f.type === 'application/pdf' || /\.pdf$/i.test(f.name)
						);
						if (files.length === 0) return false;
						event.preventDefault();
						for (const f of files) void importFile(ext.editor, f);
						return true;
					},
				},
			}),
		];
	},
});
