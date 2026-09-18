<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { invoke } from '$lib/backend.js';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import VaultGuard from '$lib/VaultGuard.svelte';

	// Shared text (Android share target, widgets) arrives as ?text=… — read it
	// on mount; touching searchParams during prerender throws.
	let note = $state('');
	let status = $state<'loading' | 'ready' | 'locked' | 'novault'>('loading');
	let saving = $state(false);

	onMount(async () => {
		note = $page.url.searchParams.get('text') ?? '';
		try {
			const init = await invoke<boolean>('is_vault_initialized');
			if (!init) {
				status = 'novault';
				return;
			}
			try {
				// The vault is shared with the main app; a cold start or a share
				// from a backgrounded app is locked (the stored key file is
				// password-encrypted, by design) — the guard below unlocks inline.
				const unlocked = await invoke<boolean>('is_vault_unlocked');
				status = unlocked ? 'ready' : 'locked';
			} catch {
				status = 'locked';
			}
		} catch {
			status = 'novault';
		}
	});

	function handleUnlock() {
		// Tell the shell so it doesn't show the guard again on the way back.
		window.dispatchEvent(new CustomEvent('enclave:unlocked'));
		status = 'ready';
	}

	function leave() {
		// Android renders capture in the main window — land on the app home.
		// Desktop capture is its own popup window: close it.
		if (navigator.userAgent.includes('Android')) {
			goto('/');
			return;
		}
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
			leave();
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
			leave();
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
		<VaultGuard onunlock={handleUnlock} />
	{:else}
		<!-- svelte-ignore a11y_autofocus -->
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
		/* Dynamic viewport height: the software keyboard/toolbars on Android
		   must not push the composer under the screen. */
		height: 100dvh;
		/* Transparent: the window background carries the theme (incl. soft/
		   glassy gradients) so quick capture matches the main app. */
		background: transparent;
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
		padding: calc(14px + env(safe-area-inset-top)) 16px 14px;
	}
	.note::placeholder { color: var(--color-text-faint); }
	.bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 16px calc(8px + env(safe-area-inset-bottom));
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
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
		color: var(--color-text-muted);
		font-size: 14px;
		text-align: center;
	}
</style>
