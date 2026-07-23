<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import type { Document } from '@enclave/ui';
	import { theme } from '@enclave/ui';
	import VaultGuard from '$lib/VaultGuard.svelte';
	import SettingsPanel from '$lib/SettingsPanel.svelte';

	let { children } = $props();

	let settingsOpen = $state(false);
	theme.init();

	let vaultUnlocked = $state(false);
	let documents = $state<Document[]>([]);
	let sidebarOpen = $state(true);
	let commandPaletteOpen = $state(false);
	let searchQuery = $state('');
	let debouncedQuery = $state('');
	let networkRunning = $state(false);
	let networkStatus = $state<{ local_peer_id: string; running: boolean; port: number; peers: any[] } | null>(null);
	const currentDocId = $derived($page.params?.id);
	let contextMenu = $state<{ doc: Document; x: number; y: number } | null>(null);

	let searchTimer: ReturnType<typeof setTimeout>;

	async function loadDocuments() {
		try {
			documents = await invoke<Document[]>('get_document_list');
		} catch (e) {
			console.error('Failed to load documents:', e);
		}
	}

	async function createDocument() {
		try {
			const doc = await invoke<Document>('create_document', { title: 'Untitled' });
			await loadDocuments();
			// Use both methods for navigation — goto for SPA, with fallback
			try {
				await goto(`/${doc.id}`);
			} catch {
				window.location.href = `/${doc.id}`;
			}
		} catch (e: any) {
			console.error('Failed to create document:', e);
			alert(`Failed to create page: ${e?.message || e}`);
		}
	}

	async function toggleFavorite(id: string) {
		try {
			await invoke('toggle_favorite', { id });
			await loadDocuments();
		} catch (e) {
			console.error('Failed to toggle favorite:', e);
		}
	}

	async function duplicateDocument(id: string) {
		try {
			await invoke('duplicate_document', { id });
			await loadDocuments();
		} catch (e) {
			console.error('Failed to duplicate document:', e);
		}
	}

	async function deleteDocument(id: string) {
		const doc = documents.find(d => d.id === id);
		if (!confirm(`Delete "${doc?.title || 'Untitled'}"? This cannot be undone.`)) return;
		try {
			await invoke('delete_document', { id });
			await loadDocuments();
		} catch (e) {
			console.error('Failed to delete document:', e);
		}
	}

	async function toggleNetwork() {
		try {
			if (networkRunning) {
				await invoke('stop_network');
				networkRunning = false;
				networkStatus = null;
			} else {
				await invoke('start_network');
				networkRunning = true;
				networkStatus = await invoke<typeof networkStatus>('network_status');
			}
		} catch (e) {
			console.error('Network toggle failed:', e);
			networkRunning = false;
		}
	}

	function showContextMenu(e: MouseEvent, doc: Document) {
		e.preventDefault();
		contextMenu = { doc, x: e.clientX, y: e.clientY };
	}

	function handleKeydown(e: KeyboardEvent) {
		const mod = e.ctrlKey || e.metaKey;
		if (mod && e.key === 'k') {
			e.preventDefault();
			commandPaletteOpen = !commandPaletteOpen;
		}
		if (mod && e.key === 'n') {
			e.preventDefault();
			createDocument();
		}
		if (mod && e.key === 'b') {
			e.preventDefault();
			sidebarOpen = !sidebarOpen;
		}
		if (e.key === 'Escape') {
			commandPaletteOpen = false;
		}
	}

	// Debounced search for command palette
	$effect(() => {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => { debouncedQuery = searchQuery; }, 150);
		return () => clearTimeout(searchTimer);
	});

	let favorites = $derived(documents.filter(d => d.is_favorite));
	let regularPages = $derived(documents.filter(d => !d.is_favorite));
	let filteredDocs = $derived(
		debouncedQuery
			? documents.filter(d => d.title.toLowerCase().includes(debouncedQuery.toLowerCase()))
			: documents
	);

	$effect(() => {
		if (vaultUnlocked) loadDocuments();
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if !vaultUnlocked}
	<VaultGuard onunlock={() => (vaultUnlocked = true)} />
{:else}
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
				{#if favorites.length > 0}
					<div class="tree-section-title">Favorites</div>
					{#each favorites as doc (doc.id)}
						<a href="/{doc.id}" class="tree-item" class:active={currentDocId === doc.id} oncontextmenu={(e: MouseEvent) => { e.preventDefault(); }}>
							<button class="fav-star" onclick={(e: MouseEvent) => { e.preventDefault(); e.stopPropagation(); toggleFavorite(doc.id); }} title="Unfavorite">
								★
							</button>
							<span class="tree-item-icon">📄</span>
							<span class="tree-item-label">{doc.title || 'Untitled'}</span>
						</a>
					{/each}
				{/if}

				<div class="tree-section-title">Pages <span class="page-count">{documents.length}</span></div>
				{#each regularPages as doc (doc.id)}
					<a href="/{doc.id}" class="tree-item" class:active={currentDocId === doc.id} oncontextmenu={(e: MouseEvent) => { e.preventDefault(); showContextMenu(e, doc); }}>
						<button class="fav-star empty" onclick={(e: MouseEvent) => { e.preventDefault(); e.stopPropagation(); toggleFavorite(doc.id); }} title="Add to favorites">
							☆
						</button>
						<span class="tree-item-icon">📄</span>
						<span class="tree-item-label">{doc.title || 'Untitled'}</span>
					</a>
				{/each}
				{#if documents.length === 0}
					<div class="tree-empty">No pages yet — press <kbd>Ctrl+N</kbd></div>
				{/if}
			</nav>

			<div class="sidebar-footer">
				<div class="sync-status" class:online={networkRunning} class:offline={!networkRunning}>
					<span class="sync-dot"></span>
					<span>{networkRunning ? `P2P:${networkStatus?.port ?? '?'}` : 'Offline'}</span>
				</div>
				<div class="footer-actions">
					<button class="icon-btn" onclick={toggleNetwork} title="Toggle P2P sync">
						{networkRunning ? '⏸' : '▶'}
					</button>
					<button class="icon-btn" onclick={() => theme.toggle()} title="Toggle theme">
						{theme.value === 'dark' ? '☀' : '🌙'}
					</button>
					<a href="/graph" class="icon-btn" title="Graph view">🔗</a>
					<button class="icon-btn" onclick={() => (settingsOpen = true)} title="Settings">
						⚙
					</button>
				</div>
				{#if networkStatus?.peers?.length}
					<div class="peer-list">
						{#each networkStatus.peers as peer}
							<div class="peer-item">
								<span class="peer-dot connected"></span>
								<span class="peer-label">{peer.id.slice(0, 8)}…</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</aside>

	<!-- Context Menu -->
	{#if contextMenu}
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="context-overlay" onclick={() => (contextMenu = null)} onkeydown={() => {}}>
			<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
			<div class="context-menu" style="left:{contextMenu.x}px;top:{contextMenu.y}px;" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<button class="context-item" onclick={() => { toggleFavorite(contextMenu!.doc.id); contextMenu = null; }}>
					{contextMenu.doc.is_favorite ? '★ Unfavorite' : '☆ Favorite'}
				</button>
				<button class="context-item" onclick={() => { duplicateDocument(contextMenu!.doc.id); contextMenu = null; }}>
					📋 Duplicate
				</button>
				<button class="context-item danger" onclick={() => { deleteDocument(contextMenu!.doc.id); contextMenu = null; }}>
					🗑 Delete
				</button>
			</div>
		</div>
	{/if}

	<!-- Main Content Area -->
	<div class="main-pane">
		{@render children?.()}
	</div>

	<!-- Command Palette Overlay -->
	{#if commandPaletteOpen}
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="overlay" role="dialog" aria-modal="true" aria-label="Command palette" onclick={() => (commandPaletteOpen = false)} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') commandPaletteOpen = false; }}>
			<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
			<div class="command-palette" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<input
					type="text"
					class="palette-input"
					placeholder="Search pages or type a command…"
					bind:value={searchQuery}
					autofocus
				/>
				<div class="palette-results">
					{#each filteredDocs as doc (doc.id)}
						<a href="/{doc.id}" class="palette-item" onclick={() => (commandPaletteOpen = false)}>
							<span class="palette-icon">{doc.is_favorite ? '★' : '📄'}</span>
							<span>{doc.title || 'Untitled'}</span>
						</a>
					{/each}
					{#if filteredDocs.length === 0 && searchQuery}
						<div class="palette-empty">No pages found</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
</div>
{/if}

<SettingsPanel bind:open={settingsOpen} onlock={() => (vaultUnlocked = false)} />

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
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.page-count {
		font-size: 10px;
		background: var(--color-surface-hover);
		padding: 1px 6px;
		border-radius: 10px;
	}

	.tree-item {
		display: flex;
		align-items: center;
		gap: 4px;
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

	.tree-item.active {
		background: var(--color-accent-subtle);
		color: var(--color-text);
	}

	.tree-item-icon { font-size: 16px; flex-shrink: 0; }
	.tree-item-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
	.tree-empty { font-size: 12px; color: var(--color-text-muted); padding: 8px; }
	.tree-empty kbd {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 3px;
		padding: 1px 4px;
		font-size: 11px;
		font-family: var(--font-mono);
	}

	.fav-star {
		background: none;
		border: none;
		cursor: pointer;
		font-size: 12px;
		padding: 0 2px;
		color: var(--color-warning);
		line-height: 1;
		flex-shrink: 0;
	}
	.fav-star.empty { color: var(--color-text-muted); }
	.fav-star.empty:hover { color: var(--color-warning); }

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

	.sync-status.online .sync-dot { background: var(--color-success); }
	.sync-status.online { color: var(--color-success); }

	.footer-actions {
		display: flex;
		gap: 4px;
		margin-top: 8px;
	}

	.icon-btn {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 14px;
		padding: 3px 8px;
		line-height: 1;
		text-decoration: none;
		transition: background 0.15s, color 0.15s;
	}

	.icon-btn:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	/* ── Context Menu ── */
	.context-overlay {
		position: fixed;
		inset: 0;
		z-index: 250;
	}

	.context-menu {
		position: fixed;
		z-index: 251;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
		padding: 4px;
		min-width: 160px;
	}

	.context-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 8px 12px;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--color-text);
		cursor: pointer;
		font-size: 13px;
		font-family: inherit;
		text-align: left;
		transition: background 0.1s;
	}
	.context-item:hover { background: var(--color-surface-hover); }
	.context-item.danger { color: var(--color-danger); }
	.context-item.danger:hover { background: rgba(224, 62, 62, 0.1); }

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
	.palette-empty {
		padding: 16px;
		text-align: center;
		color: var(--color-text-muted);
		font-size: 13px;
	}
</style>
