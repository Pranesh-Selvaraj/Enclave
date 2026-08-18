<script lang="ts">
	import { invoke, listen } from './backend.js';

	type UpdateInfo = {
		current_version: string;
		latest_version: string;
		update_available: boolean;
		notes: string;
		asset_name: string | null;
		asset_url: string | null;
		asset_size: number | null;
	};

	let info = $state<UpdateInfo | null>(null);
	let dismissed = $state(false);
	let busy = $state(false);
	let progress = $state(0);
	let statusText = $state('');

	// Check once per mount (the layout mounts this after vault unlock). Errors
	// are silent — offline or rate-limited users just don't see a banner.
	$effect(() => {
		invoke<UpdateInfo>('check_for_update')
			.then((r) => {
				if (r.update_available) info = r;
			})
			.catch(() => {});
	});

	$effect(() => {
		let unlisten: (() => void) | undefined;
		listen<{ received: number; total: number; percent: number }>('update-progress', (e) => {
			progress = e.payload?.percent ?? 0;
		}).then((fn) => (unlisten = fn));
		return () => unlisten?.();
	});

	async function updateNow() {
		if (!info?.asset_url || !info.asset_name || busy) return;
		busy = true;
		progress = 0;
		statusText = 'Downloading…';
		try {
			const path = await invoke<string>('download_update', {
				url: info.asset_url,
				filename: info.asset_name,
			});
			statusText = 'Installing…';
			await invoke('install_update', { path });
			progress = 100;
			statusText = 'Update installed — relaunch Enclave to use the new version.';
		} catch (e) {
			console.error('Update failed:', e);
			statusText = `Update failed: ${e}`;
			busy = false;
		}
	}
</script>

{#if info && !dismissed}
	<div class="update-banner" role="status">
		<div class="update-text">
			<span class="update-title">Enclave {info.latest_version} is available</span>
			<span class="update-sub" title={info.notes}>
				You're on {info.current_version} — updating keeps all your pages and settings.
			</span>
		</div>
		{#if busy}
			<div class="update-progress">
				<div class="progress-track"><div class="progress-fill" style="width:{progress}%"></div></div>
				<span class="progress-label">{statusText}</span>
			</div>
		{:else}
			<button class="update-btn" onclick={updateNow}>Update now</button>
			<button class="update-dismiss" onclick={() => (dismissed = true)} title="Dismiss" aria-label="Dismiss">
				✕
			</button>
		{/if}
	</div>
{/if}

<style>
	.update-banner {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 8px 16px;
		background: var(--color-accent-subtle);
		border-bottom: 1px solid var(--color-border);
		font-size: 13px;
	}
	.update-text {
		display: flex;
		flex-direction: column;
		gap: 1px;
		flex: 1;
		min-width: 0;
	}
	.update-title { font-weight: 600; color: var(--color-text); }
	.update-sub {
		color: var(--color-text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.update-progress {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 260px;
	}
	.progress-track {
		flex: 1;
		height: 6px;
		border-radius: 3px;
		background: var(--color-surface-hover);
		overflow: hidden;
	}
	.progress-fill {
		height: 100%;
		background: var(--color-accent);
		transition: width 0.15s;
	}
	.progress-label {
		font-size: 12px;
		color: var(--color-text-muted);
		white-space: nowrap;
	}
	.update-btn {
		border: none;
		border-radius: var(--radius-md);
		background: var(--color-accent);
		color: #fff;
		font-size: 13px;
		font-weight: 600;
		padding: 6px 14px;
		cursor: pointer;
		flex-shrink: 0;
	}
	.update-btn:hover { filter: brightness(1.1); }
	.update-dismiss {
		border: none;
		background: none;
		color: var(--color-text-faint);
		cursor: pointer;
		padding: 4px;
		font-size: 12px;
		display: flex;
	}
	.update-dismiss:hover { color: var(--color-text); }
</style>
