<script lang="ts">
	import { goto } from '$app/navigation';
	import { invoke } from '$lib/backend.js';
	import type { Document } from '@enclave/ui';
	import { theme } from '@enclave/ui';
	import { extractLinks } from '$lib/graphLinks.js';

	let documents = $state<Document[]>([]);
	let links = $state<Array<{ source: string; target: string }>>([]);
	let canvasEl = $state<HTMLCanvasElement | undefined>();
	let loading = $state(true);
	// Simulation lifecycle: one rAF loop per render pass, cancelled on resize,
	// theme change and unmount — overlapped loops burned CPU and kept drawing
	// after the page was gone.
	let rafId = 0;
	let canvasObserver: ResizeObserver | undefined;

	async function loadGraph() {
		try {
			documents = await invoke<Document[]>('get_document_list');

			const pageList = await invoke<Array<{ id: string; title: string }>>('get_page_list');
			const titleToId = new Map(pageList.map(p => [p.title, p.id]));
			const perDoc = await Promise.all(documents.map(async (doc) => {
				const blocks = await invoke<Array<{ content: Record<string, unknown> }>>('get_blocks', { documentId: doc.id });
				return blocks.flatMap((b) => extractLinks(b.content, titleToId, doc.id));
			}));
			links = perDoc.flat();
		} catch (e) {
			console.error('Failed to load graph:', e);
		} finally {
			loading = false;
		}
	}

	function render() {
		if (!canvasEl || documents.length === 0) return;
		const ctx = canvasEl.getContext('2d');
		if (!ctx) return;
		cancelAnimationFrame(rafId);

		// Read matte tokens live so the canvas tracks theme + accent changes.
		const cssVar = (n: string, fb: string) =>
			getComputedStyle(document.documentElement).getPropertyValue(n).trim() || fb;
		const edgeColor = cssVar('--color-border-strong', '#3f3830');
		const nodeColor = cssVar('--color-accent', '#8b7cf6');
		const labelColor = cssVar('--color-text-muted', '#a39b90');

		const w = canvasEl.width = canvasEl.clientWidth * devicePixelRatio;
		const h = canvasEl.height = canvasEl.clientHeight * devicePixelRatio;
		ctx.scale(devicePixelRatio, devicePixelRatio);

		const width = canvasEl.clientWidth;
		const height = canvasEl.clientHeight;

		interface GraphNode {
			id: string;
			title: string;
			x: number;
			y: number;
			vx: number;
			vy: number;
		}

		const nodes: GraphNode[] = documents.map(() => ({
			id: '',
			title: '',
			x: width / 2 + (Math.random() - 0.5) * 200,
			y: height / 2 + (Math.random() - 0.5) * 200,
			vx: 0,
			vy: 0,
		}));

		for (let i = 0; i < documents.length; i++) {
			nodes[i].id = documents[i].id;
			nodes[i].title = documents[i].title || 'Untitled';
		}

		const nodeMap = new Map(nodes.map(n => [n.id, n]));

		function simulate() {
			ctx!.clearRect(0, 0, width, height);

			const kRepel = 5000;
			const damping = 0.85;

			for (const n of nodes) {
				for (const m of nodes) {
					if (n === m) continue;
					const dx = n.x - m.x;
					const dy = n.y - m.y;
					const dist = Math.max(1, Math.sqrt(dx * dx + dy * dy));
					const force = kRepel / (dist * dist);
					n.vx += (dx / dist) * force;
					n.vy += (dy / dist) * force;
				}
				n.vx += (width / 2 - n.x) * 0.005;
				n.vy += (height / 2 - n.y) * 0.005;
			}

			for (const edge of links) {
				const s = nodeMap.get(edge.source);
				const t = nodeMap.get(edge.target);
				if (!s || !t) continue;
				const dx = t.x - s.x;
				const dy = t.y - s.y;
				const dist = Math.max(1, Math.sqrt(dx * dx + dy * dy));
				const force = (dist - 100) * 0.01;
				const fx = (dx / dist) * force;
				const fy = (dy / dist) * force;
				s.vx += fx;
				s.vy += fy;
				t.vx -= fx;
				t.vy -= fy;
			}

			let totalEnergy = 0;
			for (const n of nodes) {
				n.vx *= damping;
				n.vy *= damping;
				n.x += n.vx;
				n.y += n.vy;
				totalEnergy += Math.abs(n.vx) + Math.abs(n.vy);
			}

			ctx!.strokeStyle = edgeColor;
			ctx!.lineWidth = 1;
			for (const edge of links) {
				const s = nodeMap.get(edge.source);
				const t = nodeMap.get(edge.target);
				if (!s || !t) continue;
				ctx!.beginPath();
				ctx!.moveTo(s.x, s.y);
				ctx!.lineTo(t.x, t.y);
				ctx!.stroke();
			}

			for (const n of nodes) {
				ctx!.fillStyle = nodeColor;
				ctx!.beginPath();
				ctx!.arc(n.x, n.y, 6, 0, Math.PI * 2);
				ctx!.fill();

				ctx!.fillStyle = labelColor;
				ctx!.font = '11px Inter, sans-serif';
				ctx!.fillText(n.title.slice(0, 20), n.x + 10, n.y + 4);
			}

			if (totalEnergy > 0.5) {
				rafId = requestAnimationFrame(simulate);
			}
		}

		canvasEl.onclick = (e: MouseEvent) => {
			const rect = canvasEl!.getBoundingClientRect();
			const mx = e.clientX - rect.left;
			const my = e.clientY - rect.top;
			// Fingers need a much bigger hit target than a mouse pointer.
			const hitRadius = matchMedia('(pointer: coarse)').matches ? 24 : 12;
			for (const n of nodes) {
				const dx = mx - n.x;
				const dy = my - n.y;
				if (dx * dx + dy * dy < hitRadius * hitRadius) {
					goto(`/${n.id}`);
					return;
				}
			}
		};

		rafId = requestAnimationFrame(simulate);
	}

	$effect(() => {
		loadGraph();
	});
	// Re-render when the graph is ready, the theme changes (canvas colors are
	// read from live CSS tokens) or the canvas is resized.
	$effect(() => {
		const _t = theme.value;
		if (loading || !canvasEl) return;
		// Fit node positions to the bitmap whenever the pane changes size —
		// without this the canvas bitmap stays stretched on window resize.
		canvasObserver = new ResizeObserver(() => render());
		canvasObserver.observe(canvasEl);
		render();
		return () => {
			cancelAnimationFrame(rafId);
			canvasObserver?.disconnect();
			canvasObserver = undefined;
		};
	});
