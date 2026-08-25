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
	<div class="home-header">
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

	<div class="home-content">
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
					<h2 class="section-title">Favorites</h2>
					<div class="recent-list">
						{#each favorites as doc (doc.id)}
							<a href="/{doc.id}" class="recent-item">
								<span class="recent-icon fav"><Icon name="star" size={14} /></span>
								<span class="recent-title">{doc.title || 'Untitled'}</span>
								<span class="recent-date">{timeAgo(doc.updated_at)}</span>
							</a>
						{/each}
					</div>
				</section>
			{/if}

			<section class="home-section">
				<h2 class="section-title">Recent pages</h2>
				<div class="recent-list">
					{#each recent as doc (doc.id)}
						<a href="/{doc.id}" class="recent-item">
							<span class="recent-icon"><Icon name="page" size={14} /></span>
							<span class="recent-title">{doc.title || 'Untitled'}</span>
							<span class="recent-date">{timeAgo(doc.updated_at)}</span>
						</a>
					{/each}
				</div>
			</section>
		{/if}
	</div>
</div>

<style>
	.home-page {
		max-width: 680px;
		margin: 0 auto;
		padding: 56px 32px 80px;
	}

	.home-header { margin-bottom: 28px; }
	.home-title { font-size: 26px; font-weight: 700; margin: 0 0 4px; letter-spacing: -0.02em; }
	.home-subtitle { color: var(--color-text-muted); font-size: 14px; margin: 0; }

	.quick-actions {
		display: flex;
		gap: 10px;
		margin-bottom: 36px;
	}

	.quick-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 9px 16px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-surface);
		color: var(--color-text);
		font-size: 14px;
		font-family: inherit;
		cursor: pointer;
		transition: background 0.15s, border-color 0.15s;
	}
	.quick-btn:hover { background: var(--color-surface-hover); border-color: var(--color-border-strong); }
	.quick-btn.primary { background: var(--color-accent); border-color: var(--color-accent); color: #fff; }
	.quick-btn.primary:hover { background: var(--color-accent-hover); }
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

	.home-section { margin-bottom: 28px; }
	.section-title {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
		margin: 0 0 10px;
	}
	/* Cards instead of plain rows — reads better as a page picker. */
	.recent-list {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}
	.recent-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 14px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		color: var(--color-text);
		text-decoration: none;
		transition: border-color 0.15s, transform 0.15s, box-shadow 0.15s;
	}
	.recent-item:hover {
		border-color: var(--color-border-strong);
		box-shadow: var(--shadow-sm);
		transform: translateY(-1px);
	}
	.recent-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 34px;
		height: 34px;
		border-radius: 9px;
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
		flex-shrink: 0;
	}
	.recent-icon.fav {
		background: color-mix(in srgb, var(--color-warning) 14%, transparent);
		color: var(--color-warning);
	}
	.recent-title { font-size: 14px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.recent-date { font-size: 12px; color: var(--color-text-faint); flex-shrink: 0; }

	/* ── Phone layout ── */
	@media (max-width: 768px) {
		.home-page { padding: 24px 16px 48px; }
		.home-title { font-size: 24px; }
		.home-subtitle { font-size: 13px; }

		/* Full-width, thumb-sized actions instead of a cramped row. */
		.quick-actions { flex-direction: column; margin-bottom: 28px; }
		.quick-btn {
			justify-content: center;
			padding: 14px 16px;
			font-size: 15px;
			border-radius: var(--radius-lg);
		}

		.recent-item { padding: 13px 12px; min-height: 48px; }
		.recent-date { flex-shrink: 0; }
		.recent-list { grid-template-columns: 1fr; }

		/* Keyboard tips are meaningless on a phone. */
		.home-tips { display: none; }
		.home-empty { padding: 36px 20px; }
	}
</style>
