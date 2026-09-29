<script lang="ts">
	import { invoke } from '$lib/backend.js';
	import type { Document } from '@enclave/ui';
	import { theme, Icon, Logo } from '@enclave/ui';
	import { templates } from '@enclave/editor';
	import { goto } from '$app/navigation';
	import { haptic } from '$lib/haptics.js';

	let documents = $state<Document[]>([]);

	// Mobile notes layouts: list, gallery, thumbnails or icon tiles.
	let isMobile = $state(false);
	$effect(() => {
		const mq = window.matchMedia('(max-width: 768px)');
		isMobile = mq.matches;
		const onChange = (e: MediaQueryListEvent) => (isMobile = e.matches);
		mq.addEventListener('change', onChange);
		return () => mq.removeEventListener('change', onChange);
	});

	const viewMode = $derived(isMobile ? theme.homeView : 'list');
	let fabOpen = $state(false);

	async function loadDocuments() {
		try {
			documents = await invoke<Document[]>('get_document_list');
		} catch (e) {
			console.error('Failed to load documents:', e);
		}
	}

	async function createAndOpen() {
		try {
			haptic();
			const doc = await invoke<Document>('create_document', { title: 'Untitled' });
			goto(`/${doc.id}`);
		} catch (e) {
			console.error('Failed to create document:', e);
		}
	}

	async function createJournal() {
		try {
			const today = new Date().toISOString().slice(0, 10);
			const doc = await invoke<Document>('find_or_create_document', { title: today });
			goto(`/${doc.id}`);
		} catch (e) {
			console.error('Failed to create journal:', e);
		}
	}

	/** Create a page pre-filled from one of the editor templates. */
	async function createFromTemplate(tplId: string) {
		const tpl = templates.find((t) => t.id === tplId);
		if (!tpl) return;
		try {
			haptic();
			const doc = await invoke<Document>('create_document', { title: tpl.name });
			goto(`/${doc.id}?t=${tpl.id}`);
		} catch (e) {
			console.error('Failed to create from template:', e);
		}
	}

	$effect(() => { loadDocuments(); });

	const favorites = $derived(documents.filter(d => d.is_favorite));
	const hasRail = $derived(documents.length === 0 || !isMobile || favorites.length > 0);
	const recent = $derived(
		[...documents]
			.sort((a, b) => {
				if (theme.homeSort === 'title') return (a.title || '').localeCompare(b.title || '');
				if (theme.homeSort === 'created') return b.created_at.localeCompare(a.created_at);
				return b.updated_at.localeCompare(a.updated_at);
			})
			.slice(0, 8)
	);
	const greeting = $derived(
		new Date().getHours() < 12 ? 'Good morning' : new Date().getHours() < 18 ? 'Good afternoon' : 'Good evening'
	);

	function timeAgo(iso: string): string {
		const s = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 1000));
		if (s < 60) return 'just now';
		const m = Math.floor(s / 60);
		if (m < 60) return `${m}m ago`;
		const h = Math.floor(m / 60);
		if (h < 24) return `${h}h ago`;
		const d = Math.floor(h / 24);
		if (d < 7) return `${d}d ago`;
		return new Date(iso).toLocaleDateString();
	}
</script>

