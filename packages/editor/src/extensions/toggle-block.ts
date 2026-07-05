// Toggle block — collapsible content section (like Notion's toggle).
// Renders as <details><summary>...</summary> content </details>.

import { Node, mergeAttributes } from '@tiptap/core';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		toggleBlock: {
			setToggleBlock: () => ReturnType;
		};
	}
}

export const ToggleBlock = Node.create({
	name: 'toggleBlock',

	content: 'toggleSummary block+',
	group: 'block',
	defining: true,

	parseHTML() {
		return [{ tag: 'details' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['details', mergeAttributes(HTMLAttributes, { 'data-toggle': '' }), 0];
	},

	addCommands() {
		return {
			setToggleBlock:
				() =>
				({ commands }) =>
					commands.insertContent({
						type: this.name,
						content: [{ type: 'toggleSummary', content: [{ type: 'text', text: 'Toggle' }] }, { type: 'paragraph' }],
					}),
		};
	},
});

export const ToggleSummary = Node.create({
	name: 'toggleSummary',

	content: 'inline*',
	group: 'toggleSummary',
	defining: true,

	parseHTML() {
		return [{ tag: 'summary' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['summary', mergeAttributes(HTMLAttributes), 0];
	},
});
