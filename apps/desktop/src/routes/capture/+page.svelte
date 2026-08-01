<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	let note = $state('');
	let status = $state<'loading' | 'ready' | 'locked' | 'novault'>('loading');
	let saving = $state(false);

	onMount(async () => {
		// Auto-unlock with the stored key (password vaults). Seed-phrase vaults
		// keep no key file — user must unlock the main window first.
		try {
			const init = await invoke<boolean>('is_vault_initialized');
			if (!init) {
				status = 'novault';
				return;
			}
			try {
				const key = await invoke<number[]>('load_vault_key');
				await invoke('unlock_vault', { key });
				status = 'ready';
			} catch {
				status = 'locked';
			}
		} catch {
			status = 'novault';
		}
	});

	function close() {
		getCurrentWindow().close();
	}

	async function save() {
		const text = note.trim();
		if (!text || saving) return;
		saving = true;
		try {
			const title = text.split('\n')[0].slice(0, 80) || 'Quick note';
			const doc = await invoke<{ id: string }>('create_document', { title });
			await invoke('upsert_block', {
				id: `${doc.id}-content`,
				documentId: doc.id,
				blockType: 'doc',
				content: {
					type: 'doc',
					content: [{ type: 'paragraph', content: [{ type: 'text', text }] }],
				},
				sortOrder: 0,
			});
			close();
		} catch (e) {
			console.error('Quick capture failed:', e);
			saving = false;
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
			e.preventDefault();
			save();
		} else if (e.key === 'Escape') {
			close();
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<main class="capture">
	{#if status === 'loading'}
		<div class="hint">Loading…</div>
	{:else if status === 'novault'}
		<div class="hint">No vault yet — open Enclave and create one first.</div>
	{:else if status === 'locked'}
		<div class="hint">Vault is locked — unlock Enclave, then try Quick Capture again.</div>
	{:else}
		<textarea
			class="note"
			bind:value={note}
			placeholder="Capture a thought…"
			autofocus
			onkeydown={(e) => {
				if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
					// Enter creates the page instantly (single-line quick notes)
					e.preventDefault();
					save();
				}
			}}
		></textarea>
		<footer class="bar">
			<span class="count">{note.trim().length} chars</span>
			<span class="shortcut"><kbd>Enter</kbd> save · <kbd>Esc</kbd> close</span>
		</footer>
	{/if}
</main>

<style>
	.capture {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: var(--color-bg);
	}
	.note {
		flex: 1;
		border: none;
		outline: none;
		resize: none;
		background: none;
		color: var(--color-text);
		font-size: 15px;
		line-height: 1.6;
		font-family: inherit;
		padding: 14px 16px;
	}
	.note::placeholder { color: var(--color-text-faint); }
	.bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 16px;
		border-top: 1px solid var(--color-border);
		color: var(--color-text-faint);
		font-size: 12px;
	}
	.shortcut kbd {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 3px;
		padding: 1px 5px;
		font-size: 11px;
		font-family: var(--font-mono);
	}
	.hint {
		padding: 24px;
		color: var(--color-text-muted);
		font-size: 14px;
	}
</style>
