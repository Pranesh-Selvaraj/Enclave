<script lang="ts">
	// Enclave icon set — one consistent stroke style (24px grid, 1.75 stroke,
	// round caps/joins, Lucide-inspired geometry). No icon dependency.
	let {
		name,
		size = 16,
		filled = false,
	}: {
		name: string;
		size?: number;
		filled?: boolean;
	} = $props();

	const paths: Record<string, string> = {
		// ── Navigation & chrome ──
		home: 'M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM9 22V12h6v10',
		graph: 'M6 6h4v4H6zM14 14h4v4h-4zM8 8l6 4M16 16l-2-4',
		search: 'M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM21 21l-4.3-4.3',
		menu: 'M4 6h16M4 12h16M4 18h16',
		more: 'M5 12a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM12 12a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM19 12a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z',
		chevronLeft: 'M15 5l-7 7 7 7',
		chevronRight: 'M9 5l7 7-7 7',
		chevronDown: 'M5 9l7 7 7-7',
		chevronUp: 'M5 15l7-7 7 7',
		arrowLeft: 'M19 12H5M11 6l-6 6 6 6',
		arrowRight: 'M5 12h14M13 6l6 6-6 6',
		x: 'M6 6l12 12M18 6L6 18',
		check: 'M5 13l4 4L19 7',
		checkCircle: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM8.5 12.5l2.5 2.5 4.5-5',
		alertTriangle: 'M12 3 2.5 20h19zM12 9.5v4.5M12 17.5h.01',
		info: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8a1 1 0 1 0 0 2 1 1 0 0 0 0-2z',
		expand: 'M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5',
		print: 'M6 8V3h12v5M6 15H3V8h18v7h-3M6 21h12v-6H6z',
		layout: 'M4 4h16v16H4zM9 4v16',
		grid: 'M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z',
		externalLink: 'M14 4h6v6M20 4 11 13M18 13v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h5',
		logOut: 'M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9',

		// ── Actions ──
		plus: 'M5 12h14M12 5v14',
		edit: 'M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z',
		trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6',
		duplicate: 'M9 9h11v11H9zM5 15H4V4h11v1',
		copy: 'M8 8h12v12H8zM4 16V4h12',
		star: 'M12 3.5l2.6 5.3 5.8.8-4.2 4.1 1 5.8-5.2-2.7-5.2 2.7 1-5.8L3.6 9.6l5.8-.8z',
		refresh: 'M21 12a9 9 0 1 1-9-9c2.5 0 4.8 1 6.4 2.6L21 8M21 3v5h-5',
		download: 'M12 3v12M7 10l5 5 5-5M4 19h16',
		upload: 'M12 21V9M7 14l5-5 5 5M4 5h16',
		send: 'M22 2 11 13M22 2l-7 20-4-9-9-4z',
		cut: 'M6 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM6 16a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM4 6l16 12M20 6 4 18',
		filter: 'M22 3H2l8 9.5V19l4 2v-8.5z',

		// ── Content types ──
		page: 'M7 3h7l4 4v14H7zM14 3v4h4',
		folder: 'M3 6h6l2 2h10v11H3z',
		text: 'M4 6h16M12 6v14',
		table: 'M4 4h16v16H4zM4 10h16M10 4v16',
		database: 'M12 4c4.4 0 8 1.3 8 3s-3.6 3-8 3-8-1.3-8-3 3.6-3 8-3zM4 7v10c0 1.7 3.6 3 8 3s8-1.3 8-3V7M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3',
		bookmark: 'M6 3h12v18l-6-4-6 4z',
		tag: 'M12.6 2.6A2 2 0 0 0 11.2 2H4a2 2 0 0 0-2 2v7.2a2 2 0 0 0 .6 1.4l8.7 8.7a2.4 2.4 0 0 0 3.4 0l6.6-6.6a2.4 2.4 0 0 0 0-3.4zM7.5 7.5h.01',
		calendar: 'M4 4h16v16H4zM4 9h16M8 3v4M16 3v4',
		clock: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2',
		link: 'M10 14a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1.5 1.5M14 10a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1.5-1.5',
		image: 'M4 4h16v16H4zM4 15.5l4-4 4 4 3.5-3.5 4.5 4.5M14.5 8.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z',
		zap: 'M13 2 4 14h6l-1 8 9-12h-6z',
		smile: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM8.5 10a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM15.5 10a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM8.8 14.5a5 5 0 0 0 6.4 0',
		sparkles: 'M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9zM19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z',
		messageCircle: 'M7.9 20A9 9 0 1 0 4 16.1L2 22z',
		palette: 'M12 3a9 9 0 0 0 0 18h1.5a2 2 0 0 0 0-4H12a2 2 0 0 1-2-2 2 2 0 0 1 2-2h3a2 2 0 0 0 2-2 5 5 0 0 0-5-5zM7 11a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM16 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2zM9 6a1 1 0 1 0 0 2 1 1 0 0 0 0-2z',

		// ── Formatting ──
		bold: 'M7 5h7a4 4 0 0 1 0 8H7zM7 13h8a4 4 0 0 1 0 8H7z',
		italic: 'M19 4h-9M14 20H5M15 4 9 20',
		strike: 'M16 4H9a3 3 0 0 0-2.83 4M14 12a4 4 0 0 1 0 8H6M4 12h16',
		code: 'm16 18 6-6-6-6M8 6l-6 6 6 6',
		codeBlock: 'M4 4h16v16H4zM10 9l-3 3 3 3M14 15l3-3-3-3',
		list: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',
		listOrdered: 'M10 6h11M10 12h11M10 18h11M4 10h2M4 6h1v4M6 18H4c0-1 2-2 2-3s-1-1.5-2-1',
		listChecks: 'M3 17.5 5.5 20 9 16.5M3 6.5 5.5 9 9 5.5M13 7h8M13 12h8M13 17h8',
		quote: 'M7 4a2 2 0 0 0-2 2v5a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 5 5 0 0 0 5-5V6a2 2 0 0 0-2-2zM19 4a2 2 0 0 0-2 2v5a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 5 5 0 0 0 5-5V6a2 2 0 0 0-2-2z',
		heading1: 'M4 6v12M10 6v12M4 12h6M17 8.5V18M17 8.5l-2.5 1.7',
		heading2: 'M4 6v12M10 6v12M4 12h6M16 18.5h4.5L17 15.2c1.3 0 2.5-1 2.5-2.3 0-1.6-1.3-2.4-2.5-2.4-1 0-1.8.5-2.2 1.2',
		heading3: 'M4 6v12M10 6v12M4 12h6M17 9.3c1.6-1 3.2.2 3.2 1.7a2 2 0 0 1-2 2c1.2 0 2.3.8 2.3 2.1 0 1.5-1.6 2.7-3.2 1.7-.7-.5-1.1-1.2-1.2-2',
		divider: 'M4 12h16',
		toggle: 'M4 10.5h11a2.5 2.5 0 0 1 0 5H4zM17 12.5h3',
		callout: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 8v4M12 15.5h.01',

		// ── Status & security ──
		lock: 'M6 11h12v9H6zM8 11V7a4 4 0 0 1 8 0v4',
		unlock: 'M6 11h12v9H6zM8 11V7a5 5 0 0 1 9.9-1',
		keyhole: 'M12 2a6 6 0 0 0-4 10.5V21h8v-8.5A6 6 0 0 0 12 2zM12 7a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
		shield: 'M12 2.5 20 6v6c0 4.8-3.4 8.3-8 9.5-4.6-1.2-8-4.7-8-9.5V6z',
		eye: 'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
		eyeOff: 'M3 3l18 18M10.6 10.6a3 3 0 0 0 4.2 4.2M9.9 5.1A10.4 10.4 0 0 1 12 5c6.5 0 10 7 10 7a17.6 17.6 0 0 1-2.2 3.2M6.6 6.6A17.6 17.6 0 0 0 2 12s3.5 7 10 7a10.4 10.4 0 0 0 3.3-.5',
		settings: 'M14 4h7M10 4H3M12 12h9M8 12H3M16 20h5M12 20H3M14 2v4M8 10v4M16 18v4',
		sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10zM12 1v3M12 20v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M1 12h3M20 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1',
		moon: 'M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z',
		network: 'M4 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM20 14a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM12 5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM5 8h3M16 16h3M8.5 8.5 14.5 14.5M8.5 7l7-2',
	};
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill={filled ? 'currentColor' : 'none'}
	stroke="currentColor"
	stroke-width="1.75"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	{#if paths[name]}
		<path d={paths[name]} />
	{/if}
</svg>
