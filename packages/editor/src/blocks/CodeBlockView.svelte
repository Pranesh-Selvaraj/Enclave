<script lang="ts">
	import { Icon } from '@enclave/ui';

	const LANGUAGES = ['plaintext', 'js', 'ts', 'py', 'bash', 'json', 'sql', 'md', 'html', 'css', 'rust', 'go', 'java', 'c', 'cpp', 'yaml', 'toml'];

	let {
		language,
		onLanguageChange,
	}: {
		language: string;
		onLanguageChange: (lang: string) => void;
	} = $props();

	// Local copy so external updates (via setLanguage) can drive the select.
	// The warning is expected: the prop is captured once at mount on purpose.
	// svelte-ignore state_referenced_locally
	let lang = $state(language);

	// Svelte 5 has no $set — node views push updates through exports.
	export function setLanguage(next: string) {
		lang = next;
	}

	let root: HTMLDivElement | undefined = $state();
	let copied = $state(false);

	async function copy() {
		// The node view mounts this toolbar next to the code <pre> (contentDOM),
		// so read the text from the shared .cb-node wrapper (issue #162).
		const text = root?.closest('.cb-node')?.querySelector('.cb-pre')?.textContent ?? '';
		try {
			await navigator.clipboard.writeText(text);
			copied = true;
			setTimeout(() => (copied = false), 1200);
		} catch {
			// clipboard unavailable (webkitgtk) — silently skip
		}
	}
</script>

<div class="cb-bar" bind:this={root}>
	<div class="cb-bar-left">
		<Icon name="code" size={13} />
		<select
			class="cb-lang"
			value={lang || 'plaintext'}
			onchange={(e: Event) => onLanguageChange((e.currentTarget as HTMLSelectElement).value)}
			aria-label="Code language"
		>
			{#each LANGUAGES as l}
				<option value={l}>{l}</option>
			{/each}
		</select>
	</div>
	<button class="cb-copy" onclick={copy} title="Copy code">
		<Icon name={copied ? 'check' : 'copy'} size={13} />
		{copied ? 'Copied' : 'Copy'}
	</button>
</div>

<style>
	/* The node view builds .cb-node (wrapper) + .cb-pre (ProseMirror content)
	   itself, so style them globally here. */
	:global(.cb-node) {
		margin: 0.5em 0;
		border: 1px solid var(--color-border);
		border-radius: 8px;
		overflow: hidden;
		background: var(--color-surface);
	}
	:global(.cb-node .cb-pre) {
		margin: 0;
		border: none;
		border-radius: 0;
		padding: 10px 12px;
		overflow-x: auto;
		/* Long code lines scroll instead of wrapping (issue #164). */
		white-space: pre;
		overflow-wrap: normal;
		word-break: normal;
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
	.cb-bar-left {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--color-text-faint);
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
		display: flex;
		align-items: center;
		gap: 5px;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		background: none;
		color: var(--color-text-muted);
		font-size: 11px;
		cursor: pointer;
		padding: 3px 8px;
		font-family: inherit;
	}
	.cb-copy:hover {
		background: var(--color-surface-active);
		color: var(--color-text);
	}
</style>
