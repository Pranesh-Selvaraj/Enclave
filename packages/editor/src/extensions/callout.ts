// Callout block — colored info/warning/tip boxes (Notion-style).
// Renders as a styled div with an icon and background color.

import { Node, mergeAttributes } from '@tiptap/core';

interface CalloutOptions {
	HTMLAttributes: Record<string, any>;
}

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		callout: {
			setCallout: (attrs?: { type?: string }) => ReturnType;
			toggleCallout: (attrs?: { type?: string }) => ReturnType;
		};
	}
}

export const Callout = Node.create<CalloutOptions>({
	name: 'callout',

	content: 'block+',
	group: 'block',
	defining: true,

	addAttributes() {
		return {
			type: { default: 'info', parseHTML: (el) => el.getAttribute('data-type') || 'info' },
		};
	},

	parseHTML() {
		return [{ tag: 'div[data-callout]' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['div', mergeAttributes(HTMLAttributes, { 'data-callout': '' }), 0];
	},

	addCommands() {
		return {
			setCallout:
				(attrs) =>
				({ commands }) =>
					commands.wrapIn(this.type, attrs),
			toggleCallout:
				(attrs) =>
				({ commands }) =>
					commands.toggleWrap(this.type, attrs),
		};
	},
});
