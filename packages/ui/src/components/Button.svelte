<script lang="ts">
	let { children, onclick, variant = 'primary', type = 'button' }: {
		children: any;
		onclick?: () => void;
		variant?: 'primary' | 'secondary' | 'ghost';
		type?: 'button' | 'submit';
	} = $props();
</script>

<button {type} class="btn btn-{variant}" {onclick}>
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
		border: 1px solid transparent;
		transition: background-color 0.15s, border-color 0.15s, color 0.15s;
	}

	.btn-primary {
		background-color: var(--color-accent);
		color: #fff;
		border-color: var(--color-accent);
	}

	.btn-primary:hover {
		background-color: var(--color-accent-hover);
		border-color: var(--color-accent-hover);
	}

	.btn-secondary {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-text);
		border-color: var(--color-border);
	}

	.btn-secondary:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}

	.btn-ghost {
		background: none;
		color: var(--color-text-muted);
	}

	.btn-ghost:hover {
		color: var(--color-text);
		background-color: rgba(255, 255, 255, 0.05);
	}
</style>
