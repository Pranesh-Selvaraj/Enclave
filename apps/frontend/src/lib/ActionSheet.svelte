<script lang="ts">
	import { Icon } from '@enclave/ui';

	let {
		open = $bindable(false),
		title = '',
		items = [] as { icon: string; label: string; danger?: boolean; action: () => void }[],
	}: {
		open?: boolean;
		title?: string;
		items?: { icon: string; label: string; danger?: boolean; action: () => void }[];
	} = $props();
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="sheet-backdrop" onclick={() => (open = false)}></div>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="sheet"
		role="dialog"
		aria-label={title || 'Actions'}
		onclick={(e: MouseEvent) => e.stopPropagation()}
		onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') open = false; }}
	>
		<div class="sheet-handle" aria-hidden="true"></div>
		{#if title}
			<div class="sheet-title">{title}</div>
		{/if}
		<div class="sheet-list">
			{#each items as item (item.label)}
				<button
					class="sheet-item"
					class:danger={item.danger}
					onclick={() => {
						open = false;
						item.action();
					}}
				>
					<span class="sheet-item-icon"><Icon name={item.icon} size={18} /></span>
					<span class="sheet-item-label">{item.label}</span>
				</button>
			{/each}
		</div>
	</div>
{/if}

<style>
	.sheet-backdrop {
		position: fixed;
		inset: 0;
		z-index: 400;
		background: var(--color-overlay);
	}

	.sheet {
		position: fixed;
		left: 8px;
		right: 8px;
		bottom: 8px;
		z-index: 401;
		/* Liquid glass: translucent + heavy blur over the page. */
		background: color-mix(in srgb, var(--color-surface) 62%, transparent);
		backdrop-filter: blur(28px) saturate(170%);
		-webkit-backdrop-filter: blur(28px) saturate(170%);
		border: 1px solid color-mix(in srgb, var(--color-border) 55%, transparent);
		border-radius: 22px;
		padding: 8px 10px calc(14px + env(safe-area-inset-bottom));
		box-shadow: 0 -8px 40px rgba(0, 0, 0, 0.4);
		animation: sheet-up 0.22s ease;
	}

	@keyframes sheet-up {
		from { transform: translateY(40px); opacity: 0.6; }
		to { transform: translateY(0); opacity: 1; }
	}

	.sheet-handle {
		width: 40px;
		height: 4px;
		border-radius: 999px;
		background: var(--color-border-strong);
		margin: 0 auto 10px;
		opacity: 0.6;
	}

	.sheet-title {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
		padding: 0 8px 6px;
	}

	.sheet-list {
		display: flex;
		flex-direction: column;
	}

	.sheet-item {
		display: flex;
		align-items: center;
		gap: 14px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 15px;
		font-family: inherit;
		text-align: left;
		padding: 14px 12px;
		border-radius: 12px;
		cursor: pointer;
		min-height: 52px;
		-webkit-tap-highlight-color: transparent;
	}
	.sheet-item:active {
		background: var(--color-surface-hover);
	}
	.sheet-item-icon {
		display: flex;
		color: var(--color-text-muted);
	}
	.sheet-item.danger {
		color: var(--color-danger);
	}
	.sheet-item.danger .sheet-item-icon {
		color: var(--color-danger);
	}
	.sheet-item-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
