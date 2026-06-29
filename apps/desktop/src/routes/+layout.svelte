<script lang="ts">
	import '../app.css';
	import { Sidebar } from '@enclave/ui';
	import { invoke } from '@tauri-apps/api/core';
	import type { Document } from '@enclave/ui';

	let { children } = $props();

	let documents = $state<Document[]>([]);
	let sidebarOpen = $state(true);
	let commandPaletteOpen = $state(false);
	let searchQuery = $state('');

	async function loadDocuments() {
		try {
			documents = await invoke<Document[]>('get_document_list');
		} catch (e) {
			console.error('Failed to load documents:', e);
		}
	}

	async function createDocument() {
		try {
			await invoke('create_document', { title: 'Untitled' });
			await loadDocuments();
		} catch (e) {
			console.error('Failed to create document:', e);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
			e.preventDefault();
			commandPaletteOpen = !commandPaletteOpen;
		}
		if (e.key === 'Escape') {
			commandPaletteOpen = false;
		}
	}

	$effect(() => {
		loadDocuments();
	});
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app-shell">
	<!-- Left Sidebar -->
	<aside class="sidebar" class:collapsed={!sidebarOpen}>
		<div class="sidebar-header">
			<div class="sidebar-brand">
				<span class="brand-icon">🔒</span>
				{#if sidebarOpen}
					<span class="brand-name">Enclave</span>
				{/if}
			</div>
			<button class="sidebar-toggle" onclick={() => (sidebarOpen = !sidebarOpen)}>
				{sidebarOpen ? '◀' : '▶'}
			</button>
		</div>

		{#if sidebarOpen}
			<div class="sidebar-section">
				<button class="new-page-btn" onclick={createDocument}>
					<span class="new-page-icon">+</span>
					New page
				</button>
			</div>

			<nav class="page-tree">
				<div class="tree-section-title">Pages</div>
				{#each documents as doc (doc.id)}
					<a href="/{doc.id}" class="tree-item">
						<span class="tree-item-icon">📄</span>
						<span class="tree-item-label">{doc.title || 'Untitled'}</span>
					</a>
				{/each}
				{#if documents.length === 0}
					<div class="tree-empty">No pages yet</div>
				{/if}
			</nav>

			<div class="sidebar-footer">
				<div class="sync-status offline">
					<span class="sync-dot"></span>
					<span>Offline</span>
				</div>
			</div>
		{/if}
	</aside>

	<!-- Main Content Area -->
	<div class="main-pane">
		{@render children?.()}
	</div>

	<!-- Command Palette Overlay -->
	{#if commandPaletteOpen}
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="overlay" onclick={() => (commandPaletteOpen = false)}>
			<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
			<div class="command-palette" onclick={(e) => e.stopPropagation()}>
				<input
					type="text"
					class="palette-input"
					placeholder="Search pages or type a command…"
					bind:value={searchQuery}
					autofocus
				/>
				<div class="palette-results">
					{#each documents.filter((d) => !searchQuery || d.title.toLowerCase().includes(searchQuery.toLowerCase())) as doc (doc.id)}
						<a href="/{doc.id}" class="palette-item" onclick={() => (commandPaletteOpen = false)}>
							<span class="palette-icon">📄</span>
							<span>{doc.title || 'Untitled'}</span>
						</a>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.app-shell {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}

	/* ── Sidebar ── */
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 260px;
		min-width: 260px;
		background-color: var(--color-surface);
		border-right: 1px solid var(--color-border);
		transition: width 0.2s, min-width 0.2s;
	}

	.sidebar.collapsed {
		width: 52px;
		min-width: 52px;
	}

	.sidebar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 12px;
		min-height: 48px;
	}

	.sidebar-brand {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.brand-icon { font-size: 18px; }

	.brand-name {
		font-size: 15px;
		font-weight: 700;
		white-space: nowrap;
	}

	.sidebar-toggle {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 11px;
		padding: 4px 6px;
		border-radius: var(--radius-sm);
	}

	.sidebar-toggle:hover {
		color: var(--color-text);
		background: var(--color-surface-hover);
	}

	.sidebar-section {
		padding: 4px 8px;
	}

	.new-page-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 6px 10px;
		border: none;
		border-radius: var(--radius-md);
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 14px;
		font-family: inherit;
		transition: background 0.15s, color 0.15s;
	}

	.new-page-btn:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	.new-page-icon {
		font-size: 18px;
		font-weight: 300;
		width: 24px;
		text-align: center;
	}

	/* ── Page Tree ── */
	.page-tree {
		flex: 1;
		padding: 8px;
		overflow-y: auto;
	}

	.tree-section-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		padding: 8px 8px 4px;
	}

	.tree-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 8px;
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		text-decoration: none;
		font-size: 14px;
		min-height: 30px;
		transition: background 0.1s, color 0.1s;
	}

	.tree-item:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	.tree-item-icon { font-size: 16px; flex-shrink: 0; }
	.tree-item-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.tree-empty { font-size: 12px; color: var(--color-text-muted); padding: 8px; }

	.sidebar-footer {
		padding: 10px 16px;
		border-top: 1px solid var(--color-border);
	}

	.sync-status {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--color-text-muted);
	}

	.sync-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #666;
	}

	/* ── Main Pane ── */
	.main-pane {
		flex: 1;
		overflow-y: auto;
		background: var(--color-bg);
	}

	/* ── Command Palette ── */
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 200;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		justify-content: center;
		padding-top: 15vh;
	}

	.command-palette {
		width: 560px;
		max-height: 400px;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.palette-input {
		width: 100%;
		padding: 14px 18px;
		border: none;
		border-bottom: 1px solid var(--color-border);
		background: none;
		color: var(--color-text);
		font-size: 16px;
		font-family: inherit;
		outline: none;
	}

	.palette-input::placeholder {
		color: var(--color-text-muted);
	}

	.palette-results {
		flex: 1;
		overflow-y: auto;
		padding: 6px;
	}

	.palette-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 12px;
		border-radius: var(--radius-md);
		color: var(--color-text);
		text-decoration: none;
		font-size: 14px;
		transition: background 0.1s;
	}

	.palette-item:hover {
		background: var(--color-surface-hover);
	}

	.palette-icon { font-size: 18px; }
</style>
