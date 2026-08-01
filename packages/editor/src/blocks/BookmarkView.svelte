<script lang="ts">
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
	<span class="bm-avatar">{initial}</span>
	<div class="bm-body">
		<input
			class="bm-title"
			value={title}
			placeholder="Untitled bookmark"
			oninput={(e: Event) => onTitleChange((e.currentTarget as HTMLInputElement).value)}
			aria-label="Bookmark title"
		/>
		<span class="bm-url">{domain}</span>
	</div>
</div>

<style>
	.bm-card {
		display: flex;
		align-items: center;
		gap: 12px;
		border: 1px solid var(--color-border);
		border-radius: 8px;
		background: var(--color-hover);
		padding: 10px 14px;
		margin: 8px 0;
		width: fit-content;
		min-width: 320px;
		max-width: 100%;
	}

	.bm-avatar {
		width: 36px;
		height: 36px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 8px;
		background: var(--color-accent-subtle);
		color: var(--color-accent);
		font-weight: 700;
		font-size: 15px;
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
		font-size: 12px;
		color: var(--color-text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
