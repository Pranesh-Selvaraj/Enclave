<script lang="ts">
	import { Button } from '@enclave/ui';
	import { invoke } from '@tauri-apps/api/core';
	import type { Document } from '@enclave/ui';
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
</script>

<div class="home-page">
	<div class="home-header">
		<h1 class="home-title">Enclave</h1>
		<p class="home-subtitle">Encrypted, local-first, private pages</p>
	</div>

	<div class="quick-actions">
		<button class="quick-btn" onclick={createJournal}>
			<span class="quick-icon">📅</span>
			<span>Today's Journal</span>
		</button>
		<button class="quick-btn" onclick={createAndOpen}>
			<span class="quick-icon">📝</span>
			<span>New Page</span>
		</button>
	</div>

	<div class="home-content">
		{#if documents.length === 0}
			<div class="home-empty">
				<div class="home-empty-icon">🔒</div>
				<h2>Welcome to Enclave</h2>
				<p>
					Create your first page or start today's journal. All data is encrypted and stored locally on your device.
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
			<div class="home-recent">
				<h2>Recent pages</h2>
				<div class="recent-list">
					{#each documents as doc (doc.id)}
						<a href="/{doc.id}" class="recent-item">
							<span class="recent-icon">{doc.is_favorite ? '★' : '📄'}</span>
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

	.home-header { margin-bottom: 32px; }
	.home-title { font-size: 28px; font-weight: 700; margin: 0 0 4px; }
	.home-subtitle { color: var(--color-text-muted); font-size: 15px; margin: 0; }

	.quick-actions {
		display: flex;
		gap: 10px;
		margin-bottom: 36px;
	}

	.quick-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 18px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-surface);
		color: var(--color-text);
		font-size: 14px;
		cursor: pointer;
		font-family: inherit;
		transition: background 0.15s, border-color 0.15s;
	}
	.quick-btn:hover { background: var(--color-surface-hover); border-color: var(--color-accent); }
	.quick-icon { font-size: 18px; }

	.home-empty {
		text-align: center;
		padding: 40px 20px;
		border: 1px dashed var(--color-border);
		border-radius: 16px;
	}
	.home-empty-icon { font-size: 48px; margin-bottom: 16px; }
	.home-empty h2 { font-size: 20px; font-weight: 600; margin: 0 0 8px; }
	.home-empty p { color: var(--color-text-muted); max-width: 420px; margin: 0 auto 24px; line-height: 1.6; }

	.home-tips {
		display: flex;
		flex-direction: column;
		gap: 8px;
		align-items: center;
	}
	.tip-row { font-size: 13px; color: var(--color-text-muted); display: flex; align-items: center; gap: 6px; }
	kbd {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 1px 6px;
		font-size: 12px;
		font-family: var(--font-mono);
	}

	.home-recent h2 {
		font-size: 13px; font-weight: 600; text-transform: uppercase;
		letter-spacing: 0.05em; color: var(--color-text-muted); margin: 0 0 12px;
	}
	.recent-list { display: flex; flex-direction: column; gap: 2px; }
	.recent-item {
		display: flex; align-items: center; gap: 12px;
		padding: 10px 12px; border-radius: var(--radius-md);
		color: var(--color-text); text-decoration: none; transition: background 0.1s;
	}
	.recent-item:hover { background: var(--color-surface-hover); }
	.recent-icon { font-size: 16px; width: 24px; text-align: center; }
	.recent-info { display: flex; flex-direction: column; }
	.recent-title { font-size: 15px; }
	.recent-date { font-size: 12px; color: var(--color-text-muted); }
</style>
