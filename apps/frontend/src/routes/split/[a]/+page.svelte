<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import DocPane from '$lib/DocPane.svelte';
	import { invoke } from '$lib/backend.js';
	import { Icon } from '@enclave/ui';

	const a = $derived($page.params.a);
	const b = $derived($page.url.searchParams.get('b') ?? '');

	let pickerOpen = $state(false);
	let pages = $state<{ id: string; title: string }[]>([]);
	let query = $state('');

	const filtered = $derived(
		query
			? pages.filter((p) => (p.title || '').toLowerCase().includes(query.toLowerCase()))
			: pages
	);

	async function openPicker() {
		try {
			pages = await invoke<{ id: string; title: string }[]>('get_page_list');
		} catch (e) {
			console.error('Failed to load page list:', e);
		}
		query = '';
		pickerOpen = true;
	}

	function pick(p: { id: string; title: string }) {
		pickerOpen = false;
		goto(`/split/${a}?b=${p.id}`);
	}

	function closePaneB() {
		goto(`/split/${a}`);
	}

	// ── Draggable divider: the user owns the split ratio ──
	let splitPanesEl: HTMLDivElement | undefined = $state();
	let ratio = $state(0.5);
	let dragging = $state(false);

	$effect(() => {
		try {
			const saved = Number(localStorage.getItem('enclave-split-ratio'));
			if (saved >= 0.25 && saved <= 0.75) ratio = saved;
		} catch { /* ignore */ }
	});

	function onDividerDown(e: PointerEvent) {
		const el = splitPanesEl;
		if (!el) return;
		e.preventDefault();
		dragging = true;
		const rect = el.getBoundingClientRect();
		const move = (ev: PointerEvent) => {
			const x = Math.min(Math.max(ev.clientX - rect.left, 80), rect.width - 80);
			ratio = x / rect.width;
		};
		const up = () => {
			dragging = false;
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', up);
			try { localStorage.setItem('enclave-split-ratio', String(ratio)); } catch { /* ignore */ }
		};
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', up);
	}
</script>

