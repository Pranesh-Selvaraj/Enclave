<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	let {
		docId,
		title,
		onPick,
	}: {
		docId: string;
		title: string;
		onPick: (docId: string, title: string) => void;
	} = $props();

	let pages = $state<{ id: string; title: string }[]>([]);
	let query = $state('');
	let pickerOpen = $state(false);

	let filtered = $derived(
		query
			? pages.filter((p) => p.title.toLowerCase().includes(query.toLowerCase()))
			: pages
	);

	async function openPicker() {
		if (pages.length === 0) {
			try {
				pages = await invoke<{ id: string; title: string }[]>('get_page_list');
			} catch (e) {
				console.error('Failed to load pages:', e);
			}
		}
		pickerOpen = true;
		query = '';
	}
</script>

{#if docId}
	<a class="pe-card" href="/{docId}" onclick={(e: MouseEvent) => { if (e.metaKey || e.ctrlKey) return; }} title={`Open ${title || 'page'}`}>
		<span class="pe-icon">📄</span>
		<span class="pe-title">{title || 'Untitled'}</span>
		<span class="pe-arrow">↗</span>
	</a>
{:else}
	<div class="pe-picker-wrap">
		<button class="pe-pick-btn" onclick={openPicker}>Embed a page…</button>
		{#if pickerOpen}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="pe-backdrop" onclick={() => (pickerOpen = false)}></div>
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="pe-picker" role="listbox" aria-label="Pick a page" tabindex="-1" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					class="pe-search"
					placeholder="Search pages…"
					bind:value={query}
					autofocus
				/>
				{#each filtered as p (p.id)}
					<button class="pe-item" role="option" aria-selected={false} onclick={() => { onPick(p.id, p.title || 'Untitled'); pickerOpen = false; }}>
						<span class="pe-item-icon">📄</span>
						<span>{p.title || 'Untitled'}</span>
					</button>
				{/each}
				{#if filtered.length === 0}
					<div class="pe-empty">No pages found</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.pe-card {
		display: flex;
		align-items: center;
		gap: 8px;
		border: 1px solid var(--color-border);
		border-radius: 8px;
		background: var(--color-hover);
		color: var(--color-text);
		padding: 8px 12px;
		text-decoration: none;
		font-size: 14px;
		margin: 8px 0;
		width: fit-content;
		min-width: 220px;
	}

	.pe-card:hover {
		border-color: rgba(124, 111, 240, 0.5);
	}

	.pe-icon {
		font-size: 14px;
	}

	.pe-title {
		font-weight: 500;
	}

	.pe-arrow {
		margin-left: auto;
		color: var(--color-text-muted);
		font-size: 12px;
	}

	.pe-picker-wrap {
		position: relative;
		margin: 8px 0;
	}

	.pe-pick-btn {
		border: 1px dashed var(--color-border);
		border-radius: 8px;
		background: none;
		color: var(--color-text-muted);
		padding: 8px 12px;
		cursor: pointer;
		font-size: 13px;
		width: 100%;
		text-align: left;
	}

	.pe-pick-btn:hover {
		border-color: rgba(124, 111, 240, 0.5);
		color: var(--color-text);
	}

	.pe-backdrop {
		position: fixed;
		inset: 0;
		z-index: 300;
	}

	.pe-picker {
		position: absolute;
		z-index: 301;
		top: calc(100% + 4px);
		left: 0;
		right: 0;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 6px;
		max-height: 260px;
		overflow-y: auto;
	}

	.pe-search {
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		padding: 6px 8px;
		outline: none;
		border-bottom: 1px solid var(--color-border);
		margin-bottom: 4px;
	}

	.pe-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		padding: 6px 8px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
		text-align: left;
	}

	.pe-item:hover {
		background: var(--color-hover);
	}

	.pe-item-icon {
		font-size: 13px;
	}

	.pe-empty {
		padding: 10px 8px;
		font-size: 12px;
		color: var(--color-text-muted);
	}
</style>
