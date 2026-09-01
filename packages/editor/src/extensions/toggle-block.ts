// Toggle block — collapsible content section (Obsidian/Notion-style).
// Custom node view (pure DOM, no native <details>): a chevron button toggles
// a `collapsed` attribute that persists in the doc JSON. The summary row
// (toggleSummary node) stays visible while collapsed; the body is hidden
// with CSS so ProseMirror keeps rendering into it.

import { Node, mergeAttributes } from '@tiptap/core';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		toggleBlock: {
			setToggleBlock: () => ReturnType;
		};
	}
}

function chevronSVG(collapsed: boolean): string {
	return (
		'<svg viewBox="0 0 16 16" width="15" height="15" fill="none" stroke="currentColor" ' +
		'stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
		`<path d="${collapsed ? 'M6 4l4 4-4 4' : 'M4 6l4 4 4-4'}"/>` +
		'</svg>'
	);
}

export const ToggleBlock = Node.create({
	name: 'toggleBlock',

	content: 'toggleSummary block+',
	group: 'block',
	defining: true,

	addAttributes() {
		return {
			collapsed: {
				default: false,
				parseHTML: (el) => el.getAttribute('data-collapsed') === 'true',
				renderHTML: (attrs) => ({ 'data-collapsed': attrs.collapsed ? 'true' : null }),
			},
		};
	},

	parseHTML() {
		return [{ tag: 'details' }, { tag: 'div[data-toggle]' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['div', mergeAttributes(HTMLAttributes, { 'data-toggle': '' }), 0];
	},

	addCommands() {
		return {
			setToggleBlock:
				() =>
				({ commands }) =>
					commands.insertContent({
						type: this.name,
						content: [
							{ type: 'toggleSummary', content: [{ type: 'text', text: 'Toggle' }] },
							{ type: 'paragraph' },
						],
					}),
		};
	},

	addNodeView() {
		return ({ node, editor, getPos }) => {
			const dom = document.createElement('div');
			dom.className = 'toggle-block';
			dom.setAttribute('data-toggle', '');

			const chevron = document.createElement('button');
			chevron.type = 'button';
			chevron.className = 'toggle-chevron';
			chevron.setAttribute('contenteditable', 'false');
			chevron.setAttribute('aria-label', node.attrs.collapsed ? 'Expand section' : 'Collapse section');
			chevron.innerHTML = chevronSVG(node.attrs.collapsed);

			const body = document.createElement('div');
			body.className = 'toggle-body';

			chevron.addEventListener('mousedown', (e) => e.preventDefault());
			chevron.addEventListener('click', () => {
				const pos = getPos();
				if (pos == null) return;
				// Read the current node at click time — the closure's `node` is
				// stale after the first toggle (consecutive clicks would both
				// set the same value).
				const current = editor.state.doc.nodeAt(pos);
				if (!current) return;
				const next = !current.attrs.collapsed;
				editor.view.dispatch(
					editor.state.tr.setNodeMarkup(pos, undefined, { collapsed: next }).scrollIntoView()
				);
			});

			dom.append(chevron, body);

			return {
				dom,
				contentDOM: body,
				update(updatedNode) {
					if (updatedNode.type.name !== 'toggleBlock') return false;
					dom.classList.toggle('collapsed', updatedNode.attrs.collapsed);
					chevron.innerHTML = chevronSVG(updatedNode.attrs.collapsed);
					chevron.setAttribute(
						'aria-label',
						updatedNode.attrs.collapsed ? 'Expand section' : 'Collapse section'
					);
					return true;
				},
			};
		};
	},
});

export const ToggleSummary = Node.create({
	name: 'toggleSummary',

	content: 'inline*',
	group: 'toggleSummary',
	defining: true,

	parseHTML() {
		return [{ tag: 'summary' }, { tag: 'div[data-toggle-summary]' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['div', mergeAttributes(HTMLAttributes, { 'data-toggle-summary': '', class: 'toggle-summary' }), 0];
	},
});
