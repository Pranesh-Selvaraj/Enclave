<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { invoke, listen } from '$lib/backend.js';
	import type { Document } from '@enclave/ui';
	import { theme, ShortcutsDialog, Icon, Logo } from '@enclave/ui';
	import VaultGuard from '$lib/VaultGuard.svelte';
	import SettingsPanel from '$lib/SettingsPanel.svelte';
	import { haptic } from '$lib/haptics.js';
	import { importMarkdownFiles, exportVaultAsMarkdown } from '$lib/importExport.js';

	let { children } = $props();

	let settingsOpen = $state(false);
	let shortcutsOpen = $state(false);
	theme.init();

	let vaultUnlocked = $state(false);
	let documents = $state<Document[]>([]);
	let archivedDocs = $state<Document[]>([]);
	let sidebarOpen = $state(true);
	// Phone layout: the sidebar becomes a slide-in drawer behind a hamburger.
	let isMobile = $state(false);
	$effect(() => {
		const mq = window.matchMedia('(max-width: 768px)');
		isMobile = mq.matches;
		sidebarOpen = !mq.matches;
		const onChange = (e: MediaQueryListEvent) => {
			isMobile = e.matches;
			sidebarOpen = !e.matches;
		};
		mq.addEventListener('change', onChange);
		return () => mq.removeEventListener('change', onChange);
	});
	// Close the drawer after navigating so the page isn't half-covered.
	$effect(() => {
		const _p = $page.url.pathname;
		if (isMobile) sidebarOpen = false;
	});

	// ── Android back button: closing the topmost overlay first ──
	// Opening an overlay pushes a same-URL history entry; the system back
	// gesture pops it and we close the overlay instead of leaving the app.
	// Desktop is untouched (isMobile guard).
	let uiStack = $state<string[]>([]);
	function openUI(name: 'drawer' | 'palette' | 'settings') {
		if (name === 'drawer') sidebarOpen = true;
		else if (name === 'palette') {
			commandPaletteOpen = true;
			paletteIndex = 0;
		}
		else settingsOpen = true;
		if (isMobile && !uiStack.includes(name)) {
			uiStack = [...uiStack, name];
			try { history.pushState({ enclave: name }, ''); } catch { /* custom scheme may reject pushState */ }
		}
	}
	$effect(() => {
		const onPop = () => {
			if (!isMobile || uiStack.length === 0) return;
			const top = uiStack[uiStack.length - 1];
			uiStack = uiStack.slice(0, -1);
			if (top === 'drawer') sidebarOpen = false;
			else if (top === 'palette') commandPaletteOpen = false;
			else settingsOpen = false;
		};
		window.addEventListener('popstate', onPop);
		return () => window.removeEventListener('popstate', onPop);
	});
	// Overlays also close via Escape/backdrop clicks — drop their stack entry
	// so a later back press doesn't land on a stale no-op entry.
	$effect(() => {
		const open = (n: string) => (n === 'drawer' ? sidebarOpen : n === 'palette' ? commandPaletteOpen : settingsOpen);
		const filtered = uiStack.filter(open);
		if (filtered.length !== uiStack.length) uiStack = filtered;
	});

	// ── Auto-lock after inactivity (privacy on a phone in your pocket) ──
	// Plain let, not $state: the interval only reads the latest value; making it
	// reactive would tear down and recreate the interval on every keystroke.
	let lastActivity = Date.now();
	$effect(() => {
		const bump = () => (lastActivity = Date.now());
		window.addEventListener('pointerdown', bump);
		window.addEventListener('keydown', bump);
		window.addEventListener('touchstart', bump);
		return () => {
			window.removeEventListener('pointerdown', bump);
			window.removeEventListener('keydown', bump);
			window.removeEventListener('touchstart', bump);
		};
	});
	$effect(() => {
		if (!vaultUnlocked || theme.lockAfter <= 0) return;
		const t = setInterval(() => {
			if (Date.now() - lastActivity > theme.lockAfter * 60_000) {
				invoke('lock_vault').then(() => (vaultUnlocked = false)).catch(() => {});
			}
		}, 15_000);
		return () => clearInterval(t);
	});

	// ── Snackbar (toast + optional undo) — replaces native confirm() for
	// non-destructive actions. Modern Android feedback. ──
	let snackbar = $state<{ msg: string; undoLabel?: string; undo?: () => void } | null>(null);
	let snackTimer: ReturnType<typeof setTimeout>;
	function showSnack(msg: string, undo?: () => void, undoLabel = 'Undo') {
		snackbar = { msg, undo, undoLabel };
		clearTimeout(snackTimer);
		snackTimer = setTimeout(() => (snackbar = null), 5000);
	}
	/** Permanent deletes go through this in-app confirm dialog (no native alert). */
	let confirmDelete = $state<Document | null>(null);
	let commandPaletteOpen = $state(false);
	let searchQuery = $state('');
	let debouncedQuery = $state('');
	// Keyboard navigation for the palette: arrow keys move a highlight over
	// the rendered items (search results, recents, actions), Enter activates.
	let paletteEl = $state<HTMLDivElement | undefined>();
	let paletteIndex = $state(0);
	function paletteItems(): HTMLElement[] {
		return Array.from(paletteEl?.querySelectorAll<HTMLElement>('.palette-item') ?? []);
	}
	function paletteMove(dir: 1 | -1) {
		const items = paletteItems();
		if (items.length === 0) return;
		paletteIndex = (paletteIndex + dir + items.length) % items.length;
	}
	function paletteActivate() {
		const items = paletteItems();
		const el = items[Math.min(paletteIndex, items.length - 1)];
		el?.click();
	}
	// Keep the highlight class in sync after renders (query keystrokes re-render
	// the list) — tracks searchResults/paletteIndex reactively.
	$effect(() => {
		const _r = searchResults;
		const items = paletteItems();
		if (items.length === 0) return;
		const idx = Math.min(paletteIndex, items.length - 1);
		items.forEach((el, i) => el.classList.toggle('palette-active', i === idx));
		items[idx]?.scrollIntoView({ block: 'nearest' });
	});
	let networkRunning = $state(false);
	let networkStatus = $state<{
		local_peer_id: string;
		running: boolean;
		port: number;
		peers: { id: string; host: string; port: number; connected: boolean; name: string }[];
		last_sync_at: number | null;
	} | null>(null);
	let lastSync = $state('');
	const connectedCount = $derived(networkStatus?.peers.filter(p => p.connected).length ?? 0);

	function timeAgo(ts: number): string {
		const s = Math.max(0, Math.floor((Date.now() - ts) / 1000));
		if (s < 60) return `${s}s`;
		const m = Math.floor(s / 60);
		if (m < 60) return `${m}m`;
		return `${Math.floor(m / 60)}h`;
	}
	let statusTimer: ReturnType<typeof setInterval>;
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
		try {
			await invoke('delete_document', { id });
			await loadArchived();
			showSnack('Page permanently deleted');
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

	// Trash flow: moving a page to trash is undoable — no scary native confirm,
	// just a snackbar with Undo (Notion-style). Only the permanent delete below
	// asks for explicit confirmation.
	async function deleteDocument(id: string) {
		const doc = documents.find(d => d.id === id);
		try {
			await invoke('archive_document', { id });
			await loadDocuments();
			await loadArchived();
			loadTags();
			showSnack(`Moved "${doc?.title || 'Untitled'}" to trash`, () => restoreDocument(id));
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
				clearInterval(statusTimer);
			} else {
				await invoke('start_network', { name: null });
				networkRunning = true;
				networkStatus = await invoke<typeof networkStatus>('network_status');
				// Poll so mDNS-discovered peers show up without an event bridge.
				statusTimer = setInterval(async () => {
					try {
						networkStatus = await invoke<typeof networkStatus>('network_status');
					} catch { /* ignore */ }
				}, 3000);
			}
		} catch (e) {
			console.error('Network toggle failed:', e);
			networkRunning = false;
		}
	}

	function handleSyncDone(e: { payload: { peer: string; docs_changed: number; blocks_changed: number } }) {
		const d = e?.payload ?? {};
		lastSync = `${new Date().toLocaleTimeString()} · +${d.docs_changed ?? 0} docs, +${d.blocks_changed ?? 0} blocks`;
		setTimeout(() => (lastSync = ''), 8000);
	}

	function showContextMenu(e: MouseEvent, doc: Document) {
		e.preventDefault();
		// Clamp to the viewport — unclamped, a tap near the edge renders the
		// menu off-screen (unusable on phones).
		contextMenu = {
			doc,
			x: Math.min(e.clientX, window.innerWidth - 190),
			y: Math.min(e.clientY, window.innerHeight - 150),
		};
	}

	function handleKeydown(e: KeyboardEvent) {
		const mod = e.ctrlKey || e.metaKey;
		if (commandPaletteOpen) {
			if (e.key === 'ArrowDown') {
				e.preventDefault();
				paletteMove(1);
				return;
			}
			if (e.key === 'ArrowUp') {
				e.preventDefault();
				paletteMove(-1);
				return;
			}
			if (e.key === 'Enter') {
				e.preventDefault();
				paletteActivate();
				return;
			}
		}
		if (mod && e.key === 'k') {
			e.preventDefault();
			commandPaletteOpen = !commandPaletteOpen;
			if (commandPaletteOpen) paletteIndex = 0;
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
	// Window the page tree: rendering thousands of rows at once is the
	// slowest thing on a big vault. Cap it; one tap reveals the rest.
	const PAGE_CAP = 120;
	let showAllPages = $state(false);
	const visiblePages = $derived(showAllPages ? regularPages : regularPages.slice(0, PAGE_CAP));

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

	// Listen for sync completions from the LAN sync task.
	$effect(() => {
		let unlisten: (() => void) | undefined;
		listen('sync-done', handleSyncDone).then((fn) => (unlisten = fn));
		return () => unlisten?.();
	});

	// Android: lock the vault when the app loses visibility (app switcher,
	// screen off, another app on top). Desktop keeps its behavior — minimizing
	// a window must not wipe the session (issue #2, docs/android-mobile.md).
	$effect(() => {
		if (!vaultUnlocked || !navigator.userAgent.includes('Android')) return;
		const onHide = () => {
			if (document.hidden) {
				invoke('lock_vault')
					.then(() => (vaultUnlocked = false))
					.catch(() => {});
			}
		};
		document.addEventListener('visibilitychange', onHide);
		return () => document.removeEventListener('visibilitychange', onHide);
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if currentPath.startsWith('/capture')}
	{@render children?.()}
{:else if !vaultUnlocked}
	<VaultGuard onunlock={() => (vaultUnlocked = true)} />
{:else}
<div class="app-shell">
	<!-- Left Sidebar (drawer on phones) -->
	<aside class="sidebar" class:collapsed={!sidebarOpen} class:open={sidebarOpen}>
		<div class="sidebar-header">
			<a href="/" class="sidebar-brand" title="Enclave home">
				<span class="brand-mark">
					<Logo size={20} />
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

					{#each visiblePages as doc (doc.id)}
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

					{#if !showAllPages && regularPages.length > PAGE_CAP}
						<button class="tree-show-all" onclick={() => (showAllPages = true)}>
							Show all {regularPages.length} pages
						</button>
					{/if}

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
									<button class="row-btn danger-row" onclick={() => (confirmDelete = doc)} title="Delete permanently">
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
						<button class="icon-btn" onclick={() => openUI('settings')} title="Settings">
							<Icon name="settings" size={15} />
						</button>
					</div>
				</div>
				{#if networkStatus?.peers?.length}
					<div class="peer-list">
						{#each networkStatus.peers as peer}
							<div class="peer-item" title={peer.host}>
								<span class="peer-dot" class:connected={peer.connected}></span>
								<span class="peer-label">{peer.name || peer.id.slice(0, 8)}…</span>
							</div>
						{/each}
					</div>
				{/if}
				{#if lastSync}
					<div class="last-sync" role="status">Synced {lastSync}</div>
				{/if}
				{#if networkRunning && networkStatus?.last_sync_at}
					<div class="last-sync" role="status">
						{connectedCount}/{networkStatus.peers.length} peers online · last sync {timeAgo(networkStatus.last_sync_at)} ago
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

	<!-- Tap-away backdrop for the phone drawer -->
	{#if isMobile && sidebarOpen}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="sidebar-backdrop" onclick={() => (sidebarOpen = false)}></div>
	{/if}

	<!-- Main Content Area -->
	<div class="content-area">
		<!-- Phone top bar (hidden on desktop): drawer trigger + search -->
		<header class="mobile-topbar">
			<button class="topbar-btn" onclick={() => { openUI('drawer'); haptic(); }} aria-label="Menu" title="Menu">
				<Icon name="menu" size={18} />
			</button>
			<a href="/" class="topbar-brand"><Logo size={22} /> Enclave</a>
			<div class="topbar-spacer"></div>
			<button class="topbar-btn" onclick={() => openUI('palette')} aria-label="Search" title="Search">
				<Icon name="search" size={18} />
			</button>
		</header>
		<div class="main-pane">
			{@render children?.()}
		</div>
	</div>

	<!-- Phone bottom navigation: Home / Graph / Settings -->
	{#if isMobile && !currentDocId}
		<nav class="bottom-nav" aria-label="Main">
			<a href="/" class="nav-tab" class:active={currentPath === '/'} onclick={() => haptic()} aria-current={currentPath === '/' ? 'page' : undefined}>
				<span class="nav-tab-pill"><Icon name="home" size={20} /></span>
				<span>Home</span>
			</a>
			<a href="/graph" class="nav-tab" class:active={currentPath === '/graph'} onclick={() => haptic()} aria-current={currentPath === '/graph' ? 'page' : undefined}>
				<span class="nav-tab-pill"><Icon name="graph" size={20} /></span>
				<span>Graph</span>
			</a>
			<button class="nav-tab" class:active={settingsOpen} onclick={() => { openUI('settings'); haptic(); }}>
				<span class="nav-tab-pill"><Icon name="settings" size={20} /></span>
				<span>Settings</span>
			</button>
		</nav>
	{/if}

	<!-- Snackbar: transient feedback + undo -->
	{#if snackbar}
		<div class="snackbar" role="status">
			<span class="snack-msg">{snackbar.msg}</span>
			{#if snackbar.undo}
				<button class="snack-undo" onclick={() => { snackbar!.undo?.(); snackbar = null; }}>{snackbar.undoLabel}</button>
			{/if}
		</div>
	{/if}

	<!-- In-app confirm for permanent delete -->
	{#if confirmDelete}
		{@const doc = confirmDelete}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="confirm-backdrop" role="alertdialog" aria-modal="true" aria-label="Confirm permanent delete" onclick={() => (confirmDelete = null)}>
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="confirm-dialog" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<h3>Delete permanently?</h3>
				<p>"{doc.title || 'Untitled'}" will be gone forever. This cannot be undone.</p>
				<div class="confirm-actions">
					<button class="confirm-btn secondary" onclick={() => (confirmDelete = null)}>Cancel</button>
					<button class="confirm-btn danger" onclick={() => { deleteDocumentPermanently(doc.id); confirmDelete = null; }}>Delete permanently</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Command Palette Overlay -->
	{#if commandPaletteOpen}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="overlay" role="dialog" aria-modal="true" aria-label="Command palette" tabindex="-1" onclick={() => (commandPaletteOpen = false)} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') commandPaletteOpen = false; }}>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="command-palette" bind:this={paletteEl} onclick={(e: MouseEvent) => e.stopPropagation()}>
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

	/* ── Phone drawer backdrop + top bar (hidden on desktop) ── */
	.sidebar-backdrop {
		position: fixed;
		inset: 0;
		z-index: 125;
		background: var(--color-overlay);
	}
	.mobile-topbar {
		display: none;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		padding-top: calc(8px + env(safe-area-inset-top));
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
		flex-shrink: 0;
	}
	.topbar-brand {
		font-size: 15px;
		font-weight: 700;
		letter-spacing: -0.01em;
		color: var(--color-text);
		text-decoration: none;
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.topbar-spacer { flex: 1; }
	.topbar-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: none;
		border-radius: var(--radius-md);
		background: none;
		color: var(--color-text);
		padding: 0;
	}
	.topbar-btn:active { background: var(--color-surface-hover); }

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
	/* Same specificity as the density rules above — must come last so a
	 * collapsed sidebar really collapses under any density. */
	:global([data-density="narrow"]) .sidebar.collapsed,
	:global([data-density="wide"]) .sidebar.collapsed { width: 48px; min-width: 48px; }

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
	.tree-show-all {
		display: block;
		width: 100%;
		border: 1px dashed var(--color-border-strong);
		border-radius: var(--radius-md);
		background: none;
		color: var(--color-text-muted);
		font-size: 13px;
		font-family: inherit;
		padding: 8px;
		margin-top: 6px;
		cursor: pointer;
	}
	.tree-show-all:hover { color: var(--color-accent); border-color: var(--color-accent); }

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
	.last-sync { font-size: 11px; color: var(--color-text-faint); }

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
	.content-area {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
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
	.palette-item.palette-active { background: var(--color-surface-hover); box-shadow: inset 2px 0 0 var(--color-accent); }
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

	/* ── Snackbar ── */
	.bottom-nav { display: none; }
	.snackbar {
		position: fixed;
		left: 50%;
		transform: translateX(-50%);
		bottom: 24px;
		z-index: 500;
		display: flex;
		align-items: center;
		gap: 14px;
		background: var(--color-text);
		color: var(--color-bg);
		border-radius: 999px;
		padding: 10px 18px;
		font-size: 13px;
		box-shadow: var(--shadow-lg);
		max-width: 90vw;
	}
	.snack-msg { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.snack-undo {
		background: none;
		border: none;
		color: var(--color-accent);
		font-weight: 600;
		font-size: 13px;
		font-family: inherit;
		cursor: pointer;
		padding: 0;
		flex-shrink: 0;
	}

	/* ── In-app confirm dialog (permanent delete) ── */
	.confirm-backdrop {
		position: fixed;
		inset: 0;
		z-index: 450;
		background: var(--color-overlay);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 20px;
	}
	.confirm-dialog {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 14px;
		width: 360px;
		max-width: 100%;
		padding: 20px;
		box-shadow: var(--shadow-lg);
	}
	.confirm-dialog h3 { font-size: 15px; font-weight: 600; margin: 0 0 8px; }
	.confirm-dialog p { font-size: 13px; color: var(--color-text-muted); line-height: 1.5; margin: 0 0 18px; word-break: break-word; }
	.confirm-actions { display: flex; justify-content: flex-end; gap: 8px; }
	.confirm-btn {
		border: none;
		border-radius: var(--radius-md);
		padding: 8px 14px;
		font-size: 13px;
		font-weight: 500;
		font-family: inherit;
		cursor: pointer;
	}
	.confirm-btn.secondary { background: var(--color-surface-hover); color: var(--color-text); border: 1px solid var(--color-border); }
	.confirm-btn.danger { background: var(--color-danger); color: #fff; }

	/* ── Phone layout ── */
	@media (max-width: 768px) {
		.sidebar,
		.sidebar.collapsed,
		:global([data-density="narrow"]) .sidebar,
		:global([data-density="wide"]) .sidebar {
			position: fixed;
			top: 0;
			bottom: 0;
			left: 0;
			z-index: 130;
			width: 280px;
			min-width: 280px;
			transform: translateX(-105%);
			transition: transform 0.22s ease;
			box-shadow: var(--shadow-lg);
		}
		.sidebar.open { transform: translateX(0); }
		.sidebar-toggle { display: none; }

		.mobile-topbar { display: flex; }

		/* Touch: no hover — row actions must be tappable without a long-press. */
		.tree-item-actions { display: flex; }

		/* Sidebar footer pads for gesture bar. */
		.sidebar-footer { padding-bottom: calc(10px + env(safe-area-inset-bottom)); }

		/* Bottom navigation: Home / Graph / Settings. */
		.bottom-nav {
			display: flex;
			position: fixed;
			left: 0;
			right: 0;
			bottom: 0;
			z-index: 110;
			background: color-mix(in srgb, var(--color-surface) 92%, transparent);
			backdrop-filter: blur(12px);
			-webkit-backdrop-filter: blur(12px);
			border-top: 1px solid var(--color-border);
			padding: 6px 12px calc(6px + env(safe-area-inset-bottom));
		}
		.nav-tab {
			flex: 1;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 2px;
			background: none;
			border: none;
			color: var(--color-text-faint);
			font-size: 11px;
			font-family: inherit;
			padding: 6px 0;
			border-radius: var(--radius-md);
			text-decoration: none;
			cursor: pointer;
			transition: color 0.15s;
		}
		.nav-tab.active { color: var(--color-accent); }
		.nav-tab:active { background: var(--color-surface-hover); }
		.nav-tab-pill {
			display: flex;
			padding: 4px 20px;
			border-radius: 999px;
			transition: background 0.15s;
		}
		.nav-tab.active .nav-tab-pill { background: var(--color-accent-subtle); }

		/* Keep page content clear of the fixed nav. */
		.main-pane { padding-bottom: calc(64px + env(safe-area-inset-bottom)); }

		/* Snackbar floats above the nav. */
		.snackbar {
			bottom: calc(76px + env(safe-area-inset-bottom));
			max-width: calc(100vw - 32px);
		}

		/* Command palette: near-full-screen, thumb-reachable. */
		.overlay { padding-top: 8vh; align-items: flex-start; }
		.command-palette {
			width: 94vw;
			max-width: 94vw;
			max-height: 82vh;
		}
		.palette-input-wrap { padding: 14px 16px; }
		.palette-item { padding: 12px 10px; }
	}
</style>
