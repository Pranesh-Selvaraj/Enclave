<script lang="ts">
	// Enclave brand mark — a keyhole on a 24-unit grid.
	//
	// Geometry is the same as docs/brand/logo.svg: head radius 7.2 at (12, 9), the pin
	// punched through it at radius 2.5, and a stem tapering 7.0 → 5.2. Keep the two in step.
	//
	// The tile follows the theme rather than being fixed to the brand ink, because the brand
	// ink is the dark theme's own background and an ink tile would vanish into it. So the light
	// theme gets an ink tile with a paper keyhole and the dark theme gets the inverse — the same
	// rule the light/dark lockups follow.
	let {
		size = 28,
	}: {
		size?: number;
	} = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	role="img"
	aria-label="Enclave"
>
	<rect width="24" height="24" rx="5.4" fill="var(--color-text, #16130f)" />
	<g fill="var(--color-bg, #ffffff)">
		<circle cx="12" cy="9" r="7.2" />
		<path d="M8.5 13.6L15.5 13.6L14.6 20Q14.6 21.6 13 21.6L11 21.6Q9.4 21.6 9.4 20Z" />
	</g>
	<!-- the pin: filled with the tile colour, which reads as a hole at any size -->
	<circle cx="12" cy="9" r="2.5" fill="var(--color-text, #16130f)" />
</svg>
