// Drag handle — hovers a grip beside the block under the mouse; clicking it
// opens a block menu (duplicate / cut / copy / delete).
// ponytail: no real drag-reorder — ProseMirror's dragDrop is a plugin-shaped
// project of its own; the menu covers the 90% use case. Add drag-reorder
// when it's actually asked for.

import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

export interface DragHandleState {
	open: boolean;
	pos: number;
	size: number;
	json: Record<string, unknown>;
}

export const DragHandlePluginKey = new PluginKey<DragHandleState>('dragHandle');

interface FoundBlock {
	pos: number;
	size: number;
	json: Record<string, unknown>;
}

/** Outermost block containing the pointer (skipping the doc itself). */
function findBlockAt(view: EditorView, x: number, y: number): FoundBlock | null {
	const hit = view.posAtCoords({ left: x, top: y });
	if (!hit) return null;
	let found: FoundBlock | null = null;
	view.state.doc.descendants((n, p) => {
		if (found) return false;
		if (n.isBlock && n.type !== view.state.doc.type && p <= hit.pos && hit.pos <= p + n.nodeSize) {
			found = { pos: p, size: n.nodeSize, json: n.toJSON() };
			return false;
		}
		return true;
	});
	return found;
}

export const DragHandle = Extension.create({
	name: 'dragHandle',

	addProseMirrorPlugins() {
		const ext = this;
		let grip: HTMLButtonElement | null = null;
		let lastPos = -1;

		const hideGrip = () => {
			if (grip) grip.style.display = 'none';
			lastPos = -1;
		};

		return [
			new Plugin({
				key: DragHandlePluginKey,
				props: {
					handleDOMEvents: {
						mouseover(view, event) {
							const target = event.target as HTMLElement;
							if (!view.editable || target.closest('.drag-grip')) return false;
							const found = findBlockAt(view, event.clientX, event.clientY);
							if (!found) {
								hideGrip();
								return false;
							}
							if (found.pos === lastPos) return false;
							lastPos = found.pos;
							if (!grip) {
								grip = document.createElement('button');
								grip.className = 'drag-grip';
								grip.type = 'button';
								grip.setAttribute('aria-label', 'Block menu');
								// Inline SVG, not a text glyph — the old '⠿' Braille pattern
								// renders as two dot columns that read as "::" at small sizes.
								grip.innerHTML =
									'<svg viewBox="0 0 16 16" width="13" height="13" fill="currentColor" aria-hidden="true">' +
									[3, 8, 13]
										.map((y) => [4, 8, 12].map((x) => `<circle cx="${x}" cy="${y}" r="1.5"/>`).join(''))
										.join('') +
									'</svg>';
								grip.addEventListener('mousedown', (e) => {
									e.preventDefault();
									const st = DragHandlePluginKey.getState(ext.editor.state);
									const open = !st?.open;
									ext.editor.view.dispatch(
										ext.editor.state.tr.setMeta(DragHandlePluginKey, {
											open,
											...found,
										} satisfies DragHandleState)
									);
									hideGrip();
								});
								view.dom.appendChild(grip);
							}
							const coords = view.coordsAtPos(found.pos);
							grip.style.display = 'flex';
							grip.style.left = `${coords.left - 30}px`;
							grip.style.top = `${coords.top + 6}px`;
							return false;
						},
						mouseleave(view) {
							// Keep it visible while the menu is open.
							if (!DragHandlePluginKey.getState(view.state)?.open) hideGrip();
							return false;
						},
					},
				},
				view(_view) {
					return { destroy: () => grip?.remove() };
				},
			}),
		];
	},
});
