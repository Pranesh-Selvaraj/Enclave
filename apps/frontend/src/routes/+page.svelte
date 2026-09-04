<script lang="ts">
	import { invoke } from '$lib/backend.js';
	import type { Document } from '@enclave/ui';
	import { theme, Icon, Logo } from '@enclave/ui';
	import { goto } from '$app/navigation';

	let documents = $state<Document[]>([]);

	async function loadDocuments() {
		try {
			documents = await invoke<Document[]>('get_document_list');
		} catch (e) {
			console.error('Failed to load documents:', e);
		}
	}

	async function createAndOpen() {
		try {
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

	$effect(() => { loadDocuments(); });

	const favorites = $derived(documents.filter(d => d.is_favorite));
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
	<div class="home-head">
		<div class="home-heading">
			<h1 class="home-title">{greeting}</h1>
			<p class="home-subtitle">Your encrypted workspace — everything stays on this device.</p>
		</div>
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
		{#if favorites.length > 0}
			<section class="home-section">
				<div class="sec-head">
					<h2 class="sec-title">Favorites</h2>
					<span class="sec-count">{favorites.length}</span>
				</div>
				<div class="doc-panel">
					{#each favorites as doc (doc.id)}
						<a href="/{doc.id}" class="doc-row">
							<span class="row-icon fav"><Icon name="star" size={14} /></span>
							<span class="row-title">{doc.title || 'Untitled'}</span>
							<span class="row-meta">{timeAgo(doc.updated_at)}</span>
							<span class="row-chev"><Icon name="chevronRight" size={14} /></span>
						</a>
					{/each}
				</div>
			</section>
		{/if}

		<section class="home-section">
			<div class="sec-head">
				<h2 class="sec-title">Recent pages</h2>
				<span class="sec-count">{recent.length}</span>
			</div>
			<div class="doc-panel">
				{#each recent as doc (doc.id)}
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
		</section>
	{/if}
</div>

<style>
	/* Dashboard layout — uses the desktop width like a proper workspace:
	   greeting + quick actions in one header row, then dense matte panels. */
	.home-page {
		max-width: 1080px;
		width: 100%;
		box-sizing: border-box;
		margin: 0 auto;
		padding: 30px 36px 72px;
	}

	.home-head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 20px;
		margin-bottom: 26px;
	}

	.home-title { font-size: 24px; font-weight: 700; margin: 0 0 4px; letter-spacing: -0.02em; }
	.home-subtitle { color: var(--color-text-muted); font-size: 13px; margin: 0; }

	.quick-actions { display: flex; gap: 8px; flex-shrink: 0; }

	.quick-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 14px;
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
	.home-section { margin-bottom: 26px; }
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

	/* ── Phone layout ── */
	@media (max-width: 768px) {
		.home-page { padding: 20px 16px 56px; }
		.home-head { flex-direction: column; align-items: stretch; gap: 16px; margin-bottom: 22px; }
		.home-title { font-size: 23px; }

		/* Full-width, thumb-sized actions instead of a cramped row. */
		.quick-actions { flex-direction: column; }
		.quick-btn {
			justify-content: center;
			padding: 13px 16px;
			font-size: 15px;
			border-radius: var(--radius-lg);
		}

		.doc-row { min-height: 50px; }
		.row-chev { opacity: 0.6; }
		.doc-panel { border-radius: var(--radius-lg); }

		/* Keyboard tips are meaningless on a phone. */
		.home-tips { display: none; }
		.home-empty { padding: 36px 20px; }
	}
</style>
