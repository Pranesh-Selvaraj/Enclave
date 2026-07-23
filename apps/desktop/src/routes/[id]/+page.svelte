<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import { TipTapEditor, SlashMenu, BubbleMenu } from '@enclave/editor';
	import type { Document, Block } from '@enclave/ui';
	import { htmlToMarkdown } from '@enclave/editor';

	let document = $state<Document | null>(null);
	let documentTitle = $state('');
	let editor = $state(undefined as any);
	let loading = $state(true);
	let backlinks = $state<Array<{ doc_id: string; doc_title: string; block_content: string }>>([]);
	let editorContent = $state<object | undefined>(undefined);

	const docId = $derived($page.params.id);

	// ── Debounce helpers ──
	let titleSaveTimer: ReturnType<typeof setTimeout>;
	let contentSaveTimer: ReturnType<typeof setTimeout>;

	async function loadDocument() {
		try {
			document = await invoke<Document>('get_document', { id: docId });
			documentTitle = document.title || '';
		} catch (e) {
			console.error('Failed to load document:', e);
		}
	}

	async function loadBlocks() {
		try {
			const blocks = await invoke<Block[]>('get_blocks', { documentId: docId });
			if (blocks.length > 0) {
				// Find the first block that has actual content JSON
				const contentBlock = blocks.find(b => {
					if (typeof b.content === 'object' && b.content !== null) {
						// Check if it looks like a TipTap doc (has "type": "doc")
						if ((b.content as any).type === 'doc') return true;
						// Check if it has text content
						if ((b.content as any).text) return true;
					}
					return false;
				});
				if (contentBlock && (contentBlock.content as any).type === 'doc') {
					editorContent = contentBlock.content as object;
				}
			}
		} catch (e) {
			console.error('Failed to load blocks:', e);
		} finally {
			loading = false;
		}
	}

	async function loadBacklinks() {
		if (!documentTitle) return;
		try {
			backlinks = await invoke<typeof backlinks>('get_backlinks', { title: documentTitle });
		} catch (e) {
			// Backlinks are non-critical; ignore errors
		}
	}

	async function saveTitle() {
		if (!document) return;
		try {
			document = await invoke<Document>('update_document_title', {
				id: docId,
				title: documentTitle,
			});
		} catch (e) {
			console.error('Failed to save title:', e);
		}
	}

	function debouncedSaveTitle() {
		clearTimeout(titleSaveTimer);
		titleSaveTimer = setTimeout(saveTitle, 500);
	}

	async function saveContent(json: object) {
		if (!document) return;
		try {
			await invoke('upsert_block', {
				id: `${docId}-content`,
				documentId: docId,
				blockType: 'doc',
				content: json,
				sortOrder: 0,
			});
		} catch (e) {
			console.error('Failed to save content:', e);
		}
	}

	function handleEditorChange(json: object, _html: string) {
		// Auto-title: use first heading text if title is still "Untitled"
		if (documentTitle === 'Untitled' || documentTitle === '') {
			const doc = json as any;
			const firstBlock = doc?.content?.[0];
			if (firstBlock?.type === 'heading' && firstBlock?.content?.[0]?.text) {
				documentTitle = firstBlock.content[0].text;
				debouncedSaveTitle();
			}
		}
		// Debounced auto-save
		clearTimeout(contentSaveTimer);
		contentSaveTimer = setTimeout(() => saveContent(json), 1000);
	}

	async function deleteDocument() {
		if (!document || !confirm(`Delete "${documentTitle || 'Untitled'}"? This cannot be undone.`)) return;
		try {
			await invoke('delete_document', { id: docId });
			goto('/');
		} catch (e) {
			console.error('Failed to delete document:', e);
		}
	}

	async function exportMarkdown() {
		if (!editor) return;
		try {
			const html = editor.getHTML();
			const md = htmlToMarkdown(html);
			const filename = documentTitle || 'untitled';
			const path = await invoke<string>('export_markdown', { filename, contents: md });
			console.log('Exported to', path);
		} catch (e) {
			console.error('Export failed:', e);
		}
	}

	$effect(() => {
		if (docId) {
			loading = true;
			loadDocument();
			loadBlocks();
		}
	});

	$effect(() => {
		if (documentTitle && document) {
			loadBacklinks();
		}
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
				oninput={debouncedSaveTitle}
				placeholder="Untitled"
			/>
			<div class="doc-actions">
				<button class="export-btn" onclick={deleteDocument} title="Delete page">
					🗑
				</button>
				<button class="export-btn" onclick={exportMarkdown} title="Export as Markdown">
					📥 .md
				</button>
			</div>
		</div>

		<div class="doc-body">
			<div class="doc-editor">
				<TipTapEditor
					bind:editor
					content={editorContent}
					placeholder="Type / for commands…"
					autofocus
					onChange={handleEditorChange}
				/>
				<SlashMenu {editor} />
				<BubbleMenu {editor} />
			</div>

			{#if backlinks.length > 0}
				<aside class="backlinks-panel">
					<div class="backlinks-header">Backlinks ({backlinks.length})</div>
					{#each backlinks as bl}
						<a href="/{bl.doc_id}" class="backlink-item">
							<span class="backlink-doc">{bl.doc_title}</span>
							<span class="backlink-content">{bl.block_content.slice(0, 100)}</span>
						</a>
					{/each}
				</aside>
			{/if}
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
		max-width: 960px;
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

	.doc-actions {
		display: flex;
		align-items: center;
		gap: 4px;
		padding-bottom: 8px;
	}

	.export-btn {
		background: none;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 12px;
		padding: 4px 8px;
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
	.doc-title-input::placeholder { color: var(--color-text-muted); }

	.doc-body {
		display: flex;
		gap: 32px;
		flex: 1;
	}

	.doc-editor {
		flex: 1;
		padding-top: 8px;
		min-width: 0;
	}

	.backlinks-panel {
		width: 240px;
		flex-shrink: 0;
		border-left: 1px solid var(--color-border);
		padding-left: 20px;
		padding-top: 12px;
		overflow-y: auto;
	}

	.backlinks-header {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin-bottom: 12px;
	}

	.backlink-item {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px;
		border-radius: 8px;
		color: var(--color-text-muted);
		text-decoration: none;
		font-size: 13px;
		transition: background 0.1s;
		margin-bottom: 4px;
	}
	.backlink-item:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	.backlink-doc {
		font-size: 13px;
		font-weight: 500;
		color: var(--color-text);
	}

	.backlink-content {
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
