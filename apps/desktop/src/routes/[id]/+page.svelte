<script lang="ts">
	import { page } from '$app/stores';
	import { invoke } from '@tauri-apps/api/core';
	import { TipTapEditor, SlashMenu, BubbleMenu } from '@enclave/editor';
	import type { Document } from '@enclave/ui';

	let document = $state<Document | null>(null);
	let documentTitle = $state('');
	let editor = $state(undefined as any);
	let loading = $state(true);

	const docId = $derived($page.params.id);

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
