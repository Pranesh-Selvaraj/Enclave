<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '@enclave/ui';
	import { theme } from '@enclave/ui';

	let {
		open = $bindable(false),
		onlock,
	}: {
		open?: boolean;
		onlock?: () => void;
	} = $props();

	let vaultPath = $state('~/.local/share/com.enclave.app/enclave.db');
	let appVersion = $state('0.1.0');

	async function lockVault() {
		try {
			await invoke('lock_vault');
			open = false;
			onlock?.();
		} catch { /* ignore */ }
	}

	function handleBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Settings" onclick={() => (open = false)} onkeydown={handleBackdropKeydown}>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="settings-panel" role="document" onclick={(e: MouseEvent) => e.stopPropagation()}>
			<div class="settings-header">
				<h2>Settings</h2>
				<button class="settings-close" onclick={() => (open = false)}>✕</button>
			</div>

			<div class="settings-section">
				<h3>Appearance</h3>
				<div class="setting-row">
					<span>Theme</span>
					<button class="theme-toggle" onclick={() => theme.toggle()}>
						{theme.value === 'dark' ? '☀ Light' : '🌙 Dark'}
					</button>
				</div>
			</div>

			<div class="settings-section">
				<h3>Security</h3>
				<div class="setting-row">
					<span>Lock vault</span>
					<button class="danger-btn" onclick={lockVault}>Lock now</button>
				</div>
			</div>

			<div class="settings-section">
				<h3>Keyboard Shortcuts</h3>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>K</kbd> <span>Command palette</span></div>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>N</kbd> <span>New page</span></div>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>B</kbd> <span>Toggle sidebar</span></div>
				<div class="shortcut-row"><kbd>/</kbd> <span>Slash commands in editor</span></div>
				<div class="shortcut-row"><kbd>[[</kbd> <span>Link to page</span></div>
			</div>

			<div class="settings-section">
				<h3>About</h3>
				<div class="about-info">
					<div class="about-row"><span>Version</span><span class="about-value">{appVersion}</span></div>
					<div class="about-row"><span>Vault</span><span class="about-value">{vaultPath || '~/.local/share/com.enclave.app/'}</span></div>
				</div>
			</div>

			<div class="settings-footer">
				<Button onclick={() => (open = false)}>Close</Button>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		z-index: 300;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.settings-panel {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		width: 420px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.3);
	}
	.settings-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid var(--color-border);
	}
	.settings-header h2 { font-size: 16px; font-weight: 600; margin: 0; }
	.settings-close {
		background: none; border: none; color: var(--color-text-muted);
		cursor: pointer; font-size: 16px; padding: 4px;
	}
	.settings-section {
		padding: 12px 20px;
		border-bottom: 1px solid var(--color-border);
	}
	.settings-section h3 {
		font-size: 11px; font-weight: 600; text-transform: uppercase;
		letter-spacing: 0.05em; color: var(--color-text-muted); margin: 0 0 8px;
	}
	.setting-row {
		display: flex; align-items: center; justify-content: space-between; font-size: 14px;
	}
	.theme-toggle {
		background: var(--color-surface-hover); border: 1px solid var(--color-border);
		border-radius: var(--radius-md); color: var(--color-text);
		padding: 4px 12px; font-size: 13px; cursor: pointer; font-family: inherit;
	}
	.danger-btn {
		background: none; border: 1px solid var(--color-danger); color: var(--color-danger);
		border-radius: var(--radius-md); padding: 4px 12px; font-size: 13px;
		cursor: pointer; font-family: inherit;
	}
	.danger-btn:hover { background: var(--color-danger); color: white; }
	.shortcut-row {
		display: flex; align-items: center; gap: 6px;
		font-size: 13px; color: var(--color-text-muted); margin: 6px 0;
	}
	.shortcut-row kbd {
		background: var(--color-surface-hover); border: 1px solid var(--color-border);
		border-radius: 3px; padding: 1px 5px; font-size: 11px; font-family: var(--font-mono);
	}
	.about-info { display: flex; flex-direction: column; gap: 6px; }
	.about-row { display: flex; justify-content: space-between; font-size: 13px; color: var(--color-text-muted); }
	.about-value { color: var(--color-text); font-family: var(--font-mono); font-size: 12px; max-width: 220px; overflow: hidden; text-overflow: ellipsis; }
	.settings-footer {
		padding: 14px 20px; display: flex; justify-content: flex-end;
	}
</style>
