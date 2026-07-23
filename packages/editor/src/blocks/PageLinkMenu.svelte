<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { PageLinkPluginKey } from '../extensions/page-link.js';

	let {
		editor,
		allPages = [],
	}: {
		editor: Editor | undefined;
		allPages: { id: string; title: string }[];
	} = $props();

	let query = $state('');
	let selectedIndex = $state(0);
	let visible = $state(false);
	let position = $state({ x: 0, y: 0 });

	let filtered = $derived(
		query
			? allPages.filter((p) => p.title.toLowerCase().includes(query.toLowerCase()))
			: allPages
	);

	function selectPage(page: { id: string; title: string }) {
		if (!editor) return;
		const pluginState = PageLinkPluginKey.getState(editor.state);
		if (pluginState) {
			editor
				.chain()
				.focus()
				.deleteRange({ from: pluginState.range.from, to: pluginState.range.to })
				.insertContent(`[[${page.title}]]`)
				.run();
		}
		visible = false;
		query = '';
	}

	function updatePosition() {
		if (!editor) return;
		const { from } = editor.state.selection;
		const coords = editor.view.coordsAtPos(from);
		const editorEl = editor.view.dom.closest('.editor-container');
		const editorRect = editorEl?.getBoundingClientRect();
		position = {
			x: coords.left - (editorRect?.left ?? 0),
			y: coords.bottom - (editorRect?.top ?? 0) + 8,
		};
	}

	$effect(() => {
		if (!editor) return;
		const checkState = () => {
			const state = PageLinkPluginKey.getState(editor.state);
			if (state) {
				query = state.query;
				selectedIndex = 0;
				visible = true;
				updatePosition();
			} else {
				visible = false;
			}
		};
		editor.on('transaction', checkState);
		return () => { editor.off('transaction', checkState); };
	});

	function handleKeydown(e: KeyboardEvent) {
		if (!visible) return;
		if (e.key === 'ArrowDown') { e.preventDefault(); selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1); }
		else if (e.key === 'ArrowUp') { e.preventDefault(); selectedIndex = Math.max(selectedIndex - 1, 0); }
		else if (e.key === 'Enter') { e.preventDefault(); const p = filtered[selectedIndex]; if (p) selectPage(p); }
		else if (e.key === 'Escape') { visible = false; }
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if visible && editor}
	<div class="link-menu" style="left: {position.x}px; top: {position.y}px;">
		<div class="link-menu-header">Link to page</div>
		{#each filtered as page, i}
			<button class="link-item" class:selected={i === selectedIndex} onclick={() => selectPage(page)}>
				<span class="link-item-icon">📄</span>
				<span class="link-item-label">{page.title}</span>
			</button>
		{/each}
		{#if allPages.length === 0}
			<div class="link-menu-empty">No pages yet. Start typing to create a new link.</div>
		{/if}
	</div>
{/if}

<style>
	.link-menu {
		position: absolute; z-index: 100;
		background: var(--color-surface); border: 1px solid var(--color-border);
		border-radius: 10px; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 6px; width: 280px; max-height: 240px; overflow-y: auto;
	}
	.link-menu-header {
		font-size: 11px; font-weight: 600; text-transform: uppercase;
		letter-spacing: 0.05em; color: var(--color-text-muted); padding: 6px 10px 4px;
	}
	.link-item {
		display: flex; align-items: center; gap: 10px;
		width: 100%; padding: 8px 10px; border: none; border-radius: 6px;
		background: none; color: var(--color-text); cursor: pointer;
		font-size: 14px; text-align: left; font-family: inherit;
		transition: background-color 0.1s;
	}
	.link-item:hover, .link-item.selected { background-color: rgba(124, 111, 240, 0.12); }
	.link-item-icon { font-size: 16px; }
	.link-item-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.link-menu-empty { font-size: 12px; color: var(--color-text-muted); padding: 10px; text-align: center; }
</style>
