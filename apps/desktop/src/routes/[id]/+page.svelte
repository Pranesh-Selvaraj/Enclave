<script lang="ts">
	import { page } from '$app/stores';
	import { invoke } from '@tauri-apps/api/core';
	import { TipTapEditor, SlashMenu, BubbleMenu } from '@enclave/editor';
	import type { Document } from '@enclave/ui';
	import { SyncEngine } from '@enclave/sync-engine';
	import { encryptWithPassword, decryptWithPassword } from '@enclave/crypto';
	import { htmlToMarkdown } from '@enclave/editor';

	let document = $state<Document | null>(null);
	let documentTitle = $state('');
	let editor = $state(undefined as any);
	let loading = $state(true);

	const docId = $derived($page.params.id);

	let syncEngine = $state<SyncEngine | null>(null);
	let syncReady = $state(false);
	let syncPeers = $state(0);

	async function loadDocument() {
		try {
			document = await invoke<Document>('get_document', { id: docId });
			documentTitle = document.title || '';
		} catch (e) {
			console.error('Failed to load document:', e);
		} finally {
			loading = false;
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

	// Initialize sync engine for this document
	$effect(() => {
		if (!docId) return;
		const engine = new SyncEngine(
			(_docId, payload) => {
				// ponytail: outgoing sync via Tauri invoke (deferred to full integration)
				console.debug('[sync] outgoing', _docId, payload.slice(0, 32) + '...');
			},
			{
				encrypt: async (plain) => {
					// ponytail: encrypt with transport key (uses password-based for demo)
					const result = await encryptWithPassword(
						new TextDecoder().decode(plain),
						'sync-transport-key-v1',
					);
					// Encode as compact binary
					const combined = new Uint8Array(result.salt.length + result.iv.length + new Uint8Array(result.ciphertext).length);
					combined.set(result.salt, 0);
					combined.set(result.iv, result.salt.length);
					combined.set(new Uint8Array(result.ciphertext), result.salt.length + result.iv.length);
					return combined;
				},
				decrypt: async (cipher) => {
					const salt = cipher.slice(0, 32);
					const iv = cipher.slice(32, 44);
					const ct = cipher.slice(44);
					const result = await decryptWithPassword({ salt, iv, ciphertext: ct.buffer }, 'sync-transport-key-v1');
					return new TextEncoder().encode(result);
				},
			},
		);
		const doc = engine.getDoc(docId);
		const _text = doc.getText('content');
		syncEngine = engine;
		syncReady = true;

		return () => {
			engine.destroyDoc(docId);
		};
	});

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
				{#if syncReady}
					<span class="sync-badge" title="Sync ready">
						<span class="sync-dot active"></span>
						Sync
					</span>
				{/if}
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

	.sync-badge {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 11px;
		color: var(--color-text-muted);
		white-space: nowrap;
	}

	.sync-dot.active {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-success);
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
