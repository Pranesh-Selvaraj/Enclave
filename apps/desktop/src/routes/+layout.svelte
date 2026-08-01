<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import type { Document } from '@enclave/ui';
	import { theme } from '@enclave/ui';
	import { ShortcutsDialog } from '@enclave/ui';
	import Icon from '$lib/Icon.svelte';
	import VaultGuard from '$lib/VaultGuard.svelte';
	import SettingsPanel from '$lib/SettingsPanel.svelte';
	import { importMarkdownFiles, exportVaultAsMarkdown } from '$lib/importExport.js';

	let { children } = $props();

	let settingsOpen = $state(false);
	let shortcutsOpen = $state(false);
	theme.init();

	let vaultUnlocked = $state(false);
	let documents = $state<Document[]>([]);
	let archivedDocs = $state<Document[]>([]);
	let sidebarOpen = $state(true);
	let commandPaletteOpen = $state(false);
	let searchQuery = $state('');
	let debouncedQuery = $state('');
	let networkRunning = $state(false);
	let networkStatus = $state<{ local_peer_id: string; running: boolean; port: number; peers: any[] } | null>(null);
	const currentDocId = $derived($page.params?.id);
	const currentPath = $derived($page.url.pathname);
	let contextMenu = $state<{ doc: Document; x: number; y: number } | null>(null);

	let searchTimer: ReturnType<typeof setTimeout>;
	let searchResults = $state<{ doc_id: string; doc_title: string; snippet: string }[] | null>(null);
	let tagsByDoc = $state<Map<string, string[]>>(new Map());
	let selectedTag = $state<string | null>(null);

	async function loadTags() {
		try {
			const rows = await invoke<{ doc_id: string; tags: string[] }[]>('get_all_tags');
			tagsByDoc = new Map(rows.map(r => [r.doc_id, r.tags]));
		} catch (e) {
			console.error('Failed to load tags:', e);
		}
	}

	let allTags = $derived.by(() => {
		const counts = new Map<string, number>();
		for (const tags of tagsByDoc.values()) {
			for (const t of tags) counts.set(t, (counts.get(t) ?? 0) + 1);
		}
		return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([name, count]) => ({ name, count }));
	});

	async function loadDocuments() {
		try {
			documents = await invoke<Document[]>('get_document_list');
		} catch (e) {
			console.error('Failed to load documents:', e);
		}
	}

	async function loadArchived() {
		try {
			archivedDocs = await invoke<Document[]>('get_archived_documents');
		} catch (e) {
			console.error('Failed to load trash:', e);
		}
	}

	async function restoreDocument(id: string) {
		try {
			await invoke('restore_document', { id });
			await loadDocuments();
			await loadArchived();
			loadTags();
		} catch (e) {
			console.error('Failed to restore document:', e);
		}
	}

	async function deleteDocumentPermanently(id: string) {
		const doc = archivedDocs.find(d => d.id === id);
		if (!confirm(`Permanently delete "${doc?.title || 'Untitled'}"? This cannot be undone.`)) return;
		try {
			await invoke('delete_document', { id });
			await loadArchived();
		} catch (e) {
			console.error('Failed to delete document:', e);
		}
	}

	async function createDocument() {
		try {
			const doc = await invoke<Document>('create_document', { title: 'Untitled' });
			await loadDocuments();
			loadTags();
			await goto(`/${doc.id}`);
		} catch (e: any) {
			console.error('Failed to create document:', e);
			alert(`Failed to create page: ${e?.message || e}`);
		}
	}

	async function toggleFavorite(id: string) {
		try {
			await invoke('toggle_favorite', { id });
			await loadDocuments();
			loadTags();
		} catch (e) {
			console.error('Failed to toggle favorite:', e);
		}
	}

	async function duplicateDocument(id: string) {
		try {
			await invoke('duplicate_document', { id });
			await loadDocuments();
			loadTags();
		} catch (e) {
			console.error('Failed to duplicate document:', e);
		}
	}

	async function deleteDocument(id: string) {
		const doc = documents.find(d => d.id === id);
		if (!confirm(`Move "${doc?.title || 'Untitled'}" to trash?`)) return;
		try {
			await invoke('archive_document', { id });
			await loadDocuments();
			await loadArchived();
			loadTags();
		} catch (e) {
			console.error('Failed to archive document:', e);
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
		if (e.key === '?' && !mod) {
			const tag = (e.target as HTMLElement)?.tagName;
			if (tag !== 'INPUT' && tag !== 'TEXTAREA' && !(e.target as HTMLElement)?.isContentEditable) {
				e.preventDefault();
				shortcutsOpen = !shortcutsOpen;
			}
		}
		if (e.key === 'Escape') {
			commandPaletteOpen = false;
			contextMenu = null;
			shortcutsOpen = false;
		}
	}

	// Debounced search for command palette
	$effect(() => {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => { debouncedQuery = searchQuery; }, 150);
		return () => clearTimeout(searchTimer);
	});

	// Full-text search (titles + block content) via the backend
	$effect(() => {
		const q = debouncedQuery.trim();
		if (!q) {
			searchResults = null;
			return;
		}
		invoke<{ doc_id: string; doc_title: string; block_content: string; type: string }[]>('search_all', { query: q })
			.then((rows) => {
				if (q !== debouncedQuery.trim()) return; // superseded by a newer keystroke
				// FTS ranks rows interleaved — title rows always win, content
				// rows only fill in a snippet when no snippet exists yet.
				const byId = new Map<string, { doc_id: string; doc_title: string; snippet: string }>();
				for (const r of rows) {
					const snippet = r.type === 'title' ? '' : r.block_content.replace(/\s+/g, ' ').slice(0, 140);
					const existing = byId.get(r.doc_id);
					if (r.type === 'title' || !existing || existing.snippet === '') {
						byId.set(r.doc_id, {
							doc_id: r.doc_id,
							doc_title: r.doc_title,
							snippet: r.type === 'title' && existing ? existing.snippet : snippet,
						});
					}
				}
				searchResults = [...byId.values()].slice(0, 12);
			})
			.catch(() => { if (q === debouncedQuery.trim()) searchResults = null; });
	});

	let filteredDocs = $derived.by(() => {
		let out = documents;
		if (selectedTag) {
			out = out.filter(d => tagsByDoc.get(d.id)?.includes(selectedTag!));
		}
		if (debouncedQuery) {
			out = out.filter(d => d.title.toLowerCase().includes(debouncedQuery.toLowerCase()));
		}
		return out;
	});

	let favorites = $derived(filteredDocs.filter(d => d.is_favorite));
	let regularPages = $derived(filteredDocs.filter(d => !d.is_favorite));

	$effect(() => {
		if (vaultUnlocked) {
			loadDocuments();
			loadArchived();
			loadTags();
		}
	});

	// Refresh tag counts after navigating (tags are edited on the page view)
	$effect(() => {
		const _id = $page.params.id;
		if (vaultUnlocked) loadTags();
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if currentPath.startsWith('/capture')}
	{@render children?.()}
{:else if !vaultUnlocked}
	<VaultGuard onunlock={() => (vaultUnlocked = true)} />
{:else}
<div class="app-shell">
	<!-- Left Sidebar -->
	<aside class="sidebar" class:collapsed={!sidebarOpen}>
		<div class="sidebar-header">
			<a href="/" class="sidebar-brand" title="Enclave home">
				<span class="brand-mark">
					<Icon name="lock" size={14} />
				</span>
				{#if sidebarOpen}
					<span class="brand-name">Enclave</span>
				{/if}
			</a>
			<button class="sidebar-toggle" onclick={() => (sidebarOpen = !sidebarOpen)} title="Toggle sidebar (Ctrl+B)">
				<Icon name={sidebarOpen ? 'chevronLeft' : 'chevronRight'} size={14} />
			</button>
		</div>

		{#if sidebarOpen}
			<nav class="side-nav">
				<a href="/" class="nav-item" class:active={currentPath === '/'}>
					<Icon name="home" size={16} />
					<span>Home</span>
				</a>
				<a href="/graph" class="nav-item" class:active={currentPath === '/graph'}>
					<Icon name="graph" size={16} />
					<span>Graph view</span>
				</a>
			</nav>

			<div class="pages-section">
				<div class="section-head">
					<span class="section-title">Pages</span>
					<span class="page-count">{filteredDocs.length}</span>
				</div>

				<div class="page-tree">
					{#if favorites.length > 0}
						<div class="tree-section-title">Favorites</div>
						{#each favorites as doc (doc.id)}
							<a href="/{doc.id}" class="tree-item" class:active={currentDocId === doc.id}
								oncontextmenu={(e: MouseEvent) => showContextMenu(e, doc)}>
								<span class="tree-item-icon">
									<Icon name="star" size={14} />
								</span>
								<span class="tree-item-label">{doc.title || 'Untitled'}</span>
								<span class="tree-item-actions">
									<button class="row-btn" onclick={(e: MouseEvent) => { e.preventDefault(); e.stopPropagation(); toggleFavorite(doc.id); }} title="Unfavorite">
										<Icon name="star" size={13} />
									</button>
									<button class="row-btn" onclick={(e: MouseEvent) => { e.preventDefault(); e.stopPropagation(); showContextMenu(e, doc); }} title="More">
										<Icon name="more" size={13} />
									</button>
								</span>
							</a>
						{/each}
					{/if}

					{#each regularPages as doc (doc.id)}
						<a href="/{doc.id}" class="tree-item" class:active={currentDocId === doc.id}
							oncontextmenu={(e: MouseEvent) => showContextMenu(e, doc)}>
							<span class="tree-item-icon">
								<Icon name="page" size={14} />
							</span>
							<span class="tree-item-label">{doc.title || 'Untitled'}</span>
							<span class="tree-item-actions">
								<button class="row-btn" onclick={(e: MouseEvent) => { e.preventDefault(); e.stopPropagation(); toggleFavorite(doc.id); }} title="Add to favorites">
									<Icon name="star" size={13} />
								</button>
								<button class="row-btn" onclick={(e: MouseEvent) => { e.preventDefault(); e.stopPropagation(); showContextMenu(e, doc); }} title="More">
									<Icon name="more" size={13} />
								</button>
							</span>
						</a>
					{/each}

					{#if documents.length === 0}
						<div class="tree-empty">
							No pages yet — press <kbd>Ctrl+N</kbd> or
							<button class="link-btn" onclick={createDocument}>create one</button>
						</div>
					{/if}
				</div>
			</div>

			{#if allTags.length > 0}
				<div class="pages-section tags-section">
					<div class="section-head">
						<span class="section-title">Tags</span>
						{#if selectedTag}
							<button class="link-btn" onclick={() => (selectedTag = null)}>Clear</button>
						{/if}
					</div>
					<div class="tag-list">
						{#each allTags as { name, count } (name)}
							<button
								class="tag-row"
								class:active={selectedTag === name}
								onclick={() => (selectedTag = selectedTag === name ? null : name)}
								title={`Show pages tagged #${name}`}
							>
								<span class="tag-row-hash">#</span>
								<span class="tag-row-label">{name}</span>
								<span class="tag-row-count">{count}</span>
							</button>
						{/each}
					</div>
				</div>
			{/if}

			{#if archivedDocs.length > 0}
				<div class="pages-section trash-section">
					<div class="section-head">
						<span class="section-title">Trash</span>
					</div>
					<div class="page-tree">
						{#each archivedDocs as doc (doc.id)}
							<div class="tree-item" title={doc.title || 'Untitled'}>
								<span class="tree-item-icon">
									<Icon name="trash" size={14} />
								</span>
								<span class="tree-item-label">{doc.title || 'Untitled'}</span>
								<span class="tree-item-actions">
									<button class="row-btn" onclick={() => restoreDocument(doc.id)} title="Restore">
										<Icon name="upload" size={13} />
									</button>
									<button class="row-btn danger-row" onclick={() => deleteDocumentPermanently(doc.id)} title="Delete permanently">
										<Icon name="trash" size={13} />
									</button>
								</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<div class="sidebar-footer">
				<button class="new-page-btn" onclick={createDocument} title="New page (Ctrl+N)">
					<Icon name="plus" size={15} />
					<span>New page</span>
				</button>
				<div class="footer-row">
					<div class="sync-status" class:online={networkRunning} title="P2P sync">
						<span class="sync-dot"></span>
						<span>{networkRunning ? `P2P:${networkStatus?.port ?? '?'}` : 'Offline'}</span>
					</div>
					<div class="footer-actions">
						<button class="icon-btn" onclick={toggleNetwork} title="Toggle P2P sync">
							<Icon name="network" size={15} />
						</button>
						<button class="icon-btn" onclick={() => theme.toggle()} title="Toggle theme">
							<Icon name={theme.value === 'dark' ? 'sun' : 'moon'} size={15} />
						</button>
						<button class="icon-btn" onclick={() => (settingsOpen = true)} title="Settings">
							<Icon name="gear" size={15} />
						</button>
					</div>
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
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="context-overlay" onclick={() => (contextMenu = null)}>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="context-menu" style="left:{contextMenu.x}px;top:{contextMenu.y}px;" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<button class="context-item" onclick={() => { toggleFavorite(contextMenu!.doc.id); contextMenu = null; }}>
					<Icon name={contextMenu.doc.is_favorite ? 'star' : 'star'} size={14} />
					{contextMenu.doc.is_favorite ? 'Unfavorite' : 'Add to favorites'}
				</button>
				<button class="context-item" onclick={() => { duplicateDocument(contextMenu!.doc.id); contextMenu = null; }}>
					<Icon name="duplicate" size={14} />
					Duplicate
				</button>
				<div class="context-sep"></div>
				<button class="context-item danger" onclick={() => { deleteDocument(contextMenu!.doc.id); contextMenu = null; }}>
					<Icon name="trash" size={14} />
					Delete
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
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="overlay" role="dialog" aria-modal="true" aria-label="Command palette" tabindex="-1" onclick={() => (commandPaletteOpen = false)} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') commandPaletteOpen = false; }}>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="command-palette" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<div class="palette-input-wrap">
					<Icon name="search" size={16} />
					<!-- svelte-ignore a11y_autofocus -->
					<input
						type="text"
						class="palette-input"
						placeholder="Search pages…"
						bind:value={searchQuery}
						autofocus
					/>
					<kbd class="palette-kbd">esc</kbd>
				</div>
				<div class="palette-results">
					<div class="palette-group-title">{searchQuery.trim() ? 'Results' : 'Recent'}</div>
					{#if searchQuery.trim() && searchResults}
						{#each searchResults as r (r.doc_id)}
							<a href="/{r.doc_id}" class="palette-item" onclick={() => (commandPaletteOpen = false)}>
								<span class="palette-icon">
									<Icon name="search" size={15} />
								</span>
								<span class="palette-item-text">
									<span class="palette-item-title">{r.doc_title || 'Untitled'}</span>
									{#if r.snippet}
										<span class="palette-item-snippet">{r.snippet}</span>
									{/if}
								</span>
							</a>
						{/each}
						{#if searchResults.length === 0}
							<div class="palette-empty">No results found</div>
						{/if}
					{:else if !searchQuery.trim()}
						{#each documents as doc (doc.id)}
							<a href="/{doc.id}" class="palette-item" onclick={() => (commandPaletteOpen = false)}>
								<span class="palette-icon">
									<Icon name={doc.is_favorite ? 'star' : 'page'} size={15} />
								</span>
								<span>{doc.title || 'Untitled'}</span>
							</a>
						{/each}
					{:else}
						<div class="palette-empty">Searching…</div>
					{/if}
					<div class="palette-group-title">Actions</div>
					<button class="palette-item" onclick={() => { commandPaletteOpen = false; createDocument(); }}>
						<span class="palette-icon"><Icon name="plus" size={15} /></span>
						<span>New page</span>
					</button>
					<button class="palette-item" onclick={() => { commandPaletteOpen = false; importMarkdownFiles((n) => { loadDocuments(); loadTags(); if (n > 0) alert(`Imported ${n} page${n > 1 ? 's' : ''}`); }); }}>
						<span class="palette-icon"><Icon name="download" size={15} /></span>
						<span>Import Markdown…</span>
					</button>
					<button class="palette-item" onclick={() => { commandPaletteOpen = false; exportVaultAsMarkdown().then((n) => { if (n > 0) alert(`Exported ${n} pages`); }); }}>
						<span class="palette-icon"><Icon name="upload" size={15} /></span>
						<span>Export vault as Markdown…</span>
					</button>
					<button class="palette-item" onclick={() => { commandPaletteOpen = false; sidebarOpen = !sidebarOpen; }}>
						<span class="palette-icon"><Icon name="chevronLeft" size={15} /></span>
						<span>Toggle sidebar</span>
					</button>
					<button class="palette-item" onclick={() => { commandPaletteOpen = false; shortcutsOpen = true; }}>
						<span class="palette-icon"><Icon name="text" size={15} /></span>
						<span>Keyboard shortcuts…</span>
					</button>
					<button class="palette-item" onclick={() => { commandPaletteOpen = false; theme.toggle(); }}>
						<span class="palette-icon"><Icon name={theme.value === 'dark' ? 'sun' : 'moon'} size={15} /></span>
						<span>Toggle theme</span>
					</button>
				</div>
			</div>
		</div>
	{/if}
</div>
{/if}

{#if !currentPath.startsWith('/capture')}
	<SettingsPanel bind:open={settingsOpen} onlock={() => (vaultUnlocked = false)} />

	<ShortcutsDialog bind:open={shortcutsOpen} />
{/if}

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
	.sidebar.collapsed { width: 48px; min-width: 48px; }
	:global([data-density="narrow"]) .sidebar { width: 220px; min-width: 220px; }
	:global([data-density="wide"]) .sidebar { width: 320px; min-width: 320px; }

	.sidebar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 10px 10px 14px;
	}

	.sidebar-brand {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--color-text);
		text-decoration: none;
	}
	.brand-mark {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		border-radius: 8px;
		background: var(--color-accent);
		color: #fff;
	}
	.brand-name { font-size: 15px; font-weight: 700; letter-spacing: -0.01em; }

	.sidebar-toggle {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 5px;
		border-radius: var(--radius-sm);
		display: flex;
	}
	.sidebar-toggle:hover { color: var(--color-text); background: var(--color-surface-hover); }

	/* ── Nav ── */
	.side-nav {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 6px 8px;
		border-bottom: 1px solid var(--color-border);
	}
	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 10px;
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		text-decoration: none;
		font-size: 14px;
		transition: background 0.1s, color 0.1s;
	}
	.nav-item:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.nav-item.active { background: var(--color-accent-subtle); color: var(--color-text); }

	/* ── Pages ── */
	.pages-section {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.section-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 16px 6px;
	}
	.section-title {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
	}
	.page-count {
		font-size: 11px;
		color: var(--color-text-faint);
		background: var(--color-surface-hover);
		padding: 1px 7px;
		border-radius: 10px;
	}

	.page-tree {
		flex: 1;
		overflow-y: auto;
		padding: 2px 8px 8px;
	}
	.tree-section-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-faint);
		padding: 8px 8px 2px;
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
		position: relative;
	}
	.tree-item:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.tree-item.active { background: var(--color-accent-subtle); color: var(--color-text); }

	.tree-item-icon { display: flex; color: var(--color-text-faint); flex-shrink: 0; }
	.tree-item.active .tree-item-icon,
	.tree-item:hover .tree-item-icon { color: var(--color-accent); }
	.tree-item-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }

	.tree-item-actions {
		display: none;
		gap: 2px;
		flex-shrink: 0;
	}
	.tree-item:hover .tree-item-actions { display: flex; }
	.tree-item.active .tree-item-actions { display: flex; }

	.row-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		border: none;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--color-text-faint);
		cursor: pointer;
		padding: 0;
	}
	.row-btn:hover { background: var(--color-surface-active); color: var(--color-text); }
	.row-btn.danger-row:hover { color: var(--color-danger); background: rgba(229, 83, 75, 0.1); }

	.tree-empty {
		font-size: 13px;
		color: var(--color-text-faint);
		padding: 10px 8px;
		line-height: 1.5;
	}
	.tree-empty kbd {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 3px;
		padding: 1px 4px;
		font-size: 11px;
		font-family: var(--font-mono);
	}

	.tag-list {
		display: flex;
		flex-direction: column;
		padding: 0 6px 8px;
	}

	.tag-row {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		padding: 4px 8px;
		border-radius: 6px;
		cursor: pointer;
		text-align: left;
	}

	.tag-row:hover,
	.tag-row.active {
		background: rgba(124, 111, 240, 0.12);
	}

	.tag-row-hash {
		color: #9d8cff;
		font-weight: 600;
	}

	.tag-row-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tag-row-count {
		font-size: 11px;
		color: var(--color-text-muted);
		background: var(--color-surface-hover);
		border-radius: 999px;
		padding: 0 7px;
	}
	.link-btn {
		background: none;
		border: none;
		padding: 0;
		color: var(--color-accent);
		cursor: pointer;
		font-size: 13px;
	}
	.link-btn:hover { text-decoration: underline; }

	/* ── Footer ── */
	.sidebar-footer {
		border-top: 1px solid var(--color-border);
		padding: 10px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.new-page-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 7px 10px;
		border: none;
		border-radius: var(--radius-md);
		background: var(--color-accent-subtle);
		color: var(--color-text);
		cursor: pointer;
		font-size: 14px;
		font-weight: 500;
		transition: background 0.15s;
	}
	.new-page-btn:hover { background: var(--color-accent);
		color: #fff; }

	.footer-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}
	.sync-status {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--color-text-faint);
	}
	.sync-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--color-border-strong);
	}
	.sync-status.online .sync-dot { background: var(--color-success); }
	.sync-status.online { color: var(--color-text-muted); }

	.footer-actions { display: flex; gap: 2px; }
	.icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		border-radius: var(--radius-md);
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0;
		transition: background 0.1s, color 0.1s;
	}
	.icon-btn:hover { background: var(--color-surface-hover); color: var(--color-text); }

	.peer-list { display: flex; flex-direction: column; gap: 4px; }
	.peer-item { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-text-faint); }
	.peer-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--color-border-strong); }
	.peer-dot.connected { background: var(--color-success); }

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
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
		padding: 5px;
		min-width: 180px;
	}
	.context-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 7px 10px;
		border: none;
		border-radius: var(--radius-sm);
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
	.context-item.danger:hover { background: rgba(229, 83, 75, 0.12); }
	.context-sep { height: 1px; background: var(--color-border); margin: 4px 6px; }

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
		background: var(--color-overlay);
		display: flex;
		justify-content: center;
		padding-top: 14vh;
		backdrop-filter: blur(2px);
	}
	.command-palette {
		width: 560px;
		max-width: 90vw;
		max-height: 420px;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}
	.palette-input-wrap {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 14px 18px;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text-faint);
	}
	.palette-input {
		flex: 1;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 16px;
		font-family: inherit;
		outline: none;
	}
	.palette-input::placeholder { color: var(--color-text-faint); }
	.palette-kbd {
		font-size: 11px;
		color: var(--color-text-faint);
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 2px 6px;
		font-family: var(--font-mono);
	}

	.palette-results { flex: 1; overflow-y: auto; padding: 6px; }
	.palette-group-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
		padding: 8px 10px 4px;
	}
	.palette-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 8px 10px;
		border: none;
		border-radius: var(--radius-md);
		background: none;
		color: var(--color-text);
		text-decoration: none;
		font-size: 14px;
		font-family: inherit;
		cursor: pointer;
		text-align: left;
		transition: background 0.1s;
	}
	.palette-item:hover { background: var(--color-surface-hover); }
	.palette-icon { display: flex; color: var(--color-text-faint); }
	.palette-item-text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.palette-item-title {
		font-weight: 500;
	}

	.palette-item-snippet {
		font-size: 12px;
		color: var(--color-text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.palette-empty {
		padding: 14px;
		text-align: center;
		color: var(--color-text-faint);
		font-size: 13px;
	}
</style>
