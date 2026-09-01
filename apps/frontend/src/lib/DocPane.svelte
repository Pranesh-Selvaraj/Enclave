<script lang="ts">
	import { invoke } from '$lib/backend.js';
	import type { Document, Block } from '@enclave/ui';
	import { TipTapEditor } from '@enclave/editor';
	import { saveWithRetry } from '$lib/saveRetry.js';

	let {
		docId,
		editor = $bindable(undefined as any),
	}: {
		docId: string;
		editor?: any;
	} = $props();

	let doc = $state<Document | null>(null);
	let title = $state('');
	let loading = $state(true);
	let content = $state<object | undefined>(undefined);
	let saveState = $state<'saved' | 'saving' | 'error'>('saved');
	let saveVersion = 0;
	let titleTimer: ReturnType<typeof setTimeout>;
	let contentTimer: ReturnType<typeof setTimeout>;
	// Last serialized JSON — kept so a pending save can still fire after the
	// editor instance has been torn down (pane unmount).
	let latestJson: unknown = null;

	async function load() {
		loading = true;
		content = undefined;
		title = '';
		latestJson = null;
		try {
			doc = await invoke<Document>('get_document', { id: docId });
			title = doc.title || '';
			const blocks = await invoke<Block[]>('get_blocks', { documentId: docId });
			const contentBlock = blocks.find((b) => {
				if (typeof b.content === 'object' && b.content !== null) {
					return (b.content as { type?: string }).type === 'doc';
				}
				return false;
			});
			if (contentBlock) content = contentBlock.content as object;
		} catch (e) {
			console.error('Failed to load document for split pane:', e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		load();
	});

	async function saveTitle() {
		if (!doc) return;
		try {
			doc = await invoke<Document>('update_document_title', { id: docId, title });
		} catch (e) {
			console.error('Failed to save title:', e);
		}
	}

	function onTitleInput() {
		clearTimeout(titleTimer);
		titleTimer = setTimeout(saveTitle, 500);
	}

	async function saveContent() {
		const json = latestJson;
		if (json == null) return;
		const expected = ++saveVersion;
		saveState = 'saving';
		const res = await saveWithRetry(
			() =>
				invoke('upsert_block', {
					id: `${docId}-content`,
					documentId: docId,
					blockType: 'doc',
					content: json,
					sortOrder: 0,
				}),
			() => saveVersion,
			expected,
		);
		if (saveVersion === expected) saveState = res.ok ? 'saved' : 'error';
		// ponytail: no RAG embedding rebuild from split panes — embeddings
		// refresh the next time the doc is saved from its own page.
	}

	function onEditorChange() {
		latestJson = editor?.getJSON() ?? latestJson;
		clearTimeout(contentTimer);
		contentTimer = setTimeout(saveContent, 1000);
	}

	// Flush pending saves on unmount (switching pages/panes).
	$effect(() => {
		return () => {
			if (titleTimer) {
				clearTimeout(titleTimer);
				saveTitle();
			}
			if (contentTimer) {
				clearTimeout(contentTimer);
				saveContent();
			}
		};
	});
</script>

{#if loading}
	<div class="pane-loading">Loading…</div>
{:else if doc}
	<div class="pane">
		<input
			class="pane-title"
			bind:value={title}
			oninput={onTitleInput}
			onblur={saveTitle}
			placeholder="Untitled"
			aria-label="Page title"
		/>
		<div class="pane-status {saveState}" role="status">
			{saveState === 'saving' ? 'Saving…' : saveState === 'error' ? 'Save failed' : ''}
		</div>
		<TipTapEditor
			bind:editor
			content={content}
			placeholder="Type / for commands…"
			onChange={onEditorChange}
		/>
	</div>
{:else}
	<div class="pane-missing">Document not found.</div>
{/if}

<style>
	.pane {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		padding: 0 22px 32px;
	}

	.pane-title {
		flex-shrink: 0;
		font-size: 19px;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: var(--color-text);
		background: none;
		border: none;
		outline: none;
		font-family: inherit;
		padding: 4px 0;
		width: 100%;
	}
	.pane-title::placeholder { color: var(--color-text-faint); }

	.pane-status {
		flex-shrink: 0;
		font-size: 11px;
		color: var(--color-text-faint);
		min-height: 14px;
	}
	.pane-status.error { color: var(--color-danger); font-weight: 500; }

	.pane-loading,
	.pane-missing {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-muted);
		font-size: 14px;
	}
</style>
