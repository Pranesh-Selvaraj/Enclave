<script lang="ts">
	import { Button } from '@enclave/ui';
	import { theme } from '@enclave/ui';

	let {
		open = $bindable(false),
	}: {
		open?: boolean;
	} = $props();
</script>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div class="modal-backdrop" onclick={() => (open = false)}>
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="settings-panel" onclick={(e: MouseEvent) => e.stopPropagation()}>
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
				<h3>Keyboard Shortcuts</h3>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>K</kbd> <span>Command palette</span></div>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>N</kbd> <span>New page</span></div>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>S</kbd> <span>Export as Markdown</span></div>
				<div class="shortcut-row"><kbd>/</kbd> <span>Slash commands</span></div>
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
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin: 0 0 8px;
	}
	.setting-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 14px;
	}
	.theme-toggle {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		padding: 4px 12px;
		font-size: 13px;
		cursor: pointer;
		font-family: inherit;
	}
	.shortcut-row {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		color: var(--color-text-muted);
		margin: 6px 0;
	}
	.shortcut-row kbd {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 3px;
		padding: 1px 5px;
		font-size: 11px;
		font-family: var(--font-mono);
	}
	.settings-footer {
		padding: 14px 20px;
		display: flex;
		justify-content: flex-end;
	}
</style>
