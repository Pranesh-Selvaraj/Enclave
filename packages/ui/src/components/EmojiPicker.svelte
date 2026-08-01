<script lang="ts">
	// Curated emoji grid for page icons — no emoji-picker dep.
	// ponytail: 36 hand-picked emoji, no categories/search. Swap in a full
	// emoji data set (e.g. emoji-mart) when the curated set feels limiting.
	const EMOJIS = [
		'📄', '📝', '💡', '🔖', '📌', '⭐', '🔥', '✅', '🚀', '🧠',
		'📚', '📊', '📅', '🎯', '🧩', '🔬', '💼', '🎨', '🎧', '🌱',
		'🏠', '❤️', '⚡', '🌙', '☀️', '🌈', '🍀', '🎉', '🔒', '🔑',
		'🗺️', '🛠️', '📦', '🪜', '🗃️', '🧰',
	];

	let {
		value,
		onPick,
	}: {
		value: string;
		onPick: (emoji: string) => void;
	} = $props();
</script>

<div class="emoji-grid" role="listbox" aria-label="Pick an emoji">
	<button
		class="emoji-cell"
		class:selected={value === ''}
		onclick={() => onPick('')}
		title="No icon"
		role="option"
		aria-selected={value === ''}
	>
		—
	</button>
	{#each EMOJIS as e}
		<button
			class="emoji-cell"
			class:selected={value === e}
			onclick={() => onPick(e)}
			title={e}
			role="option"
			aria-selected={value === e}
		>
			{e}
		</button>
	{/each}
</div>

<style>
	.emoji-grid {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 2px;
		width: 216px;
	}

	.emoji-cell {
		border: none;
		background: none;
		font-size: 17px;
		line-height: 1;
		padding: 6px 0;
		border-radius: 6px;
		cursor: pointer;
		text-align: center;
		color: var(--color-text-muted);
	}

	.emoji-cell:hover {
		background: var(--color-surface-hover);
	}

	.emoji-cell.selected {
		background: var(--color-accent-subtle);
		outline: 1px solid var(--color-accent);
	}
</style>
