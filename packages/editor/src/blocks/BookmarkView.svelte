<script lang="ts">
	import { Icon } from '@enclave/ui';

	let {
		url,
		title,
		onTitleChange,
	}: {
		url: string;
		title: string;
		onTitleChange: (title: string) => void;
	} = $props();

	let domain = $derived.by(() => {
		try {
			const u = new URL(url);
			return u.hostname.replace(/^www\./, '');
		} catch {
			return url;
		}
	});

	let initial = $derived((domain[0] ?? '?').toUpperCase());
</script>

<div class="bm-card">
	<span class="bm-avatar"><Icon name="link" size={16} /></span>
	<div class="bm-body">
		<input
			class="bm-title"
			value={title}
			placeholder="Untitled bookmark"
			oninput={(e: Event) => onTitleChange((e.currentTarget as HTMLInputElement).value)}
			aria-label="Bookmark title"
		/>
		<span class="bm-url"><Icon name="externalLink" size={11} />{domain}</span>
	</div>
	<span class="bm-ghost" title={initial}>{initial}</span>
</div>

<style>
	.bm-card {
		display: flex;
		align-items: center;
		gap: 12px;
		border: 1px solid var(--color-border);
		border-radius: 12px;
		background: var(--color-surface);
		padding: 10px 14px;
		margin: 8px 0;
		width: fit-content;
		min-width: 320px;
		max-width: 100%;
		transition: border-color 0.15s, box-shadow 0.15s;
	}
	.bm-card:hover {
		border-color: var(--color-border-strong);
		box-shadow: var(--shadow-sm);
	}

	.bm-avatar {
		width: 36px;
		height: 36px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 10px;
		background: linear-gradient(135deg, #8a7bff, #4f46e5);
		color: #fff;
	}

	.bm-body {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.bm-title {
		border: none;
		background: none;
		outline: none;
		color: var(--color-text);
		font-size: 14px;
		font-weight: 500;
		font-family: inherit;
		padding: 0;
	}

	.bm-title::placeholder {
		color: var(--color-text-faint);
	}

	.bm-url {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 12px;
		color: var(--color-text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.bm-url :global(svg) { flex-shrink: 0; color: var(--color-text-faint); }

	.bm-ghost {
		width: 24px;
		height: 24px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background: var(--color-surface-hover);
		color: var(--color-text-faint);
		font-size: 11px;
		font-weight: 700;
	}
</style>
