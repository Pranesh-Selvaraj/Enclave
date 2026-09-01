<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { Icon } from '@enclave/ui';

	let {
		editor,
	}: {
		editor: Editor | undefined;
	} = $props();

	let visible = $state(false);
	let position = $state({ x: 0, y: 0 });
	let isMouseOverMenu = $state(false);

	function updateMenu() {
		const ed = editor;
		if (!ed) return;
		// Show only while the caret/selection is inside a table.
		if (!ed.isActive('table')) {
			visible = false;
			return;
		}
		const { from, to } = ed.state.selection;
		const start = ed.view.coordsAtPos(from);
		const end = ed.view.coordsAtPos(to);

		position = {
			x: Math.min(Math.max((start.left + end.right) / 2 - 130, 8), window.innerWidth - 276),
			y: Math.min(Math.max(start.top - 44, 8), window.innerHeight - 52),
		};
		visible = true;
	}

	$effect(() => {
		const ed = editor;
		if (!ed) return;

		function onBlur() {
			if (!isMouseOverMenu) {
				visible = false;
			}
		}

		ed.on('selectionUpdate', updateMenu);
		ed.on('blur', onBlur);

		return () => {
			ed.off('selectionUpdate', updateMenu);
			ed.off('blur', onBlur);
		};
	});
</script>

{#if visible && editor}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="tb-menu"
		style="left: {position.x}px; top: {position.y}px;"
		onpointerdown={() => (isMouseOverMenu = true)}
		onmouseenter={() => (isMouseOverMenu = true)}
		onmouseleave={() => { isMouseOverMenu = false; visible = false; }}
		role="toolbar"
		aria-label="Table controls"
		tabindex="-1"
	>
		<button class="tb-btn" onclick={() => editor?.chain().focus().addRowBefore().run()} title="Add row above" aria-label="Add row above">
			<Icon name="chevronUp" size={15} />
			<span class="tb-plus"><Icon name="plus" size={9} /></span>
		</button>
		<button class="tb-btn" onclick={() => editor?.chain().focus().addRowAfter().run()} title="Add row below" aria-label="Add row below">
			<Icon name="chevronDown" size={15} />
			<span class="tb-plus"><Icon name="plus" size={9} /></span>
		</button>
		<button class="tb-btn" onclick={() => editor?.chain().focus().addColumnBefore().run()} title="Add column left" aria-label="Add column left">
			<Icon name="chevronLeft" size={15} />
			<span class="tb-plus"><Icon name="plus" size={9} /></span>
		</button>
		<button class="tb-btn" onclick={() => editor?.chain().focus().addColumnAfter().run()} title="Add column right" aria-label="Add column right">
			<Icon name="chevronRight" size={15} />
			<span class="tb-plus"><Icon name="plus" size={9} /></span>
		</button>
		<div class="tb-divider"></div>
		<button class="tb-btn" onclick={() => editor?.chain().focus().deleteRow().run()} title="Delete row" aria-label="Delete row">
			<Icon name="trash" size={15} />
			<span class="tb-mini">row</span>
		</button>
		<button class="tb-btn" onclick={() => editor?.chain().focus().deleteColumn().run()} title="Delete column" aria-label="Delete column">
			<Icon name="trash" size={15} />
			<span class="tb-mini">col</span>
		</button>
		<div class="tb-divider"></div>
		<button class="tb-btn" onclick={() => editor?.chain().focus().deleteTable().run()} title="Delete table" aria-label="Delete table">
			<Icon name="table" size={15} />
		</button>
	</div>
{/if}

<style>
	.tb-menu {
		position: fixed;
		z-index: 300;
		display: flex;
		align-items: center;
		gap: 2px;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 4px;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
	}

	.tb-btn {
		position: relative;
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: 5px;
		background: none;
		color: var(--color-text);
		cursor: pointer;
		transition: background-color 0.1s;
	}

	.tb-btn:hover {
		background-color: rgba(255, 255, 255, 0.06);
	}

	/* Tiny + badge in the corner of the direction buttons. */
	.tb-plus {
		position: absolute;
		right: 2px;
		top: 2px;
		color: var(--color-accent);
	}

	.tb-mini {
		position: absolute;
		right: 1px;
		bottom: 1px;
		font-size: 6px;
		font-weight: 700;
		text-transform: uppercase;
		color: var(--color-text-faint);
	}

	.tb-divider {
		width: 1px;
		height: 20px;
		background: var(--color-border);
		margin: 0 4px;
	}
</style>