<div class="split-page">
	<header class="split-header">
		<a href="/{a}" class="split-back" title="Back to single view">
			<Icon name="arrowLeft" size={15} />
			<span>Single view</span>
		</a>
		<span class="split-title">Split view</span>
		{#if !b}
			<button class="split-add" onclick={openPicker}>+ Add pane</button>
		{/if}
	</header>

	<div class="split-panes" bind:this={splitPanesEl} class:dragging={dragging}>
		<section class="split-pane" aria-label="Pane A" style="flex: {b ? ratio : 1} 1 0%;">
			<div class="pane-bar">
				<span class="pane-label">A</span>
				{#if b}
					<a href="/{a}" class="pane-link" title="Open pane A in a full tab">
						<Icon name="externalLink" size={12} />
					</a>
				{/if}
			</div>
			<div class="pane-scroll">
				<DocPane docId={a ?? ''} />
			</div>
		</section>

		{#if b}
			<div
				class="split-divider"
				class:active={dragging}
				onpointerdown={onDividerDown}
				role="separator"
				aria-orientation="vertical"
				aria-label="Resize panes"
				title="Drag to resize"
			></div>
			<section class="split-pane" aria-label="Pane B" style="flex: {1 - ratio} 1 0%;">
				<div class="pane-bar">
					<span class="pane-label">B</span>
					<div class="pane-bar-actions">
						<a href="/{b}" class="pane-link" title="Open pane B in a full tab">
							<Icon name="externalLink" size={12} />
						</a>
						<button class="pane-close" onclick={closePaneB} title="Close pane B">
							<Icon name="x" size={13} />
						</button>
					</div>
				</div>
				<div class="pane-scroll">
					<DocPane docId={b} />
				</div>
			</section>
		{:else}
			<button class="split-placeholder" onclick={openPicker}>
				<Icon name="plus" size={20} />
				<span>Add a page</span>
			</button>
		{/if}
	</div>

	{#if pickerOpen}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="picker-backdrop" onclick={() => (pickerOpen = false)}></div>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="picker" role="dialog" aria-label="Add a page to the split view">
			<!-- svelte-ignore a11y_autofocus -->
			<input class="picker-input" bind:value={query} placeholder="Search pages…" autofocus />
			<div class="picker-list">
				{#each filtered as p (p.id)}
					<button class="picker-item" onclick={() => pick(p)}>
						<span class="picker-icon"><Icon name="page" size={14} /></span>
						<span class="picker-label">{p.title || 'Untitled'}</span>
					</button>
				{/each}
				{#if filtered.length === 0}
					<div class="picker-empty">No pages found</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.split-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		padding: 0 12px 12px;
	}

	.split-header {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 4px 6px;
		flex-shrink: 0;
	}
	.split-back {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--color-text-muted);
		text-decoration: none;
		font-size: 13px;
		padding: 6px 10px;
		border-radius: var(--radius-md);
	}
	.split-back:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}
	.split-title {
		flex: 1;
		font-size: 13px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
	}
	.split-add {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-surface);
		color: var(--color-accent);
		font-size: 13px;
		font-weight: 500;
		font-family: inherit;
		cursor: pointer;
		padding: 6px 12px;
		transition: border-color 0.1s, background 0.1s;
	}
	.split-add:hover {
		border-color: var(--color-accent);
		background: var(--color-accent-subtle);
	}

	.split-panes {
		flex: 1;
		min-height: 0;
		display: flex;
	}

	.split-pane {
		min-width: 0;
		display: flex;
		flex-direction: column;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-surface);
		overflow: hidden;
	}

	/* Draggable divider — the user owns the split ratio. */
	.split-divider {
		flex: 0 0 6px;
		cursor: col-resize;
		touch-action: none;
		position: relative;
		z-index: 2;
	}
	.split-divider::after {
		content: '';
		position: absolute;
		left: 2px;
		top: 0;
		bottom: 0;
		width: 2px;
		border-radius: 2px;
		background: transparent;
		transition: background 0.12s;
	}
	.split-divider:hover::after,
	.split-divider.active::after {
		background: var(--color-accent);
	}
	.split-panes.dragging {
		cursor: col-resize;
		user-select: none;
	}

	.pane-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 3px 10px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface-hover);
		flex-shrink: 0;
	}
	.pane-label {
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
	}
	.pane-bar-actions {
		display: flex;
		align-items: center;
		gap: 2px;
	}
	.pane-link {
		display: flex;
		border: none;
		background: none;
		color: var(--color-text-faint);
		cursor: pointer;
		padding: 4px;
		border-radius: var(--radius-sm);
	}
	.pane-link:hover {
		background: var(--color-surface-active);
		color: var(--color-text);
	}
	.pane-close {
		border: none;
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 4px;
		border-radius: var(--radius-sm);
		display: flex;
	}
	.pane-close:hover {
		background: var(--color-surface-active);
		color: var(--color-text);
	}

	.pane-scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}

	.split-placeholder {
		flex: 1 1 0%;
		min-width: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		border: 1px dashed var(--color-border-strong);
		border-radius: var(--radius-lg);
		background: none;
		color: var(--color-text-faint);
		font-size: 14px;
		font-family: inherit;
		cursor: pointer;
		transition: border-color 0.15s, color 0.15s;
	}
	.split-placeholder:hover {
		border-color: var(--color-accent);
		color: var(--color-accent);
	}

	/* ── Page picker ── */
	.picker-backdrop {
		position: fixed;
		inset: 0;
		z-index: 200;
		background: var(--color-overlay);
	}
	.picker {
		position: fixed;
		z-index: 201;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 420px;
		max-width: 92vw;
		max-height: 60vh;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.picker-input {
		border: none;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface-hover);
		color: var(--color-text);
		font-size: 15px;
		font-family: inherit;
		padding: 14px 16px;
		outline: none;
	}
	.picker-list {
		overflow-y: auto;
		padding: 6px;
	}
	.picker-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 14px;
		font-family: inherit;
		text-align: left;
		padding: 9px 10px;
		border-radius: var(--radius-md);
		cursor: pointer;
	}
	.picker-item:hover {
		background: var(--color-surface-hover);
	}
	.picker-icon {
		display: flex;
		color: var(--color-text-faint);
	}
	.picker-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.picker-empty {
		padding: 16px;
		text-align: center;
		color: var(--color-text-faint);
		font-size: 13px;
	}

	/* Narrow windows: stack the panes instead of squeezing them. */
	@media (max-width: 900px) {
		.split-panes {
			flex-direction: column;
			overflow-y: auto;
			gap: 8px;
		}
		.split-pane {
			flex: none !important;
			min-height: 55vh;
		}
		.split-divider {
			display: none;
		}
		.split-placeholder {
			flex: none;
			min-height: 160px;
		}
	}
</style>