<div class="home-page">
	{#snippet docList(docs: Document[])}
		{#if viewMode === 'list'}
			<div class="doc-panel">
				{#each docs as doc (doc.id)}
					<a href="/{doc.id}" class="doc-row">
						<span class="row-icon" class:fav={doc.is_favorite}>
							<Icon name={doc.is_favorite ? 'star' : 'page'} size={14} />
						</span>
						<span class="row-title">{doc.title || 'Untitled'}</span>
						<span class="row-meta">{timeAgo(doc.updated_at)}</span>
						<span class="row-chev"><Icon name="chevronRight" size={14} /></span>
					</a>
				{/each}
			</div>
		{:else if viewMode === 'gallery'}
			<div class="doc-cards doc-gallery">
				{#each docs as doc (doc.id)}
					<a href="/{doc.id}" class="doc-card">
						<span class="card-preview" class:fav={doc.is_favorite}>
							<Icon name={doc.is_favorite ? 'star' : 'page'} size={26} />
						</span>
						<span class="card-meta">
							<span class="card-title">{doc.title || 'Untitled'}</span>
							<span class="card-time">{timeAgo(doc.updated_at)}</span>
						</span>
					</a>
				{/each}
			</div>
		{:else if viewMode === 'thumbs'}
			<div class="doc-cards doc-thumbs">
				{#each docs as doc (doc.id)}
					<a href="/{doc.id}" class="doc-card keep-card">
						<span class="keep-title">{doc.title || 'Untitled'}</span>
						<span class="keep-foot">
							{#if doc.is_favorite}<span class="keep-fav"><Icon name="star" size={13} /></span>{/if}
							<span class="keep-time">{timeAgo(doc.updated_at)}</span>
						</span>
					</a>
				{/each}
			</div>
		{:else}
			<div class="doc-icons">
				{#each docs as doc (doc.id)}
					<a href="/{doc.id}" class="doc-icon-tile" title={doc.title || 'Untitled'}>
						<span class="icon-tile-art" class:fav={doc.is_favorite}>
							<Icon name={doc.is_favorite ? 'star' : 'page'} size={20} />
						</span>
						<span class="icon-tile-title">{doc.title || 'Untitled'}</span>
					</a>
				{/each}
			</div>
		{/if}
	{/snippet}
	<div class="home-head">
		<div class="home-heading">
			<h1 class="home-title">{greeting}</h1>
			{#if documents.length === 0 || !isMobile}
				<p class="home-subtitle">Your encrypted workspace — everything stays on this device.</p>
			{/if}
		</div>
	</div>

	{#if documents.length === 0}
		<div class="home-empty">
			<div class="home-empty-icon"><Logo size={40} /></div>
			<h2>Welcome to Enclave</h2>
			<p>
				Create your first page or start today's journal. All data is encrypted
				and stored locally on your device.
			</p>
			<div class="home-tips">
				<div class="tip-row"><kbd>Ctrl+K</kbd> Command palette & search</div>
				<div class="tip-row"><kbd>Ctrl+N</kbd> New page</div>
				<div class="tip-row"><kbd>Ctrl+B</kbd> Toggle sidebar</div>
				<div class="tip-row"><kbd>/</kbd> Block commands in editor</div>
				<div class="tip-row"><kbd>[[</kbd> Link to another page</div>
			</div>
		</div>
	{:else}
		<div class="home-grid">
			<section class="home-main">
				<div class="sec-head">
					<h2 class="sec-title">Recent pages</h2>
					<span class="sec-count">{recent.length}</span>
				</div>
				{@render docList(recent)}
			</section>

			{#if hasRail}
			<aside class="home-rail">
				{#if documents.length === 0 || !isMobile}
				<div class="quick-actions">
					<button class="quick-btn" onclick={createJournal}>
						<span class="quick-icon"><Icon name="check" size={15} /></span>
						<span>Today's Journal</span>
					</button>
					<button class="quick-btn primary" onclick={createAndOpen}>
						<span class="quick-icon"><Icon name="plus" size={15} /></span>
						<span>New Page</span>
					</button>
				</div>
				{/if}

				{#if favorites.length > 0}
					<div class="sec-head">
						<h2 class="sec-title">Favorites</h2>
						<span class="sec-count">{favorites.length}</span>
					</div>
					{@render docList(favorites)}
				{/if}
			</aside>
			{/if}
		</div>
	{/if}

	<!-- Android-style FAB: opens a drop-up with the creation entries. Hidden
	     on desktop — the sidebar button + Ctrl+N cover it. -->
	{#if fabOpen}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="fab-backdrop" onclick={() => (fabOpen = false)}></div>
		<div class="fab-stack" role="menu" aria-label="Create">
			<button class="fab-pill" role="menuitem" onclick={() => { fabOpen = false; createAndOpen(); }}>
				<span class="fab-pill-icon"><Icon name="text" size={19} /></span>
				<span>Text</span>
			</button>
			<button class="fab-pill" role="menuitem" onclick={() => { fabOpen = false; createFromTemplate('checklist'); }}>
				<span class="fab-pill-icon"><Icon name="listChecks" size={19} /></span>
				<span>Checklist</span>
			</button>
			<button class="fab-pill" role="menuitem" onclick={() => { fabOpen = false; createJournal(); }}>
				<span class="fab-pill-icon"><Icon name="calendar" size={19} /></span>
				<span>Journal</span>
			</button>
			<button class="fab-pill" role="menuitem" onclick={() => { fabOpen = false; createFromTemplate('meeting'); }}>
				<span class="fab-pill-icon"><Icon name="page" size={19} /></span>
				<span>Meeting notes</span>
			</button>
			<button class="fab-pill" role="menuitem" onclick={() => { fabOpen = false; window.dispatchEvent(new CustomEvent('enclave:new-folder')); }}>
				<span class="fab-pill-icon"><Icon name="folder" size={19} /></span>
				<span>Folder</span>
			</button>
		</div>
	{/if}
	<button class="fab" class:open={fabOpen} onclick={() => { haptic(); fabOpen = !fabOpen; }} aria-label="New" aria-haspopup="menu" aria-expanded={fabOpen} title="New">
		<Icon name={fabOpen ? 'x' : 'plus'} size={24} />
	</button>
</div>

<style>
	/* Dashboard layout — uses the desktop width like a proper workspace:
	   recent pages lead on the left; quick actions + favorites rail on the
	   right (collapses to a single column on phones). */
	.home-page {
		max-width: 1560px;
		width: 100%;
		box-sizing: border-box;
		margin: 0 auto;
		padding: 24px 28px 72px;
	}

	.home-head { margin-bottom: 18px; }

	.home-title { font-size: 24px; font-weight: 700; margin: 0 0 4px; letter-spacing: -0.02em; }
	.home-subtitle { color: var(--color-text-muted); font-size: 13px; margin: 0; }

	.home-grid {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 300px;
		gap: 22px;
		align-items: start;
	}
	.home-main { min-width: 0; }

	/* Medium windows: a 300px rail squeezes the page list too hard — stack
	   the rail on top and lay its actions out side by side. */
	@media (max-width: 1080px) {
		/* minmax(0, 1fr): a bare 1fr minimum is min-content, and the nowrap
		   row titles blow the track past the viewport on phones. */
		.home-grid { grid-template-columns: minmax(0, 1fr); gap: 18px; }
		.home-main { order: 2; }
		.home-rail { order: 1; }
		.quick-actions { flex-direction: row; }
		.quick-btn { flex: 1; justify-content: center; }
	}

	.quick-actions {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-bottom: 20px;
	}

	.quick-btn {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 10px;
		width: 100%;
		padding: 10px 14px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-surface);
		color: var(--color-text);
		font-size: 13px;
		font-family: inherit;
		cursor: pointer;
		transition: background 0.15s, border-color 0.15s;
	}
	.quick-btn:hover { background: var(--color-surface-hover); border-color: var(--color-border-strong); }
	.quick-btn.primary { background: var(--color-accent); border-color: var(--color-accent); color: #fff; }
	.quick-btn.primary:hover { background: var(--color-accent-hover); border-color: var(--color-accent-hover); }
	.quick-icon { display: flex; }

	.home-empty {
		text-align: center;
		padding: 48px 24px;
		border: 1px dashed var(--color-border-strong);
		border-radius: var(--radius-xl);
		background: var(--color-surface);
	}
	.home-empty-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 56px;
		height: 56px;
		margin: 0 auto 16px;
		border-radius: var(--radius-lg);
		background: var(--color-accent-subtle);
		color: var(--color-accent);
	}
	.home-empty h2 { font-size: 19px; font-weight: 600; margin: 0 0 8px; }
	.home-empty p { color: var(--color-text-muted); max-width: 420px; margin: 0 auto 24px; line-height: 1.6; font-size: 14px; }

	.home-tips { display: flex; flex-direction: column; gap: 8px; align-items: center; }
	.tip-row { font-size: 13px; color: var(--color-text-muted); display: flex; align-items: center; gap: 6px; }
	kbd {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 1px 6px;
		font-size: 12px;
		font-family: var(--font-mono);
	}

	/* ── Dense matte panels ── */
	.sec-head {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0 0 8px;
	}
	.sec-title {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
		margin: 0;
	}
	.sec-count {
		font-size: 11px;
		color: var(--color-text-faint);
		background: var(--color-surface-hover);
		border-radius: 999px;
		padding: 1px 8px;
	}

	.doc-panel {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		overflow: hidden;
	}
	.doc-row {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 42px;
		padding: 0 12px 0 10px;
		color: var(--color-text);
		text-decoration: none;
		transition: background 0.1s;
	}
	.doc-row + .doc-row { border-top: 1px solid var(--color-border); }
	.doc-row:hover { background: var(--color-surface-hover); }

	.row-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		border-radius: 7px;
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
		flex-shrink: 0;
	}
	.row-icon.fav {
		background: color-mix(in srgb, var(--color-warning) 15%, transparent);
		color: var(--color-warning);
	}
	.row-title { font-size: 13.5px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.row-meta { font-size: 12px; color: var(--color-text-faint); flex-shrink: 0; }
	.row-chev { display: flex; color: var(--color-text-faint); opacity: 0; transition: opacity 0.1s; flex-shrink: 0; }
	.doc-row:hover .row-chev { opacity: 0.7; }

	/* Density preset from Settings also governs the dashboard rows. */
	:global([data-density="narrow"]) .doc-row { min-height: 34px; }
	:global([data-density="narrow"]) .row-icon { width: 22px; height: 22px; }
	:global([data-density="wide"]) .doc-row { min-height: 50px; }

	/* ── Phone layout ── */
	@media (max-width: 768px) {
		.home-page { padding: 14px 14px 64px; }
		.home-head { margin-bottom: 16px; }
		.home-title { font-size: 22px; }

		/* Single column; rail (quick actions + favorites) sits above recents. */
		.home-grid { grid-template-columns: minmax(0, 1fr); gap: 20px; }
		.home-main { order: 2; }
		.home-rail { order: 1; }
		.quick-actions { margin-bottom: 18px; gap: 8px; }

		/* Full-width, thumb-sized actions. */
		.quick-btn {
			justify-content: center;
			padding: 14px 16px;
			font-size: 15px;
			border-radius: 14px;
			min-height: 48px;
		}

		.doc-row { min-height: 52px; }
		.row-icon { width: 30px; height: 30px; }
		.row-chev { opacity: 0.6; }
		.doc-panel { border-radius: 14px; }

		/* Keyboard tips are meaningless on a phone. */
		.home-tips { display: none; }
		.home-empty { padding: 32px 20px; }
		.home-empty p { font-size: 13.5px; }
	}

	/* ── Mobile notes layouts (list is the desktop default) ── */
	.doc-cards { display: grid; gap: 10px; }
	.doc-gallery { grid-template-columns: minmax(0, 1fr); }
	.doc-thumbs { grid-template-columns: repeat(2, minmax(0, 1fr)); }
	.doc-card {
		display: flex;
		flex-direction: column;
		min-width: 0;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		color: var(--color-text);
		text-decoration: none;
		overflow: hidden;
	}
	.doc-card:active { background: var(--color-surface-hover); }
	.card-preview {
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
	}
	.doc-gallery .card-preview { height: 116px; }
	.card-preview.fav { color: var(--color-warning); }
	.card-meta { display: flex; flex-direction: column; gap: 2px; padding: 8px 10px 10px; min-width: 0; }
	.card-title { font-size: 13.5px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.card-time { font-size: 11.5px; color: var(--color-text-faint); }

	/* Keep-style cards: title-first, quiet footer, no preview chrome. */
	.keep-card {
		min-height: 118px;
		padding: 14px;
		border-radius: 16px;
		gap: 10px;
		justify-content: space-between;
	}
	.keep-title {
		font-size: 15px;
		font-weight: 600;
		line-height: 1.35;
		color: var(--color-text);
		display: -webkit-box;
		-webkit-line-clamp: 5;
		line-clamp: 5;
		-webkit-box-orient: vertical;
		overflow: hidden;
		overflow-wrap: anywhere;
	}
	.keep-foot {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 8px;
		color: var(--color-text-faint);
		font-size: 11.5px;
	}
	.keep-fav { display: flex; color: var(--color-warning); }
	.doc-icons { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
	.doc-icon-tile {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 10px 4px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-surface);
		color: var(--color-text);
		text-decoration: none;
	}
	.doc-icon-tile:active { background: var(--color-surface-hover); }
	.icon-tile-art {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
	}
	.icon-tile-art.fav { color: var(--color-warning); }
	.icon-tile-title {
		font-size: 11px;
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-muted);
	}

	/* Floating action button + Keep-style entry stack — phones only. */
	.fab { display: none; }
	.fab-backdrop, .fab-stack { display: none; }
	@media (max-width: 768px) {
		.fab {
			display: flex;
			align-items: center;
			justify-content: center;
			position: fixed;
			right: 16px;
			bottom: calc(20px + var(--safe-bottom));
			z-index: 126;
			width: 56px;
			height: 56px;
			border: none;
			border-radius: 18px;
			background: var(--color-accent);
			color: #fff;
			box-shadow: var(--shadow-md);
			cursor: pointer;
			transition: border-radius 0.15s, transform 0.08s ease-out, box-shadow 0.15s, background 0.15s, color 0.15s;
		}
		.fab:active { transform: scale(0.94); }
		/* Open: inverted circle with an X, like Keep. */
		.fab.open {
			border-radius: 999px;
			background: var(--color-text);
			color: var(--color-bg);
			box-shadow: var(--shadow-lg);
		}

		.fab-backdrop {
			display: block;
			position: fixed;
			inset: 0;
			z-index: 124;
		}
		.fab-stack {
			display: flex;
			flex-direction: column;
			align-items: flex-end;
			gap: 10px;
			position: fixed;
			right: 16px;
			bottom: calc(92px + var(--safe-bottom));
			z-index: 127;
		}
		.fab-pill {
			display: flex;
			align-items: center;
			gap: 12px;
			height: 52px;
			padding: 0 20px;
			border: 1px solid var(--color-border);
			border-radius: 999px;
			background: var(--color-surface);
			color: var(--color-text);
			font-family: inherit;
			font-size: 15px;
			font-weight: 600;
			box-shadow: var(--shadow-md);
			cursor: pointer;
			animation: fab-menu-in 0.16s cubic-bezier(0.32, 0.72, 0, 1);
		}
		.fab-pill:active { background: var(--color-surface-hover); }
		.fab-pill-icon { display: flex; color: var(--color-accent); }
	}
	@keyframes fab-menu-in {
		from { opacity: 0; transform: translateY(10px) scale(0.98); }
		to { opacity: 1; transform: none; }
	}
</style>
