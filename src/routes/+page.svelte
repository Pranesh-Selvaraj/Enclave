<script lang="ts">
	import { Button } from '@enclave/ui';
	import { getStorage } from '$lib/storage';
	import type { Document } from '@enclave/ui';
	import { goto } from '$app/navigation';

	let documents = $state<Document[]>([]);
	let storage = getStorage();

	async function loadDocuments() {
		try {
			const s = await storage;
			documents = await s.getDocumentList();
		} catch (e) {
			console.error('Failed to load documents:', e);
		}
	}

	async function createAndOpen() {
		try {
			const s = await storage;
			const doc = await s.createDocument('Untitled');
			goto(`/${doc.id}`);
		} catch (e) {
			console.error('Failed to create document:', e);
		}
	}

	$effect(() => {
		loadDocuments();
	});
</script>

<div class="home-page">
	<div class="home-header">
		<h1 class="home-title">Enclave</h1>
		<p class="home-subtitle">Encrypted, local-first, private notes</p>
	</div>

	<div class="home-content">
		{#if documents.length === 0}
			<div class="home-empty">
				<div class="home-empty-icon">🔒</div>
				<h2>Welcome to Enclave</h2>
				<p>
					Create richly formatted documents with block-based editing.
					All data is encrypted and stored locally on your device.
				</p>
				<div class="home-actions">
					<Button onclick={createAndOpen}>Create your first page</Button>
				</div>
				<div class="home-shortcuts">
					<div class="shortcut-item">
						<kbd>Ctrl</kbd> + <kbd>K</kbd>
						<span>Command palette</span>
					</div>
					<div class="shortcut-item">
						<kbd>/</kbd>
						<span>Slash commands</span>
					</div>
					<div class="shortcut-item">
						<kbd>Ctrl</kbd> + <kbd>N</kbd>
						<span>New page</span>
					</div>
				</div>
			</div>
		{:else}
			<div class="home-recent">
				<h2>Recent pages</h2>
				<div class="recent-list">
					{#each documents as doc (doc.id)}
						<a href="/{doc.id}" class="recent-item">
							<span class="recent-icon">📄</span>
							<div class="recent-info">
								<span class="recent-title">{doc.title || 'Untitled'}</span>
								<span class="recent-date">{new Date(doc.updated_at).toLocaleDateString()}</span>
							</div>
						</a>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.home-page {
		max-width: 720px;
		margin: 0 auto;
		padding: 60px 32px;
	}

	.home-header {
		margin-bottom: 40px;
	}

	.home-title {
		font-size: 28px;
		font-weight: 700;
		margin: 0 0 4px;
	}

	.home-subtitle {
		color: var(--color-text-muted);
		font-size: 15px;
		margin: 0;
	}

	.home-empty {
		text-align: center;
		padding: 60px 20px;
	}

	.home-empty-icon {
		font-size: 48px;
		margin-bottom: 16px;
	}

	.home-empty h2 {
		font-size: 22px;
		font-weight: 600;
		margin: 0 0 8px;
	}

	.home-empty p {
		color: var(--color-text-muted);
		max-width: 420px;
		margin: 0 auto 24px;
		line-height: 1.6;
	}

	.home-actions {
		margin-bottom: 32px;
	}

	.home-shortcuts {
		display: flex;
		flex-direction: column;
		gap: 6px;
		align-items: center;
	}

	.shortcut-item {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		color: var(--color-text-muted);
	}

	kbd {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 1px 6px;
		font-size: 12px;
		font-family: var(--font-mono);
	}

	.home-recent h2 {
		font-size: 14px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin: 0 0 12px;
	}

	.recent-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.recent-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-radius: var(--radius-md);
		color: var(--color-text);
		text-decoration: none;
		transition: background 0.1s;
	}

	.recent-item:hover {
		background: var(--color-surface-hover);
	}

	.recent-icon { font-size: 20px; }

	.recent-info {
		display: flex;
		flex-direction: column;
	}

	.recent-title { font-size: 15px; }
	.recent-date { font-size: 12px; color: var(--color-text-muted); }
</style>
