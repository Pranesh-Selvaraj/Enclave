<script lang="ts">
	// Minimal inline SVG icon set (stroke-based, currentColor). No icon dep.
	let {
		name,
		size = 16,
	}: {
		name: string;
		size?: number;
	} = $props();

	const paths: Record<string, string> = {
		home: 'M3 10.5 12 3l9 7.5M5 9.5V21h5v-6h4v6h5V9.5',
		graph: 'M6 6h4v4H6zM14 14h4v4h-4zM8 8l6 4M16 16l-2-4',
		plus: 'M12 5v14M5 12h14',
		star: 'M12 3l2.7 5.6 6.1.8-4.5 4.2 1.1 6-5.4-2.9L6.6 19.6l1.1-6L3.2 9.4l6.1-.8z',
		trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6',
		duplicate: 'M9 9h11v11H9zM5 15H4V4h11v1',
		gear: 'M12 8.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7zM12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M18.7 5.3l-2.1 2.1M7.4 16.6l-2.1 2.1',
		sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10zM12 1v3M12 20v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M1 12h3M20 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1',
		moon: 'M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z',
		network: 'M4 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM20 14a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM12 5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM5 8h3M16 16h3M8.5 8.5 15 14.5M8.5 7l7-2',
		lock: 'M6 11h12v9H6zM8 11V7a4 4 0 0 1 8 0v4',
		search: 'M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM21 21l-4.3-4.3',
		more: 'M5 12a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM12 12a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM19 12a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z',
		chevronLeft: 'M15 5l-7 7 7 7',
		chevronRight: 'M9 5l7 7-7 7',
		page: 'M7 3h7l4 4v14H7zM14 3v4h4',
		link: 'M10 14a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1.5 1.5M14 10a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1.5-1.5',
		download: 'M12 3v12M7 10l5 5 5-5M4 19h16',
		upload: 'M12 21V9M7 14l5-5 5 5M4 5h16',
		x: 'M6 6l12 12M18 6L6 18',
		palette: 'M12 3a9 9 0 0 0 0 18h1.5a2 2 0 0 0 0-4H12a2 2 0 0 1-2-2 2 2 0 0 1 2-2h3a2 2 0 0 0 2-2 5 5 0 0 0-5-5zM7 11a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM16 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM9 6a1 1 0 1 0 0 2 1 1 0 0 0 0-2z',
		check: 'M5 13l4 4L19 7',
		text: 'M4 6h16M12 6v14',
		table: 'M4 4h16v16H4zM4 10h16M10 4v16',
		expand: 'M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5',
		bookmark: 'M6 3h12v18l-6-4-6 4z',
		chevronDown: 'M5 9l7 7 7-7',
		print: 'M6 8V3h12v5M6 15H3V8h18v7h-3M6 21h12v-6H6z',
		smile: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM8.5 10a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM15.5 10a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM8.8 14.5a5 5 0 0 0 6.4 0',
		info: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8a1 1 0 1 0 0 2 1 1 0 0 0 0-2z',
	};
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.8"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	{#if paths[name]}
		<path d={paths[name]} />
	{/if}
</svg>
