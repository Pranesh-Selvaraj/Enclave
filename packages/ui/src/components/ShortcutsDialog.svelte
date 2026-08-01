<script lang="ts">
	let {
		open = $bindable(false),
	}: {
		open?: boolean;
	} = $props();

	const SHORTCUTS: [string[], string][] = [
		[['Ctrl+K'], 'Command palette'],
		[['Ctrl+N'], 'New page'],
		[['Ctrl+B'], 'Toggle sidebar'],
		[['/'], 'Slash menu (in editor)'],
		[['Ctrl+B'], 'Bold (editor)'],
		[['Ctrl+I'], 'Italic (editor)'],
		[['Ctrl+Shift+7'], 'Numbered list'],
		[['Ctrl+Shift+8'], 'Bullet list'],
		[['Ctrl+Shift+9'], 'Task list'],
		[['Ctrl+Alt+1', 'Ctrl+Alt+2', 'Ctrl+Alt+3'], 'Heading 1–3'],
		[['Ctrl+E'], 'Code block (editor)'],
		[['Tab', 'Shift+Tab'], 'Indent / outdent (lists)'],
		[['Ctrl+Z', 'Ctrl+Shift+Z'], 'Undo / redo'],
		[['?'], 'This dialog'],
		[['Esc'], 'Close dialogs'],
	];
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="sd-overlay" onclick={() => (open = false)}>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			class="sd-dialog"
			role="dialog"
			aria-modal="true"
			aria-label="Keyboard shortcuts"
			onclick={(e: MouseEvent) => e.stopPropagation()}
		>
			<div class="sd-header">
				<span>Keyboard shortcuts</span>
				<button class="sd-close" onclick={() => (open = false)} aria-label="Close">✕</button>
			</div>
			<div class="sd-list">
				{#each SHORTCUTS as [keys, desc]}
					<div class="sd-row">
						<span class="sd-desc">{desc}</span>
						<span class="sd-keys">
							{#each keys as k}
								<kbd>{k}</kbd>
							{/each}
						</span>
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.sd-overlay {
		position: fixed;
		inset: 0;
		z-index: 300;
		background: var(--color-overlay);
		display: flex;
		justify-content: center;
		align-items: flex-start;
		padding-top: 12vh;
		backdrop-filter: blur(2px);
	}

	.sd-dialog {
		width: 480px;
		max-width: 90vw;
		max-height: 60vh;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.sd-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 16px;
		border-bottom: 1px solid var(--color-border);
		font-size: 14px;
		font-weight: 600;
	}

	.sd-close {
		border: none;
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 12px;
	}

	.sd-list {
		overflow-y: auto;
		padding: 8px;
	}

	.sd-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 6px 8px;
		font-size: 13px;
	}

	.sd-desc {
		color: var(--color-text);
	}

	.sd-keys {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
		justify-content: flex-end;
	}

	.sd-keys kbd {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 1px 6px;
		font-size: 11px;
		font-family: var(--font-mono);
		color: var(--color-text-muted);
	}
</style>
