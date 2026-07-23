import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';

export interface PageLinkEvent {
	query: string;
	range: { from: number; to: number };
}

export const PageLinkPluginKey = new PluginKey<PageLinkEvent | null>('page-link');

export const PageLink = Extension.create({
	name: 'pageLink',

	addProseMirrorPlugins() {
		return [
			new Plugin<PageLinkEvent | null>({
				key: PageLinkPluginKey,
				state: {
					init() { return null; },
					apply(tr, prev) {
						if (!tr.docChanged) return prev;
						const { selection } = tr;
						if (!selection.empty) return null;

						const pos = selection.$from;
						const textBefore = pos.parent.textContent.slice(0, pos.parentOffset);

						const match = textBefore.match(/\[\[([^\]]*)$/);
						if (match) {
							return {
								query: match[1],
								range: {
									from: pos.start() + pos.parentOffset - match[0].length,
									to: pos.pos,
								},
							};
						}

						return null;
					},
				},
			}),
		];
	},
});
