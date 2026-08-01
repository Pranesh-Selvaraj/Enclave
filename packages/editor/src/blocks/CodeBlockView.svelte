<script lang="ts">
	const LANGUAGES = ['plaintext', 'js', 'ts', 'py', 'bash', 'json', 'sql', 'md', 'html', 'css'];

	let {
		language,
		contentDOM,
		onLanguageChange,
	}: {
		language: string;
		contentDOM: HTMLElement;
		onLanguageChange: (lang: string) => void;
	} = $props();

	let host: HTMLDivElement | undefined = $state();
	let copied = $state(false);

	$effect(() => {
		if (host && contentDOM && !contentDOM.isConnected) host.appendChild(contentDOM);
	});

	async function copy() {
		try {
			await navigator.clipboard.writeText(contentDOM.textContent ?? '');
			copied = true;
			setTimeout(() => (copied = false), 1200);
		} catch {
			// clipboard unavailable (webkitgtk) — silently skip
		}
	}
</script>

<div class="cb-wrap">
	<div class="cb-bar">
		<select
			class="cb-lang"
			value={language || 'plaintext'}
			onchange={(e: Event) => onLanguageChange((e.currentTarget as HTMLSelectElement).value)}
			aria-label="Code language"
		>
			{#each LANGUAGES as l}
				<option value={l}>{l}</option>
			{/each}
		</select>
		<button class="cb-copy" onclick={copy} title="Copy code">
			{copied ? '✓' : 'Copy'}
		</button>
	</div>
	<div bind:this={host}></div>
</div>

<style>
	.cb-wrap {
		margin: 0.5em 0;
		border: 1px solid var(--color-border);
		border-radius: 8px;
		overflow: hidden;
		background: var(--color-surface);
	}

	.cb-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 6px 10px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface-hover);
	}

	.cb-lang {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 11px;
		font-family: var(--font-mono);
		text-transform: uppercase;
		outline: none;
		cursor: pointer;
	}

	.cb-copy {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 11px;
		cursor: pointer;
		padding: 2px 6px;
		border-radius: 4px;
		font-family: inherit;
	}

	.cb-copy:hover {
		background: var(--color-surface-active);
		color: var(--color-text);
	}

	:global(.cb-wrap pre) {
		margin: 0;
		border: none;
		border-radius: 0;
	}
</style>
