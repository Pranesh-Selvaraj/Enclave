import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';

interface SlashCommandEvent {
	query: string;
	range: { from: number; to: number };
}

export const SlashCommandPluginKey = new PluginKey<SlashCommandEvent | null>('slash-command');

/**
 * TipTap extension that detects "/" at the start of a line
 * and emits the query + range for the slash menu to consume.
 */
export const SlashCommand = Extension.create({
	name: 'slashCommand',

	addProseMirrorPlugins() {
		return [
			new Plugin<SlashCommandEvent | null>({
				key: SlashCommandPluginKey,
				state: {
					init() {
						return null;
					},
					apply(tr, prev) {
						if (!tr.docChanged) return prev;

						const { selection } = tr;
						if (!selection.empty) return null;

						const pos = selection.$from;
						const textBefore = pos.parent.textContent.slice(0, pos.parentOffset);

						// Check if "/"" is the first character after optional whitespace
						const match = textBefore.match(/^\/(\S*)$/);
						if (match && pos.parentOffset > 0) {
							return {
								query: match[1],
								range: { from: pos.start() + pos.parentOffset - textBefore.length, to: pos.pos },
							};
						}

						// Handle paragraph-internal slash
						if (textBefore.includes('/') && pos.parentOffset > 0) {
							const slashIndex = textBefore.lastIndexOf('/');
							const textAfterSlash = textBefore.slice(slashIndex + 1);
							// Only trigger if slash is at the start or after a space
							const beforeSlash = textBefore.slice(0, slashIndex);
							if (beforeSlash === '' || beforeSlash.endsWith(' ')) {
								return {
									query: textAfterSlash,
									range: {
										from: pos.start() + slashIndex,
										to: pos.pos,
									},
								};
							}
						}

						return null;
					},
				},
			}),
		];
	},
});
