<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { Icon } from '@enclave/ui';
	import { DragHandlePluginKey, type DragHandleState } from '../extensions/drag-handle.js';

	let {
		editor,
	}: {
		editor: Editor | undefined;
	} = $props();

	let visible = $state(false);
	let position = $state({ x: 0, y: 0 });
	let menuState = $state<DragHandleState | null>(null);

	function updatePosition() {
		const ed = editor;
		if (!ed || !menuState) return;
		const coords = ed.view.coordsAtPos(menuState.pos);
		position = {
			x: Math.min(Math.max(coords.left, 8), window.innerWidth - 190),
			y: Math.min(coords.bottom + 8, window.innerHeight - 200),
		};
	}

	function close() {
		const ed = editor;
		visible = false;
		menuState = null;
		if (ed) {
			ed.view.dispatch(ed.state.tr.setMeta(DragHandlePluginKey, null));
		}
	}

	function nodeSize() {
		return menuState?.size ?? null;
	}

	function duplicate() {
		const ed = editor;
		if (!ed || !menuState) return;
		const size = nodeSize();
		if (size == null) return close();
		ed.chain().focus().insertContentAt(menuState.pos + size, menuState.json as never, { updateSelection: false }).run();
		close();
	}

	function remove() {
		const ed = editor;
		if (!ed || !menuState) return;
		const size = nodeSize();
		if (size == null) return close();
		ed.chain().focus().deleteRange({ from: menuState.pos, to: menuState.pos + size }).run();
		close();
	}

	async function cut(copyOnly: boolean) {
		const ed = editor;
		if (!ed || !menuState) return;
		const text = (menuState.json as { textContent?: string }).textContent ?? '';
		try {
			await navigator.clipboard.writeText(text);
		} catch {
			// clipboard unavailable (webkitgtk) — skip copy, still cut
		}
		if (!copyOnly) remove();
		else close();
	}

	$effect(() => {
		const ed = editor;
		if (!ed) return;
		const onTx = () => {
			const st = DragHandlePluginKey.getState(ed.state);
			if (st?.open) {
				menuState = st;
				visible = true;
				updatePosition();
			} else if (visible) {
				visible = false;
				menuState = null;
			}
		};
		ed.on('transaction', onTx);
		const onScroll = () => {
			if (visible) updatePosition();
		};
		const scrollContainer = ed.view.dom.closest('.doc-editor') || ed.view.dom.parentElement;
		scrollContainer?.addEventListener('scroll', onScroll, { passive: true });
		return () => {
			ed.off('transaction', onTx);
			scrollContainer?.removeEventListener('scroll', onScroll);
		};
	});

	$effect(() => {
		const ed = editor;
		if (!ed || !visible) return;
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') {
				e.preventDefault();
				close();
			}
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});
</script>

{#if visible}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="dh-backdrop" onclick={close}></div>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="dh-menu"
		style="left: {position.x}px; top: {position.y}px;"
		role="menu"
		aria-label="Block menu"
		onclick={(e: MouseEvent) => e.stopPropagation()}
	>
		<button class="dh-item" role="menuitem" onclick={duplicate}>
			<Icon name="duplicate" size={14} />
			Duplicate
		</button>
		<button class="dh-item" role="menuitem" onclick={() => cut(true)}>
			<Icon name="copy" size={14} />
			Copy text
		</button>
		<button class="dh-item" role="menuitem" onclick={() => cut(false)}>
			<Icon name="cut" size={14} />
			Cut
		</button>
		<div class="dh-sep"></div>
		<button class="dh-item danger" role="menuitem" onclick={remove}>
			<Icon name="trash" size={14} />
			Delete
		</button>
	</div>
{/if}

<style>
	.dh-backdrop {
		position: fixed;
		inset: 0;
		z-index: 240;
	}

	.dh-menu {
		position: fixed;
		z-index: 301;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 5px;
		min-width: 150px;
	}

	.dh-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		font-family: inherit;
		text-align: left;
		padding: 7px 10px;
		border-radius: 6px;
		cursor: pointer;
	}
	.dh-item :global(svg) { color: var(--color-text-faint); }
	.dh-item:hover :global(svg) { color: var(--color-text); }

	.dh-item:hover {
		background: var(--color-surface-hover);
	}

	.dh-item.danger {
		color: var(--color-danger);
	}

	.dh-item.danger:hover {
		background: rgba(229, 83, 75, 0.12);
	}

	.dh-sep {
		height: 1px;
		background: var(--color-border);
		margin: 4px 6px;
	}

	:global(.drag-grip) {
		position: absolute;
		z-index: 20;
		width: 20px;
		height: 20px;
		display: none;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: 5px;
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
		font-size: 11px;
		cursor: pointer;
		padding: 0;
	}

	:global(.drag-grip:hover) {
		background: var(--color-accent-subtle);
		color: var(--color-accent);
	}
</style>
