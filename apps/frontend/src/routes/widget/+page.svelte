<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '$lib/backend.js';
	import { Icon, Logo } from '@enclave/ui';

	let status = $state<'loading' | 'ready' | 'locked' | 'novault'>('loading');
	let recents = $state<{ id: string; title: string; updated_at: string }[]>([]);
	let note = $state('');
	let saving = $state(false);
	let refreshing = $state(false);
	let greeting = $state('');

	onMount(async () => {
		greeting = new Date().toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' });
		await refresh();
	});

	async function refresh() {
		try {
			const init = await invoke<boolean>('is_vault_initialized');
			if (!init) {
				status = 'novault';
				return;
			}
			try {
				const key = await invoke<number[]>('load_vault_key');
				await invoke('unlock_vault', { key });
			} catch {
				status = 'locked';
				return;
			}
			status = 'ready';
			refreshing = true;
			try {
				const docs = await invoke<{ id: string; title: string; updated_at: string }[]>('get_document_list');
				recents = docs.slice(0, 5);
			} finally {
				refreshing = false;
			}
		} catch {
			status = 'novault';
		}
	}

	async function openDoc(id: string) {
		try {
			await invoke('open_doc_from_widget', { id });
		} catch (e) {
			console.error('Failed to open doc from widget:', e);
		}
	}

	async function openApp() {
		try {
			await invoke('open_doc_from_widget', { id: '' });
		} catch (e) {
			console.error(e);
		}
	}

	async function quickCapture() {
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
			note = '';
			await openDoc(doc.id);
			await refresh();
		} catch (e) {
			console.error('Quick capture failed:', e);
		} finally {
			saving = false;
		}
	}

	function close() {
		invoke('hide_widget').catch(() => {});
	}
</script>

<main class="widget">
	<header class="w-head">
		<span class="w-brand"><Logo size={16} /></span>
		<span class="w-title">Enclave</span>
		<span class="w-spacer"></span>
		<button class="w-btn" onclick={refresh} title="Refresh" aria-label="Refresh">
			<Icon name="refresh" size={13} />
		</button>
		<button class="w-btn" onclick={close} title="Hide widget" aria-label="Hide widget">
			<Icon name="x" size={13} />
		</button>
	</header>

	{#if status === 'loading'}
		<div class="w-hint">Loading…</div>
	{:else if status === 'novault'}
		<div class="w-hint">
			<p>No vault yet — open Enclave and create one first.</p>
			<button class="w-primary" onclick={openApp}>Open Enclave</button>
		</div>
	{:else if status === 'locked'}
		<div class="w-hint">
			<p>Vault is locked.</p>
			<button class="w-primary" onclick={openApp}>Unlock Enclave</button>
		</div>
	{:else}
		<div class="w-date">{greeting}</div>

		<div class="w-capture">
			<input
				class="w-input"
				bind:value={note}
				placeholder="Quick note…"
				aria-label="Quick note"
				onkeydown={(e: KeyboardEvent) => {
					if (e.key === 'Enter') {
						e.preventDefault();
						quickCapture();
					}
				}}
			/>
			<button class="w-primary" onclick={quickCapture} disabled={saving || !note.trim()}>+</button>
		</div>

		<div class="w-section">Recent pages</div>
		<div class="w-list">
			{#each recents as r (r.id)}
				<button class="w-item" onclick={() => openDoc(r.id)}>
					<span class="w-item-icon"><Icon name="page" size={13} /></span>
					<span class="w-item-label">{r.title || 'Untitled'}</span>
				</button>
			{/each}
			{#if recents.length === 0}
				<div class="w-empty">No pages yet — capture your first note above.</div>
			{/if}
		</div>
	{/if}
</main>

<style>
	.widget {
		width: 100vw;
		height: 100vh;
		box-sizing: border-box;
		padding: 10px;
		background: transparent;
		display: flex;
		flex-direction: column;
		gap: 8px;
		overflow: hidden;
		color: var(--color-text);
		font-family: var(--font-sans);
	}

	/* Matte card over the wallpaper. The widget window is transparent;
	   app.css paints html/body with --color-bg, so force it transparent here
	   and let the solid matte card (with its own radius) float on top. */
	:global(html) {
		background: transparent !important;
	}
	:global(body) {
		background: transparent !important;
	}
	:global(.widget) {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 18px;
		box-shadow: var(--shadow-md);
	}

	.w-head {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.w-brand {
		display: flex;
	}
	.w-title {
		font-size: 13px;
		font-weight: 700;
		letter-spacing: -0.01em;
	}
	.w-spacer {
		flex: 1;
	}
	.w-btn {
		width: 26px;
		height: 26px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: 7px;
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.w-btn:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	.w-hint {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		font-size: 13px;
		color: var(--color-text-muted);
		text-align: center;
		padding: 12px;
	}
	.w-hint p {
		margin: 0;
	}

	.w-date {
		font-size: 13px;
		font-weight: 600;
		color: var(--color-text);
	}

	.w-capture {
		display: flex;
		gap: 6px;
	}
	.w-input {
		flex: 1;
		min-width: 0;
		background: var(--color-inset);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		color: var(--color-text);
		font-size: 13px;
		font-family: inherit;
		padding: 8px 10px;
		outline: none;
	}
	.w-input:focus {
		border-color: var(--color-accent);
	}
	.w-input::placeholder {
		color: var(--color-text-faint);
	}

	.w-primary {
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: 10px;
		background: var(--color-accent);
		color: #fff;
		font-size: 14px;
		font-weight: 600;
		font-family: inherit;
		padding: 0 14px;
		min-width: 34px;
		cursor: pointer;
	}
	.w-primary:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.w-section {
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-text-faint);
		padding-top: 4px;
	}

	.w-list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.w-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		font-family: inherit;
		text-align: left;
		padding: 7px 8px;
		border-radius: 8px;
		cursor: pointer;
	}
	.w-item:hover {
		background: var(--color-surface-hover);
	}
	.w-item-icon {
		display: flex;
		color: var(--color-text-faint);
		flex-shrink: 0;
	}
	.w-item-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.w-empty {
		font-size: 12px;
		color: var(--color-text-faint);
		padding: 8px;
	}
</style>
