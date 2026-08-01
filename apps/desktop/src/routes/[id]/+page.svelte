<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import { TipTapEditor, SlashMenu, BubbleMenu, PageLinkMenu } from '@enclave/editor';
	import type { Document, Block } from '@enclave/ui';
	import { htmlToMarkdown } from '@enclave/editor';
	import { exportMarkdownDialog } from '$lib/importExport.js';
	import Icon from '$lib/Icon.svelte';
	import Whiteboard from '$lib/Whiteboard.svelte';

	let document = $state<Document | null>(null);
	let documentTitle = $state('');
	let editor = $state(undefined as any);
	let loading = $state(true);
	let backlinks = $state<Array<{ doc_id: string; doc_title: string; block_content: string }>>([]);
	let editorContent = $state<object | undefined>(undefined);
	let pageList = $state<{ id: string; title: string }[]>([]);
	let mode = $state<'paper' | 'whiteboard'>('paper');
	let tags = $state<string[]>([]);
	let tagInput = $state('');

	const docId = $derived($page.params.id);

	// ── Debounce helpers ──
	let titleSaveTimer: ReturnType<typeof setTimeout>;
	let contentSaveTimer: ReturnType<typeof setTimeout>;
	let tagSaveTimer: ReturnType<typeof setTimeout>;

	function saveTags() {
		try {
			invoke('upsert_block', {
				id: `${docId}-tags`,
				documentId: docId,
				blockType: 'tags',
				content: { tags },
				sortOrder: 2,
			});
		} catch (e) {
			console.error('Failed to save tags:', e);
		}
	}

	function addTag() {
		const t = tagInput.trim().replace(/^#/, '');
		if (t && !tags.includes(t)) tags = [...tags, t];
		tagInput = '';
		clearTimeout(tagSaveTimer);
		tagSaveTimer = setTimeout(saveTags, 500);
	}

	function removeTag(t: string) {
		tags = tags.filter((x) => x !== t);
		clearTimeout(tagSaveTimer);
		tagSaveTimer = setTimeout(saveTags, 500);
	}

	function setMode(m: 'paper' | 'whiteboard') {
		mode = m;
		try { localStorage.setItem(`enclave-mode-${docId}`, m); } catch { /* ignore */ }
	}

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
			const tagsBlock = blocks.find(b => b.type === 'tags');
			if (tagsBlock?.content && Array.isArray((tagsBlock.content as any).tags)) {
				tags = ((tagsBlock.content as any).tags as unknown[]).filter(t => typeof t === 'string');
			}
			if (blocks.length > 0) {
				const contentBlock = blocks.find(b => {
					if (typeof b.content === 'object' && b.content !== null) {
						if ((b.content as any).type === 'doc') return true;
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

	async function loadPageList() {
		try {
			pageList = await invoke<{ id: string; title: string }[]>('get_page_list');
		} catch (e) {
			console.error('Failed to load page list:', e);
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

	function handleEditorChange(json: object) {
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

	async function toggleFavorite() {
		if (!document) return;
		try {
			document = await invoke<Document>('toggle_favorite', { id: docId });
		} catch (e) {
			console.error('Failed to toggle favorite:', e);
		}
	}

	async function exportMarkdown() {
		if (!editor) return;
		try {
			const md = htmlToMarkdown(editor.getHTML());
			const ok = await exportMarkdownDialog(documentTitle, md);
			if (ok) console.log('Exported');
		} catch (e) {
			console.error('Export failed:', e);
		}
	}

	$effect(() => {
		if (docId) {
			loading = true;
			// Reset stale content so a failed block load can't leak the
			// previous document's content into this one.
			editorContent = undefined;
			tags = [];
			tagInput = '';
			loadDocument();
			loadBlocks();
			loadPageList();
			try { mode = (localStorage.getItem(`enclave-mode-${docId}`) as 'paper' | 'whiteboard') || 'paper'; } catch { mode = 'paper'; }
		}
	});

	$effect(() => {
		if (docId && editor && editor.storage.image?.docId !== docId) {
			editor.storage.image.docId = docId;
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
				aria-label="Page title"
			/>
			<div class="doc-actions">
				<div class="mode-toggle" role="tablist" aria-label="Page mode">
					<button class="mode-btn" class:active={mode === 'paper'} onclick={() => setMode('paper')} role="tab">Paper</button>
					<button class="mode-btn" class:active={mode === 'whiteboard'} onclick={() => setMode('whiteboard')} role="tab">Whiteboard</button>
				</div>
				<button class="icon-btn" class:faved={document.is_favorite} onclick={toggleFavorite} title={document.is_favorite ? 'Remove from favorites' : 'Add to favorites'}>
					<Icon name="star" size={15} />
				</button>
				<button class="icon-btn" onclick={exportMarkdown} title="Export as Markdown">
					<Icon name="download" size={15} />
				</button>
			<button class="icon-btn danger" onclick={deleteDocument} title="Delete page">
				<Icon name="trash" size={15} />
			</button>
		</div>
	</div>

	<div class="doc-tags">
		{#each tags as t (t)}
			<span class="tag-chip">
				<span class="tag-hash">#</span>{t}
				<button class="tag-x" aria-label={`Remove tag ${t}`} onclick={() => removeTag(t)}>✕</button>
			</span>
		{/each}
		<input
			class="tag-input"
			placeholder={tags.length ? 'Add tag…' : 'Add tags…'}
			bind:value={tagInput}
			onkeydown={(e: KeyboardEvent) => {
				if (e.key === 'Enter' || e.key === ',') {
					e.preventDefault();
					addTag();
				} else if (e.key === 'Backspace' && tagInput === '' && tags.length > 0) {
					removeTag(tags[tags.length - 1]);
				}
			}}
			onblur={() => { if (tagInput.trim()) addTag(); }}
			aria-label="Add tag"
		/>
	</div>

		{#if mode === 'paper'}
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
					<PageLinkMenu {editor} allPages={pageList} />
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
		{:else}
			<Whiteboard docId={docId!} />
		{/if}
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

	.doc-tags {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
		padding: 0 40px 4px;
	}

	.tag-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		border-radius: 999px;
		background: rgba(124, 111, 240, 0.12);
		color: #9d8cff;
		font-size: 12px;
		padding: 2px 10px;
	}

	.tag-hash {
		opacity: 0.7;
	}

	.tag-x {
		border: none;
		background: none;
		color: inherit;
		opacity: 0.6;
		cursor: pointer;
		font-size: 10px;
		padding: 0;
	}

	.tag-x:hover {
		opacity: 1;
	}

	.tag-input {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		outline: none;
		width: 100px;
	}

	.document-page {
		max-width: 1100px;
		margin: 0 auto;
		padding: 0 48px;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.doc-topbar {
		padding: 20px 0 10px;
		display: flex;
		align-items: center;
		gap: 16px;
		flex-shrink: 0;
	}

	.doc-title-input {
		flex: 1;
		min-width: 0;
		font-size: 30px;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: var(--color-text);
		background: none;
		border: none;
		outline: none;
		font-family: inherit;
		padding: 4px 0;
	}
	.doc-title-input::placeholder { color: var(--color-text-faint); }

	.doc-actions {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}

	.mode-toggle {
		display: flex;
		background: var(--color-surface-hover);
		border-radius: var(--radius-md);
		padding: 2px;
		margin-right: 8px;
	}
	.mode-btn {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		font-family: inherit;
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background 0.1s, color 0.1s;
	}
	.mode-btn.active {
		background: var(--color-surface-active);
		color: var(--color-text);
		font-weight: 500;
	}

	.icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		border: none;
		border-radius: var(--radius-md);
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0;
		transition: background 0.1s, color 0.1s;
	}
	.icon-btn:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.icon-btn.faved { color: var(--color-warning); }
	.icon-btn.danger:hover { color: var(--color-danger); background: rgba(229, 83, 75, 0.1); }

	.doc-body {
		display: flex;
		gap: 32px;
		flex: 1;
		min-height: 0;
	}

	.doc-editor {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		padding-bottom: 80px;
	}

	.backlinks-panel {
		width: 220px;
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
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
		margin-bottom: 12px;
	}
	.backlink-item {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px;
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		text-decoration: none;
		font-size: 13px;
		transition: background 0.1s;
		margin-bottom: 4px;
	}
	.backlink-item:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.backlink-doc { font-size: 13px; font-weight: 500; color: var(--color-text); }
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
	.empty-state a { color: var(--color-accent); text-decoration: none; }
</style>
