<script lang="ts">
	// Enclave brand mark — "the keyhole vault": a violet→indigo gradient tile
	// with a white keyhole cut out. Reads as locked/private at any size.
	// Matches the app-icon design in scripts/generate-icons.py.
	let {
		size = 28,
	}: {
		size?: number;
	} = $props();

	// Unique gradient id per instance (the same SVG can appear many times).
	const gid = `lg${Math.random().toString(36).slice(2, 8)}`;
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 48 48"
	fill="none"
	role="img"
	aria-label="Enclave"
>
	<defs>
		<linearGradient id={gid} x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="#8a7bff" />
			<stop offset="1" stop-color="#4f46e5" />
		</linearGradient>
	</defs>
	<rect x="1" y="1" width="46" height="46" rx="12" fill={`url(#${gid})`} />
	<path
		d="M24 8a11 11 0 0 0-7.4 19.3V36a3 3 0 0 0 3 3h8.8a3 3 0 0 0 3-3v-8.7A11 11 0 0 0 24 8z"
		fill="#ffffff"
	/>
	<circle cx="24" cy="17.5" r="4.2" fill="none" stroke={`url(#${gid})`} stroke-width="2.4" />
</svg>