</script>

<div class="graph-page">
	<div class="graph-header">
		<div class="graph-heading">
			<h1>Graph</h1>
			<span class="graph-subtitle">
				{documents.length} pages · {links.length} {links.length === 1 ? 'connection' : 'connections'}
			</span>
		</div>
		{#if links.length === 0 && documents.length > 0}
			<p class="graph-hint">Link pages with <code>[[Page Title]]</code> to connect them.</p>
		{/if}
	</div>

	{#if loading}
		<div class="loading">Loading graph…</div>
	{:else}
		<div class="graph-canvas-wrap">
			<canvas bind:this={canvasEl}></canvas>
			{#if documents.length === 0}
				<div class="graph-empty">
					<p>No pages yet. Create pages and link them to see the graph.</p>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.graph-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		padding: 24px 28px;
	}

	.graph-header {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin-bottom: 14px;
		flex-shrink: 0;
	}

	.graph-heading {
		display: flex;
		align-items: baseline;
		gap: 10px;
		flex-wrap: wrap;
	}

	.graph-header h1 {
		font-size: 20px;
		font-weight: 700;
		letter-spacing: -0.02em;
		margin: 0;
	}

	.graph-subtitle {
		font-size: 13px;
		color: var(--color-text-muted);
	}
	.graph-hint {
		font-size: 12px;
		color: var(--color-text-faint);
		margin: 0;
	}
	.graph-hint code {
		background: var(--color-surface);
		padding: 1px 5px;
		border-radius: 3px;
		font-family: var(--font-mono);
		font-size: 12px;
	}

	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1;
		color: var(--color-text-muted);
	}

	.graph-canvas-wrap {
		flex: 1;
		position: relative;
		border: 1px solid var(--color-border);
		border-radius: 12px;
		overflow: hidden;
		background: var(--color-surface);
	}

	canvas {
		width: 100%;
		height: 100%;
		display: block;
	}

	.graph-empty {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		font-size: 14px;
	}

	@media (max-width: 768px) {
		.graph-page { padding: 16px 12px; }
	}
</style>
