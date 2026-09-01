// Toggle block — collapsible content section (Obsidian/Notion-style).
// Custom node view (pure DOM, no native <details>): a chevron button toggles
// a `collapsed` attribute that persists in the doc JSON. The summary row
// (toggleSummary node) stays visible while collapsed; the body is hidden
// with CSS so ProseMirror keeps rendering into it.

import { Node, mergeAttributes } from '@tiptap/core';
import { TextSelection } from '@tiptap/pm/state';

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

	// Backspace at the start of a toggle must remove it (the summary is
	// defining-ish, so ProseMirror's default backspace refuses to cross it).
	// At the summary start the toggle unwraps, keeping its content — at the
	// start of an empty body it is deleted outright.
	addKeyboardShortcuts() {
		return {
			Backspace: () => {
				const editor = this.editor;
				const { selection } = editor.state;
				if (!selection.empty) return false;
				const $from = selection.$from;
				const togglePos = $from.before(1);
				if (togglePos < 0) return false;
				const toggle = editor.state.doc.nodeAt(togglePos);
				if (!toggle || toggle.type.name !== 'toggleBlock') return false;

				// Caret at the very start of the summary → unwrap.
				if ($from.parent.type.name === 'toggleSummary') {
					if ($from.parentOffset > 0) return false;
					const body = toggle.content.content.slice(1);
					const tr = editor.state.tr;
					tr.delete(togglePos, togglePos + toggle.nodeSize);
					// ponytail: raw replaceWith — chain().insertContentAt() silently
					// drops block content inserted at a document boundary.
					if (body.length > 0) tr.replaceWith(togglePos, togglePos, body);
					tr.setSelection(TextSelection.near(tr.doc.resolve(togglePos + (body.length > 0 ? 1 : 0))));
					editor.view.dispatch(tr);
					return true;
				}

				// Caret at the start of an empty single-paragraph body → delete.
				if (toggle.childCount === 2 && $from.parentOffset === 0 && $from.parent === toggle.child(1)) {
					const body = toggle.child(1);
					if (body.isTextblock && body.content.size === 0) {
						editor.chain().focus().deleteRange({ from: togglePos, to: togglePos + toggle.nodeSize }).run();
						return true;
					}
				}
				return false;
			},
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
	// Not selectable as a node: otherwise the base Backspace chain's
	// selectNodeBackward grabs it at the toggle boundary and swallows the
	// key, so the toggle could never be deleted by keyboard. The ToggleBlock
	// extension's own Backspace shortcut handles that case instead.
	selectable: false,

	parseHTML() {
		return [{ tag: 'summary' }, { tag: 'div[data-toggle-summary]' }];
	},

	renderHTML({ HTMLAttributes }) {
		return ['div', mergeAttributes(HTMLAttributes, { 'data-toggle-summary': '', class: 'toggle-summary' }), 0];
	},
});
