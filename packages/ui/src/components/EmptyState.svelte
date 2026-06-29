<script lang="ts">
	let { icon = '📝', title, description, children }: {
		icon?: string;
		title: string;
		description: string;
		children?: any;
	} = $props();
</script>

<div class="empty-state">
	<div class="empty-icon">{icon}</div>
	<h3>{title}</h3>
	<p>{description}</p>
	{#if children}
		{@render children()}
	{/if}
</div>

<style>
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: 80px 40px;
		border: 2px dashed var(--color-border);
		border-radius: 12px;
	}

	.empty-icon {
		font-size: 48px;
		margin-bottom: 16px;
	}

	.empty-state h3 {
		font-size: 18px;
		font-weight: 600;
		margin: 0 0 8px;
	}

	.empty-state p {
		max-width: 420px;
		color: var(--color-text-muted);
		font-size: 14px;
		line-height: 1.6;
		margin: 0 0 24px;
	}
</style>
