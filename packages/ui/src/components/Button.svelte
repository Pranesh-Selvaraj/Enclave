<script lang="ts">
	let { children, onclick, disabled = false }: {
		children: any;
		onclick?: () => void;
		disabled?: boolean;
	} = $props();
</script>

<button class="btn" {onclick} {disabled}>
	{@render children?.()}
</button>

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 8px 16px;
		border-radius: 8px;
		font-size: 14px;
		font-weight: 500;
		cursor: pointer;
		border: 1px solid var(--color-accent);
		background-color: var(--color-accent);
		color: #fff;
		transition: background-color 0.15s, border-color 0.15s;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn:hover:not(:disabled) {
		background-color: var(--color-accent-hover);
		border-color: var(--color-accent-hover);
	}
</style>
