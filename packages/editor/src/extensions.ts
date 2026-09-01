import StarterKit from '@tiptap/starter-kit';
import CodeBlock from '@tiptap/extension-code-block';
import Link from '@tiptap/extension-link';
import Table from '@tiptap/extension-table';
import TableRow from '@tiptap/extension-table-row';
import TableHeader from '@tiptap/extension-table-header';
import TableCell from '@tiptap/extension-table-cell';
import { mount, unmount } from 'svelte';
import CodeBlockView from './blocks/CodeBlockView.svelte';
import Placeholder from '@tiptap/extension-placeholder';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { SlashCommand } from './extensions/slash-command.js';
import { PageLink } from './extensions/page-link.js';
import { Mention, MentionTrigger } from './extensions/mention.js';
import { Callout } from './extensions/callout.js';
import { ToggleBlock, ToggleSummary } from './extensions/toggle-block.js';
import { Database } from './extensions/database.js';
import { Image } from './extensions/image.js';
import { File } from './extensions/file.js';
import { PageEmbed } from './extensions/page-embed.js';
import { Bookmark } from './extensions/bookmark.js';
import { DragHandle } from './extensions/drag-handle.js';

/** Shared editor extension list — used by the editor and by HTML/JSON converters. */
export function editorExtensions() {
	return [
		StarterKit.configure({
			heading: { levels: [1, 2, 3] },
			// The toolbar node view below extends codeBlock — drop starter-kit's.
			codeBlock: false,
		}),
		Placeholder,
		TaskList,
		TaskItem.configure({ nested: true }),
		// External links: clickable <a> marks. openOnClick keeps the standard
		// behavior (Cmd/Ctrl+click opens the system browser, plain click
		// keeps the caret editable); linkOnPaste turns pasted URLs into links.
		Link.configure({ openOnClick: true, linkOnPaste: true }),
		// Simple markdown-style tables (GFM round-trip via marked).
		Table.configure({ resizable: false }),
		TableRow,
		TableHeader,
		TableCell,
		Callout,
		ToggleBlock,
		ToggleSummary,
		Database,
		Image,
		File,
		PageEmbed,
		Bookmark,
		SlashCommand,
		PageLink,
		Mention,
		MentionTrigger,
		DragHandle,
		// Replaces starter-kit's bare codeBlock with the toolbar node view.
		CodeBlock.extend({
			addNodeView() {
				return ({ node, editor, getPos }) => {
					const dom = document.createElement('div');
					const contentDOM = document.createElement('pre');
					contentDOM.className = 'cb-pre';
					let lang = node.attrs.language ?? 'plaintext';
					const view = mount(CodeBlockView, {
						target: dom,
						props: {
							language: lang,
							contentDOM,
							onLanguageChange: (next: string) => {
								const pos = getPos();
								if (pos == null) return;
								lang = next;
								editor.view.dispatch(
									editor.state.tr.setNodeMarkup(pos, undefined, {
										language: next === 'plaintext' ? null : next,
									})
								);
							},
						},
					});
					return {
						dom,
						contentDOM,
						update(newNode) {
							const next = newNode.attrs.language ?? 'plaintext';
							if (next === lang) return true;
							lang = next;
							// Svelte 5: no $set — components expose update functions.
							(view as unknown as { setLanguage: (l: string) => void }).setLanguage(next);
							return true;
						},
						destroy() {
							unmount(view);
						},
					};
				};
			},
		}),
	];
}
