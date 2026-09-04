<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { MentionPluginKey } from '../extensions/mention.js';

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
		const pluginState = MentionPluginKey.getState(editor.state);
		if (pluginState) {
			editor
				.chain()
				.focus()
				.deleteRange({ from: pluginState.range.from, to: pluginState.range.to })
				.insertContent({ type: 'mention', attrs: { docId: page.id, title: page.title } })
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
		const ed = editor;
		if (!ed) return;
		const checkState = () => {
			const state = MentionPluginKey.getState(ed.state);
			if (state) {
				query = state.query;
				selectedIndex = 0;
				visible = true;
				updatePosition();
			} else {
				visible = false;
			}
		};
		ed.on('transaction', checkState);
		return () => { ed.off('transaction', checkState); };
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
	<div class="mention-menu" style="left: {position.x}px; top: {position.y}px;">
		<div class="mention-menu-header">Mention page</div>
		{#each filtered as page, i}
			<button class="mention-item" class:selected={i === selectedIndex} onclick={() => selectPage(page)}>
				<span class="mention-item-icon">@</span>
				<span class="mention-item-label">{page.title}</span>
			</button>
		{/each}
		{#if filtered.length === 0}
			<div class="mention-menu-empty">No pages match "{query}"</div>
		{/if}
	</div>
{/if}

<style>
	.mention-menu {
		position: absolute; z-index: 100;
		background: var(--color-surface); border: 1px solid var(--color-border);
		border-radius: 10px; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 6px; width: 280px; max-height: 240px; overflow-y: auto;
	}
	.mention-menu-header {
		font-size: 11px; font-weight: 600; text-transform: uppercase;
		letter-spacing: 0.05em; color: var(--color-text-muted); padding: 6px 10px 4px;
	}
	.mention-item {
		display: flex; align-items: center; gap: 10px;
		width: 100%; padding: 8px 10px; border: none; border-radius: 6px;
		background: none; color: var(--color-text); cursor: pointer;
		font-size: 14px; text-align: left; font-family: inherit;
		transition: background-color 0.1s;
	}
	.mention-item:hover, .mention-item.selected { background-color: color-mix(in srgb, var(--color-accent) 14%, transparent); }
	.mention-item-icon {
		display: flex; align-items: center; justify-content: center;
		width: 20px; height: 20px; border-radius: 50%;
		background: var(--color-accent-subtle); color: var(--color-accent);
		font-size: 13px; font-weight: 600;
	}
	.mention-item-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mention-menu-empty { font-size: 12px; color: var(--color-text-muted); padding: 10px; text-align: center; }
</style>
