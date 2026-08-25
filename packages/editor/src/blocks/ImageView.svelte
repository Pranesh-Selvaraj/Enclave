<script lang="ts">
	// Image block node view: the image plus an editable caption underneath
	// (same pattern as BookmarkView — caption is a plain attribute, no nested
	// editable node).
	let {
		src,
		alt,
		caption,
		onCaptionChange,
	}: {
		src: string;
		alt: string;
		caption: string;
		onCaptionChange: (caption: string) => void;
	} = $props();
</script>

<figure class="img-view">
	<img src={src} alt={alt} draggable="false" />
	<figcaption>
		<input
			class="img-caption"
			value={caption}
			placeholder="Add a caption…"
			aria-label="Image caption"
			oninput={(e: Event) => onCaptionChange((e.currentTarget as HTMLInputElement).value)}
		/>
	</figcaption>
</figure>

<style>
	.img-view {
		margin: 0.75em 0;
		text-align: center;
	}

	.img-view img {
		max-width: 100%;
		border-radius: 10px;
		display: block;
		margin: 0 auto;
	}

	.img-caption {
		width: 100%;
		box-sizing: border-box;
		border: none;
		background: none;
		outline: none;
		text-align: center;
		color: var(--color-text-muted);
		font-size: 12.5px;
		font-family: inherit;
		padding: 6px 8px;
		border-radius: 6px;
	}

	.img-caption:hover {
		background: var(--color-surface-hover);
	}

	.img-caption::placeholder {
		color: var(--color-text-faint);
	}
</style>
