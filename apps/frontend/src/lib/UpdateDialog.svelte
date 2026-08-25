<script lang="ts">
	import { listen } from './backend.js';
	import { checkForUpdate, downloadUpdate, installUpdate, type UpdateInfo } from './updates.js';

	let {
		open = $bindable(false),
	}: {
		open?: boolean;
	} = $props();

	// idle → checking → available | none | error | installing
	let phase = $state<'idle' | 'checking' | 'available' | 'none' | 'error'>('idle');
	let info = $state<UpdateInfo | null>(null);
	let error = $state('');
	/** Per-update consent — the install button stays disabled until the user
	 *  reads the changelog and checks this box. */
	let agreed = $state(false);
	let busy = $state(false);
	let progress = $state(0);
	let statusText = $state('');

	// The check only ever runs when this dialog is opened from Settings —
	// there is no automatic check anywhere (offline-first).
	$effect(() => {
		if (!open) return;
		phase = 'checking';
		info = null;
		error = '';
		agreed = false;
		busy = false;
		progress = 0;
		statusText = '';
		checkForUpdate()
			.then((r) => {
				info = r;
				phase = r.update_available ? 'available' : 'none';
			})
			.catch((e: any) => {
				error = e?.message || String(e);
				phase = 'error';
			});
	});

	$effect(() => {
		if (!open) return;
		let unlisten: (() => void) | undefined;
		listen<{ received: number; total: number; percent: number }>('update-progress', (e) => {
			progress = e.payload?.percent ?? 0;
		}).then((fn) => (unlisten = fn));
		return () => unlisten?.();
	});

	function fmtSize(bytes: number | null): string {
		if (!bytes) return '';
		return bytes >= 1 << 20 ? `${(bytes / (1 << 20)).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`;
	}

	async function updateNow() {
		if (!info?.asset_url || !info.asset_name || busy) return;
		busy = true;
		progress = 0;
		statusText = 'Downloading…';
		try {
			const path = await downloadUpdate(info.asset_url, info.asset_name);
			statusText = 'Installing…';
			await installUpdate(path);
			progress = 100;
			statusText = 'Update installed — relaunch Enclave to use the new version.';
		} catch (e: any) {
			console.error('Update failed:', e);
			statusText = `Update failed: ${e?.message || e}`;
			busy = false;
		}
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Update check" onclick={() => { if (!busy) open = false; }} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape' && !busy) open = false; }}>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="update-dialog" role="document" onclick={(e: MouseEvent) => e.stopPropagation()}>
			<div class="dialog-header">
				<h2>Update check</h2>
				<button class="dialog-close" onclick={() => (open = false)} aria-label="Close" disabled={busy}>✕</button>
			</div>

			<div class="dialog-body">
				{#if phase === 'checking'}
					<div class="checking">
						<span class="spinner"></span>
						<span>Checking GitHub Releases…</span>
					</div>

				{:else if phase === 'none'}
					<div class="up-to-date">
						<span class="check-badge">✓</span>
						<p>You're on the latest version (<b>{info?.current_version}</b>).</p>
						<p class="hint">This was a one-off check — Enclave won't check again until you ask it to.</p>
					</div>

				{:else if phase === 'available' && info}
					<div class="version-row">
						<span class="version-label">Enclave <b>{info.latest_version}</b> is available</span>
						<span class="version-sub">You're on {info.current_version}</span>
						{#if info.asset_name || info.asset_size}
							<span class="version-asset">{info.asset_name}{info.asset_size ? ` · ${fmtSize(info.asset_size)}` : ''}</span>
						{/if}
					</div>
					<div class="notes-label">Changelog</div>
					<div class="notes">{info.notes || 'No release notes published for this version.'}</div>
					<label class="agree-row">
						<input type="checkbox" bind:checked={agreed} disabled={busy} />
						<span>I've reviewed the changelog and want to install this update.</span>
					</label>

				{:else if phase === 'error'}
					<div class="error" role="alert">
						<p>Could not check for updates.</p>
						<p class="hint">{error}</p>
						<p class="hint">The check only runs on demand — Enclave is otherwise fully offline.</p>
					</div>
				{/if}

				{#if busy}
					<div class="progress-row">
						<div class="progress-track"><div class="progress-fill" style="width:{progress}%"></div></div>
						<span class="progress-label">{statusText}</span>
					</div>
				{/if}
			</div>

			<div class="dialog-footer">
				<button class="btn secondary" onclick={() => (open = false)} disabled={busy}>Close</button>
				{#if phase === 'available' && info}
					<button class="btn primary" onclick={updateNow} disabled={!agreed || busy}>
						{busy ? 'Working…' : 'Download & install'}
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		z-index: 400;
		background: rgba(0, 0, 0, 0.45);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 16px;
	}
	.update-dialog {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 14px;
		width: 520px;
		max-width: 100%;
		max-height: min(88vh, 640px);
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
		display: flex;
		flex-direction: column;
	}
	.dialog-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 18px;
		border-bottom: 1px solid var(--color-border);
	}
	.dialog-header h2 { font-size: 15px; font-weight: 600; margin: 0; }
	.dialog-close {
		background: none; border: none; color: var(--color-text-muted);
		cursor: pointer; font-size: 14px; padding: 4px;
	}
	.dialog-close:disabled { opacity: 0.4; cursor: default; }

	.dialog-body { flex: 1; overflow-y: auto; padding: 16px 18px; display: flex; flex-direction: column; gap: 12px; }

	.checking { display: flex; align-items: center; gap: 10px; font-size: 14px; color: var(--color-text-muted); padding: 12px 0; }
	.spinner {
		width: 16px; height: 16px; flex-shrink: 0;
		border: 2px solid var(--color-border);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin { to { transform: rotate(360deg); } }

	.up-to-date { text-align: center; padding: 18px 0 10px; font-size: 14px; color: var(--color-text); }
	.check-badge {
		display: inline-flex; align-items: center; justify-content: center;
		width: 40px; height: 40px; border-radius: 50%;
		background: color-mix(in srgb, var(--color-success) 15%, transparent);
		color: var(--color-success); font-size: 18px; margin-bottom: 10px;
	}
	.hint { font-size: 12px; color: var(--color-text-faint); line-height: 1.5; }

	.version-row { display: flex; flex-direction: column; gap: 3px; }
	.version-label { font-size: 15px; font-weight: 600; }
	.version-sub { font-size: 12px; color: var(--color-text-muted); }
	.version-asset { font-size: 12px; color: var(--color-text-faint); font-family: var(--font-mono); word-break: break-all; }

	.notes-label {
		font-size: 11px; font-weight: 600; text-transform: uppercase;
		letter-spacing: 0.05em; color: var(--color-text-muted);
	}
	.notes {
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		padding: 12px 14px;
		max-height: 240px;
		overflow-y: auto;
		font-size: 13px;
		line-height: 1.6;
		white-space: pre-wrap;
		word-break: break-word;
		color: var(--color-text-muted);
	}

	.agree-row {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		font-size: 13px;
		color: var(--color-text);
		line-height: 1.5;
		cursor: pointer;
		padding: 4px 0;
	}
	.agree-row input { margin-top: 3px; accent-color: var(--color-accent); }

	.error { font-size: 13px; color: var(--color-danger); }
	.error .hint { color: var(--color-text-faint); }

	.progress-row { display: flex; flex-direction: column; gap: 6px; padding: 6px 0; }
	.progress-track { height: 6px; border-radius: 3px; background: var(--color-surface-hover); overflow: hidden; }
	.progress-fill { height: 100%; background: var(--color-accent); transition: width 0.15s; }
	.progress-label { font-size: 12px; color: var(--color-text-muted); }

	.dialog-footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		padding: 12px 18px;
		border-top: 1px solid var(--color-border);
	}
	.btn {
		border: none;
		border-radius: var(--radius-md);
		padding: 8px 14px;
		font-size: 13px;
		font-weight: 500;
		font-family: inherit;
		cursor: pointer;
	}
	.btn.primary { background: var(--color-accent); color: #fff; }
	.btn.primary:hover { background: var(--color-accent-hover); }
	.btn.primary:disabled { opacity: 0.45; cursor: default; }
	.btn.secondary { background: var(--color-surface-hover); color: var(--color-text); border: 1px solid var(--color-border); }
	.btn.secondary:hover { background: var(--color-surface-active); }
	.btn.secondary:disabled { opacity: 0.45; cursor: default; }

	@media (max-width: 768px) {
		.modal-backdrop { align-items: flex-end; padding: 0; }
		.update-dialog {
			width: 100%;
			max-height: 88vh;
			border-radius: 16px 16px 0 0;
			border-bottom: none;
		}
	}
</style>
