<script lang="ts">
	let { open = $bindable(false), title = '', children }: {
		open?: boolean;
		title?: string;
		children: any;
	} = $props();
</script>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div class="modal-backdrop" onclick={() => (open = false)}>
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="modal-panel" onclick={(e: MouseEvent) => e.stopPropagation()}>
			{#if title}
				<div class="modal-header">
					<h2 class="modal-title">{title}</h2>
					<button class="modal-close" onclick={() => (open = false)}>✕</button>
				</div>
			{/if}
			<div class="modal-body">
				{@render children?.()}
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
	.modal-panel {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		min-width: 360px;
		max-width: 500px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.3);
	}
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px 0;
	}
	.modal-title {
		font-size: 16px;
		font-weight: 600;
		margin: 0;
	}
	.modal-close {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 16px;
		padding: 4px;
	}
	.modal-close:hover { color: var(--color-text); }
	.modal-body {
		padding: 16px 20px 20px;
	}
</style>
