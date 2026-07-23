<script lang="ts">
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import type { Document } from '@enclave/ui';

	let documents = $state<Document[]>([]);
	let links = $state<Array<{ source: string; target: string }>>([]);
	let canvasEl = $state<HTMLCanvasElement | undefined>();
	let loading = $state(true);

	async function loadGraph() {
		try {
			documents = await invoke<Document[]>('get_document_list');

			const pageList = await invoke<Array<{ id: string; title: string }>>('get_page_list');
			const titleToId = new Map(pageList.map(p => [p.title, p.id]));
			const allLinks: Array<{ source: string; target: string }> = [];

			for (const doc of documents) {
				const blocks = await invoke<Array<{ content: Record<string, unknown> }>>('get_blocks', { documentId: doc.id });
				for (const block of blocks) {
					const text = JSON.stringify(block.content);
					const matches = text.matchAll(/\[\[([^\]]+)\]\]/g);
					for (const match of matches) {
						const targetId = titleToId.get(match[1]);
						if (targetId && targetId !== doc.id) {
							allLinks.push({ source: doc.id, target: targetId });
						}
					}
				}
			}
			links = allLinks;
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

			ctx!.strokeStyle = '#444';
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
				ctx!.fillStyle = '#6b5ce7';
				ctx!.beginPath();
				ctx!.arc(n.x, n.y, 6, 0, Math.PI * 2);
				ctx!.fill();

				ctx!.fillStyle = '#e8e8e8';
				ctx!.font = '11px Inter, sans-serif';
				ctx!.fillText(n.title.slice(0, 20), n.x + 10, n.y + 4);
			}

			if (totalEnergy > 0.5) {
				requestAnimationFrame(simulate);
			}
		}

		canvasEl.onclick = (e: MouseEvent) => {
			const rect = canvasEl!.getBoundingClientRect();
			const mx = e.clientX - rect.left;
			const my = e.clientY - rect.top;
			for (const n of nodes) {
				const dx = mx - n.x;
				const dy = my - n.y;
				if (dx * dx + dy * dy < 100) {
					goto(`/${n.id}`);
					return;
				}
			}
		};

		requestAnimationFrame(simulate);
	}

	$effect(() => {
		loadGraph();
	});
	$effect(() => {
		if (!loading && canvasEl) render();
	});
</script>

<div class="graph-page">
	<div class="graph-header">
		<h1>Graph View</h1>
		<p class="graph-subtitle">
			{documents.length} pages, {links.length} connections
			{#if links.length === 0}
				— Link pages with <code>[[Page Title]]</code> to see connections
			{/if}
		</p>
		<a href="/" class="back-link">← Back to pages</a>
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
		padding: 24px 32px;
	}

	.graph-header {
		margin-bottom: 16px;
		flex-shrink: 0;
	}

	.graph-header h1 {
		font-size: 22px;
		font-weight: 700;
		margin: 0 0 4px;
	}

	.graph-subtitle {
		font-size: 13px;
		color: var(--color-text-muted);
		margin: 0 0 8px;
	}
	.graph-subtitle code {
		background: var(--color-surface);
		padding: 1px 5px;
		border-radius: 3px;
		font-family: var(--font-mono);
		font-size: 12px;
	}

	.back-link {
		font-size: 13px;
		color: var(--color-accent);
		text-decoration: none;
	}
	.back-link:hover { text-decoration: underline; }

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
</style>
