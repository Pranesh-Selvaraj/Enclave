<script lang="ts">
	import { page } from '$app/stores';
	import { getStorage } from '$lib/storage';
	import { TipTapEditor, SlashMenu, BubbleMenu } from '@enclave/editor';
	import type { Document } from '@enclave/ui';
	import { htmlToMarkdown } from '@enclave/editor';

	let document = $state<Document | null>(null);
	let documentTitle = $state('');
	let editor = $state(undefined as any);
	let loading = $state(true);
	let storage = getStorage();

	const docId = $derived($page.params.id);

	async function loadDocument() {
		if (!docId) return;
		try {
			const s = await storage;
			document = await s.getDocument(docId);
			documentTitle = document.title || '';
		} catch (e) {
			console.error('Failed to load document:', e);
		} finally {
			loading = false;
		}
	}

	async function saveTitle() {
		if (!document || !docId) return;
		try {
			const s = await storage;
			document = await s.updateDocumentTitle(docId, documentTitle);
		} catch (e) {
			console.error('Failed to save title:', e);
		}
	}

	async function exportMarkdown() {
		if (!editor) return;
		try {
			const html = editor.getHTML();
			const md = htmlToMarkdown(html);
			const filename = documentTitle || 'untitled';
			const s = await storage;
			const path = await s.exportMarkdown(filename, md);
			console.log('Exported to', path);
		} catch (e) {
			console.error('Export failed:', e);
		}
	}

	$effect(() => {
		if (docId) loadDocument();
	});
</script>

{#if loading}
	<div class="loading">Loading…</div>
{:else if document}
	<div class="document-page">
		<div class="doc-topbar">
			<input
				type="text"
				class="doc-title-input"
				bind:value={documentTitle}
				onblur={saveTitle}
				placeholder="Untitled"
			/>
			<div class="doc-actions">
				<button class="export-btn" onclick={exportMarkdown} title="Export as Markdown">
					📥 .md
				</button>
			</div>
		</div>

		<div class="doc-editor">
			<TipTapEditor
				bind:editor
				placeholder="Type / for commands…"
				autofocus
			/>
			<SlashMenu {editor} />
			<BubbleMenu {editor} />
		</div>
	</div>
{:else}
	<div class="empty-state">
		<p>Document not found.</p>
		<a href="/">Go home</a>
	</div>
{/if}

<style>
	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-muted);
	}

	.document-page {
		max-width: 800px;
		margin: 0 auto;
		padding: 0 64px;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.doc-topbar {
		padding: 24px 0 12px;
		display: flex;
		align-items: flex-end;
		gap: 12px;
	}

	.doc-title-input {
		flex: 1;
	}

	.doc-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		padding-bottom: 8px;
	}

	.export-btn {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 11px;
		padding: 3px 8px;
		font-family: inherit;
		transition: background 0.15s, color 0.15s;
		white-space: nowrap;
	}
	.export-btn:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	.doc-title-input {
		width: 100%;
		font-size: 32px;
		font-weight: 700;
		color: var(--color-text);
		background: none;
		border: none;
		outline: none;
		font-family: inherit;
		letter-spacing: -0.02em;
	}

	.doc-title-input::placeholder {
		color: var(--color-text-muted);
	}

	.doc-editor {
		flex: 1;
		padding-top: 8px;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		gap: 12px;
		color: var(--color-text-muted);
	}

	.empty-state a {
		color: var(--color-accent);
		text-decoration: none;
	}
</style>
