import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { SlashCommand } from './extensions/slash-command.js';
import { PageLink } from './extensions/page-link.js';
import { Callout } from './extensions/callout.js';
import { ToggleBlock, ToggleSummary } from './extensions/toggle-block.js';
import { Database } from './extensions/database.js';
import { Image } from './extensions/image.js';
import { PageEmbed } from './extensions/page-embed.js';

/** Shared editor extension list — used by the editor and by HTML/JSON converters. */
export function editorExtensions() {
	return [
		StarterKit.configure({
			heading: { levels: [1, 2, 3] },
		}),
		Placeholder,
		TaskList,
		TaskItem.configure({ nested: true }),
		Callout,
		ToggleBlock,
		ToggleSummary,
		Database,
		Image,
		PageEmbed,
		SlashCommand,
		PageLink,
	];
}
