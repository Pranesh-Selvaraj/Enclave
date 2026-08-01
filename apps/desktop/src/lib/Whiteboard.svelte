<script lang="ts">
	// Enclave Whiteboard — infinite canvas ("edgeless" mode).
	// MVP: pan/zoom, sticky notes, shapes, arrows, text, PNG export.
	// ponytail: no connectors, no resize handles, no undo — add when real use
	// demands it. Persistence: one 'whiteboard' block per doc (no schema change).
	import { invoke } from '@tauri-apps/api/core';
	import Icon from '$lib/Icon.svelte';

	let { docId, title = 'untitled' }: { docId: string; title?: string } = $props();

	type WBTool = 'select' | 'pan' | 'sticky' | 'rect' | 'ellipse' | 'arrow' | 'text';

	interface WBEl {
		id: string;
		type: 'sticky' | 'rect' | 'ellipse' | 'arrow' | 'text';
		x: number;
		y: number;
		w: number;
		h: number;
		text: string;
	}

	let canvasEl = $state<HTMLCanvasElement | undefined>();
	let tool = $state<WBTool>('select');
	let elements = $state<WBEl[]>([]);
	let selected = $state(new Set<string>());
	let editingId = $state<string | null>(null);
	let editText = $state('');
	let editBox = $state({ x: 0, y: 0, w: 200, h: 40 });

	let cam = $state({ x: 0, y: 0, zoom: 1 });
	let spaceDown = $state(false);
	let loaded = $state(false);

	type DragState = {
		mode: 'create' | 'move' | 'pan';
		type?: WBTool;
		startSX: number;
		startSY: number;
		startWX: number;
		startWY: number;
		origCam?: { x: number; y: number };
		orig?: Map<string, { x: number; y: number }>;
	};
	let drag = $state<DragState | null>(null);

	let saveTimer: ReturnType<typeof setTimeout>;
	let raf = 0;

	const COLORS = {
		sticky: '#f0b429',
		shape: '#7c6cf0',
		text: 'var(--color-text)',
		bg: 'var(--color-bg)',
		grid: 'var(--color-border)',
	};

	function screenToWorld(sx: number, sy: number): { x: number; y: number } {
		return { x: sx / cam.zoom + cam.x, y: sy / cam.zoom + cam.y };
	}
	function worldToScreen(wx: number, wy: number): { x: number; y: number } {
		return { x: (wx - cam.x) * cam.zoom, y: (wy - cam.y) * cam.zoom };
	}

	function uid(): string {
		return Math.random().toString(36).slice(2, 10);
	}

	function newElement(type: WBTool, x: number, y: number, w = 0, h = 0): WBEl {
		return { id: uid(), type: type as WBEl['type'], x, y, w, h, text: '' };
	}

	function markDirty() {
		clearTimeout(saveTimer);
		saveTimer = setTimeout(save, 800);
	}

	async function save() {
		try {
			await invoke('upsert_block', {
				id: `${docId}-whiteboard`,
				documentId: docId,
				blockType: 'whiteboard',
				content: { elements: elements.map(e => ({ ...e })) },
				sortOrder: 1,
			});
		} catch (e) {
			console.error('Failed to save whiteboard:', e);
		}
	}

	async function load() {
		loaded = false;
		try {
			const blocks = await invoke<Array<{ type: string; content: any }>>('get_blocks', { documentId: docId });
			const wb = blocks.find(b => b.type === 'whiteboard');
			const els = wb?.content?.elements;
			elements = Array.isArray(els) ? els.map((e: any) => ({
				id: e.id ?? uid(),
				type: e.type,
				x: e.x ?? 0,
				y: e.y ?? 0,
				w: e.w ?? 0,
				h: e.h ?? 0,
				text: e.text ?? '',
			})) : [];
		} catch (e) {
			console.error('Failed to load whiteboard:', e);
		} finally {
			loaded = true;
		}
	}

	// ── Hit testing (world coords) ──
	function hitTest(wx: number, wy: number): WBEl | null {
		for (let i = elements.length - 1; i >= 0; i--) {
			const el = elements[i];
			if (el.type === 'ellipse') {
				const nx = (wx - el.x) / (el.w / 2);
				const ny = (wy - el.y) / (el.h / 2);
				if (nx * nx + ny * ny <= 1) return el;
			} else if (el.type === 'arrow') {
				const d = distToSegment(wx, wy, el.x, el.y, el.x + el.w, el.y + el.h);
				if (d < 6 / cam.zoom) return el;
			} else {
				if (wx >= el.x && wx <= el.x + el.w && wy >= el.y && wy <= el.y + el.h) return el;
			}
		}
		return null;
	}

	function distToSegment(px: number, py: number, ax: number, ay: number, bx: number, by: number): number {
		const dx = bx - ax, dy = by - ay;
		const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)));
		return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
	}

	// ── Pointer handling ──
	function onPointerDown(e: PointerEvent) {
		if (editingId) commitEdit();
		const rect = canvasEl!.getBoundingClientRect();
		const sx = e.clientX - rect.left;
		const sy = e.clientY - rect.top;
		const { x: wx, y: wy } = screenToWorld(sx, sy);
		canvasEl!.setPointerCapture(e.pointerId);

		if (spaceDown || tool === 'pan' || e.button === 1) {
			drag = { mode: 'pan', startSX: sx, startSY: sy, startWX: wx, startWY: wy, origCam: { ...cam } };
			return;
		}
		if (tool === 'select') {
			const hit = hitTest(wx, wy);
			if (hit) {
				if (e.shiftKey) {
					const next = new Set(selected);
					next.has(hit.id) ? next.delete(hit.id) : next.add(hit.id);
					selected = next;
				} else if (!selected.has(hit.id)) {
					selected = new Set([hit.id]);
				}
				const orig = new Map<string, { x: number; y: number }>();
				for (const id of selected) {
					const el = elements.find(el => el.id === id);
					if (el) orig.set(id, { x: el.x, y: el.y });
				}
				drag = { mode: 'move', startSX: sx, startSY: sy, startWX: wx, startWY: wy, orig };
			} else {
				if (!e.shiftKey) selected = new Set();
				drag = { mode: 'pan', startSX: sx, startSY: sy, startWX: wx, startWY: wy, origCam: { ...cam } };
			}
			return;
		}
		// Create tools
		selected = new Set();
		const el = newElement(tool, wx, wy);
		elements = [...elements, el];
		selected = new Set([el.id]);
		drag = { mode: 'create', type: tool, startSX: sx, startSY: sy, startWX: wx, startWY: wy };
	}

	function onPointerMove(e: PointerEvent) {
		const d = drag;
		if (!d) return;
		const rect = canvasEl!.getBoundingClientRect();
		const sx = e.clientX - rect.left;
		const sy = e.clientY - rect.top;
		const { x: wx, y: wy } = screenToWorld(sx, sy);

		if (d.mode === 'pan') {
			cam = {
				x: d.origCam!.x - (sx - d.startSX) / cam.zoom,
				y: d.origCam!.y - (sy - d.startSY) / cam.zoom,
				zoom: cam.zoom,
			};
		} else if (d.mode === 'move' && d.orig) {
			const dx = wx - d.startWX;
			const dy = wy - d.startWY;
			elements = elements.map(el => {
				const o = d.orig!.get(el.id);
				return o ? { ...el, x: o.x + dx, y: o.y + dy } : el;
			});
		} else if (d.mode === 'create' && d.type) {
			const dx = wx - d.startWX;
			const dy = wy - d.startWY;
			const id = selected.values().next().value;
			elements = elements.map(el => {
				if (el.id !== id) return el;
				if (d.type === 'arrow') {
					return { ...el, w: dx, h: dy };
				}
				return {
					...el,
					x: dx < 0 ? wx : d.startWX,
					y: dy < 0 ? wy : d.startWY,
					w: Math.abs(dx),
					h: Math.abs(dy),
				};
			});
		}
	}

	function onPointerUp(e: PointerEvent) {
		if (drag?.mode === 'create') {
			const id = selected.values().next().value;
			const el = elements.find(el => el.id === id);
			if (el && (el.w < 4 || el.h < 4)) {
				// Tiny drag = click placement with default size
				const defaults: Record<string, { w: number; h: number }> = {
					sticky: { w: 180, h: 120 },
					rect: { w: 160, h: 100 },
					ellipse: { w: 160, h: 100 },
					arrow: { w: 100, h: 0 },
					text: { w: 200, h: 24 },
				};
				const d = defaults[el.type] ?? { w: 160, h: 100 };
				elements = elements.map(x => (x.id === id ? { ...x, w: d.w, h: d.h } : x));
				if (el.type === 'text' || el.type === 'sticky') {
					beginEdit(el);
				}
			}
			if (el) markDirty();
		} else if (drag?.mode === 'move') {
			markDirty();
		}
		drag = null;
	}

	function onWheel(e: WheelEvent) {
		e.preventDefault();
		const rect = canvasEl!.getBoundingClientRect();
		const sx = e.clientX - rect.left;
		const sy = e.clientY - rect.top;
		const { x: wx, y: wy } = screenToWorld(sx, sy);
		if (e.ctrlKey || e.metaKey) {
			const factor = Math.exp(-e.deltaY * 0.0015);
			const zoom = Math.min(4, Math.max(0.2, cam.zoom * factor));
			cam = { zoom, x: wx - sx / zoom, y: wy - sy / zoom };
		} else {
			cam = { ...cam, x: cam.x + e.deltaX / cam.zoom, y: cam.y + e.deltaY / cam.zoom };
		}
	}

	function onDblClick(e: MouseEvent) {
		const rect = canvasEl!.getBoundingClientRect();
		const { x: wx, y: wy } = screenToWorld(e.clientX - rect.left, e.clientY - rect.top);
		const hit = hitTest(wx, wy);
		if (hit && (hit.type === 'text' || hit.type === 'sticky' || hit.type === 'rect')) {
			beginEdit(hit);
		}
	}

	function beginEdit(el: WBEl) {
		editingId = el.id;
		editText = el.text;
		const p = worldToScreen(el.x, el.y);
		editBox = { x: p.x, y: p.y, w: Math.max(80, el.w * cam.zoom), h: Math.max(40, el.h * cam.zoom) };
	}

	function commitEdit() {
		if (!editingId) return;
		const id = editingId;
		editingId = null;
		elements = elements.map(el => {
			if (el.id !== id) return el;
			const text = editText.trimEnd();
			if (el.type === 'sticky' && text) {
				// Auto-size sticky to content
				const lines = text.split('\n').reduce((n, l) => n + Math.max(1, Math.ceil(l.length / 20)), 0);
				return { ...el, text, h: Math.max(60, lines * 18 + 24) };
			}
			return { ...el, text };
		});
		markDirty();
	}

	function deleteSelection() {
		if (editingId || selected.size === 0) return;
		const ids = selected;
		elements = elements.filter(el => !ids.has(el.id));
		selected = new Set();
		markDirty();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (editingId) {
			if (e.key === 'Escape') { editingId = null; }
			if (e.key === 'Enter' && !e.shiftKey) { commitEdit(); }
			return;
		}
		if (e.key === ' ') { spaceDown = true; e.preventDefault(); }
		else if (e.key === 'Delete' || e.key === 'Backspace') { deleteSelection(); e.preventDefault(); }
		else if (e.key === 'Escape') { selected = new Set(); tool = 'select'; }
		else if (e.key === 'v') { tool = 'select'; }
		else if (e.key === 'h') { tool = 'pan'; }
	}
	function handleKeyup(e: KeyboardEvent) {
		if (e.key === ' ') spaceDown = false;
	}

	// ── Rendering ──
	function drawElement(ctx: CanvasRenderingContext2D, el: WBEl, zoom: number) {
		const x = (el.x - cam.x) * zoom;
		const y = (el.y - cam.y) * zoom;
		const w = el.w * zoom;
		const h = el.h * zoom;
		ctx.font = `500 ${Math.max(12, 14 * zoom)}px Inter, sans-serif`;
		ctx.textBaseline = 'top';

		if (el.type === 'sticky') {
			ctx.fillStyle = COLORS.sticky;
			roundRect(ctx, x, y, w, h, 6);
			ctx.fill();
			drawWrapped(ctx, el.text, x + 10, y + 10, w - 20, zoom, '#1b1b1f');
		} else if (el.type === 'rect') {
			ctx.strokeStyle = COLORS.shape;
			ctx.lineWidth = 2;
			roundRect(ctx, x, y, w, h, 4);
			ctx.stroke();
			drawWrapped(ctx, el.text, x + 8, y + 8, w - 16, zoom, 'var(--color-text)');
		} else if (el.type === 'ellipse') {
			ctx.strokeStyle = COLORS.shape;
			ctx.lineWidth = 2;
			ctx.beginPath();
			ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
			ctx.stroke();
			drawWrapped(ctx, el.text, x + 10, y + h / 2 - 10, w - 20, zoom, 'var(--color-text)');
		} else if (el.type === 'arrow') {
			ctx.strokeStyle = COLORS.shape;
			ctx.lineWidth = 2.5;
			ctx.lineCap = 'round';
			ctx.beginPath();
			ctx.moveTo(x, y);
			ctx.lineTo(x + w, y + h);
			ctx.stroke();
			const ang = Math.atan2(h, w);
			const head = 10 * zoom;
			ctx.beginPath();
			ctx.moveTo(x + w, y + h);
			ctx.lineTo(x + w - head * Math.cos(ang - 0.5), y + h - head * Math.sin(ang - 0.5));
			ctx.moveTo(x + w, y + h);
			ctx.lineTo(x + w - head * Math.cos(ang + 0.5), y + h - head * Math.sin(ang + 0.5));
			ctx.stroke();
		} else {
			ctx.fillStyle = 'var(--color-text)';
			drawWrapped(ctx, el.text, x, y, w, zoom, 'var(--color-text)');
		}

		if (selected.has(el.id)) {
			ctx.strokeStyle = COLORS.shape;
			ctx.lineWidth = 1.5;
			ctx.setLineDash([4 * zoom, 3 * zoom]);
			roundRect(ctx, x - 4 * zoom, y - 4 * zoom, w + 8 * zoom, h + 8 * zoom, 6);
			ctx.stroke();
			ctx.setLineDash([]);
		}
	}

	function drawWrapped(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, maxW: number, zoom: number, color: string) {
		if (!text) return;
		ctx.fillStyle = color;
		const lh = Math.max(12, 18 * zoom);
		const words = text.split('\n');
		let line = '';
		let cy = y;
		for (const word of words) {
			line = '';
			const chars = word.split('');
			for (const ch of chars) {
				const test = line + ch;
				if (ctx.measureText(test).width > maxW && line) {
					ctx.fillText(line, x, cy);
					cy += lh;
					line = ch;
				} else {
					line = test;
				}
			}
			ctx.fillText(line, x, cy);
			cy += lh;
		}
	}

	function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
		ctx.beginPath();
		ctx.roundRect(x, y, w, h, r);
	}

	function render(ctx: CanvasRenderingContext2D, dpr: number) {
		const w = canvasEl!.clientWidth;
		const h = canvasEl!.clientHeight;
		if (canvasEl!.width !== w * dpr || canvasEl!.height !== h * dpr) {
			canvasEl!.width = w * dpr;
			canvasEl!.height = h * dpr;
		}
		ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
		ctx.clearRect(0, 0, w, h);
		const bg = getComputedStyle(document.documentElement).getPropertyValue('--color-bg');
		ctx.fillStyle = bg || '#141416';
		ctx.fillRect(0, 0, w, h);

		// Grid dots (world-aligned, spaced 32 units)
		const grid = 32 * cam.zoom;
		if (grid > 8) {
			ctx.fillStyle = 'var(--color-border)';
			const x0 = cam.x;
			const y0 = cam.y;
			for (let gx = Math.floor(x0 / 32) * 32; gx * cam.zoom - cam.x * cam.zoom < w + grid; gx += 32) {
				for (let gy = Math.floor(y0 / 32) * 32; gy * cam.zoom - cam.y * cam.zoom < h + grid; gy += 32) {
					ctx.fillRect((gx - cam.x) * cam.zoom - 1, (gy - cam.y) * cam.zoom - 1, 2, 2);
				}
			}
		}

		for (const el of elements) drawElement(ctx, el, cam.zoom);
	}

	$effect(() => {
		const el = canvasEl;
		if (!el) return;
		const ctx = el.getContext('2d')!;
		const loop = () => {
			render(ctx, Math.max(1, window.devicePixelRatio || 1));
			raf = requestAnimationFrame(loop);
		};
		loop();
		return () => cancelAnimationFrame(raf);
	});

	// Load on mount / doc change
	$effect(() => {
		load();
		return () => { clearTimeout(saveTimer); };
	});

	// ── Export ──
	async function exportPNG() {
		if (elements.length === 0) return;
		const pad = 48;
		const minX = Math.min(...elements.map(e => e.x)) - pad;
		const minY = Math.min(...elements.map(e => e.y)) - pad;
		const maxX = Math.max(...elements.map(e => e.x + e.w)) + pad;
		const maxY = Math.max(...elements.map(e => e.y + e.h)) + pad;
		const scale = 2;
		const c = document.createElement('canvas');
		c.width = (maxX - minX) * scale;
		c.height = (maxY - minY) * scale;
		const ctx = c.getContext('2d')!;
		ctx.scale(scale, scale);
		ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--color-bg') || '#141416';
		ctx.fillRect(0, 0, c.width, c.height);
		ctx.translate(-minX, -minY);
		const saveCam = cam;
		cam = { x: 0, y: 0, zoom: 1 };
		ctx.save();
		for (const el of elements) {
			drawElement(ctx, el, 1);
		}
		ctx.restore();
		cam = saveCam;
		const blob = await new Promise<Blob | null>(res => c.toBlob(res, 'image/png'));
		if (!blob) return;
		const data = Array.from(new Uint8Array(await blob.arrayBuffer()));
		const safe = `${title.replace(/[^\p{L}\p{N}\-_ .]/gu, '_') || 'whiteboard'}-whiteboard.png`;
		try {
			const path = await invoke<string>('export_file', { filename: safe, data });
			console.log('Whiteboard exported to', path);
		} catch (err) {
			console.error('Export failed:', err);
		}
	}

	function setTool(t: WBTool) {
		if (editingId) commitEdit();
		tool = t;
		selected = new Set();
	}
</script>

<div class="wb-wrap">
	<canvas
		bind:this={canvasEl}
		class="wb-canvas"
		class:panning={drag?.mode === 'pan'}
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		onpointerleave={onPointerUp}
		onwheel={onWheel}
		ondblclick={onDblClick}
	></canvas>

	{#if editingId}
		<!-- svelte-ignore a11y_autofocus -->
		<textarea
			class="wb-edit"
			bind:value={editText}
			style="left:{editBox.x}px; top:{editBox.y}px; width:{editBox.w}px; height:{editBox.h}px;"
			onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') editingId = null; if (e.key === 'Enter' && !e.shiftKey) commitEdit(); }}
			onblur={commitEdit}
			autofocus
		></textarea>
	{/if}

	{#if loaded && elements.length === 0}
		<div class="wb-empty">
			<p>Empty whiteboard — pick a tool below and click to add.</p>
		</div>
	{/if}

	<div class="wb-toolbar">
		<div class="tool-group">
			<button class="tool-btn" class:active={tool === 'select'} onclick={() => setTool('select')} title="Select (V)">
				<Icon name="link" size={15} />
			</button>
			<button class="tool-btn" class:active={tool === 'pan'} onclick={() => setTool('pan')} title="Pan (H, or hold Space)">
				<Icon name="more" size={15} />
			</button>
		</div>
		<div class="tool-group">
			<button class="tool-btn" class:active={tool === 'sticky'} onclick={() => setTool('sticky')} title="Sticky note">
				<span class="swatch sticky"></span>
			</button>
			<button class="tool-btn" class:active={tool === 'rect'} onclick={() => setTool('rect')} title="Rectangle">
				<span class="swatch rect"></span>
			</button>
			<button class="tool-btn" class:active={tool === 'ellipse'} onclick={() => setTool('ellipse')} title="Ellipse">
				<span class="swatch ellipse"></span>
			</button>
			<button class="tool-btn" class:active={tool === 'arrow'} onclick={() => setTool('arrow')} title="Arrow">
				<Icon name="chevronRight" size={15} />
			</button>
			<button class="tool-btn" class:active={tool === 'text'} onclick={() => setTool('text')} title="Text (T)">
				<Icon name="text" size={15} />
			</button>
		</div>
		<div class="tool-group">
			<button class="tool-btn" onclick={() => cam = { ...cam, zoom: Math.min(4, cam.zoom * 1.25) }} title="Zoom in">
				+
			</button>
			<span class="zoom-label">{Math.round(cam.zoom * 100)}%</span>
			<button class="tool-btn" onclick={() => cam = { ...cam, zoom: Math.max(0.2, cam.zoom / 1.25) }} title="Zoom out">
				−
			</button>
			<button class="tool-btn" onclick={() => cam = { x: 0, y: 0, zoom: 1 }} title="Reset view">⟳</button>
		</div>
		<div class="tool-group">
			<button class="tool-btn" onclick={exportPNG} title="Export as PNG">
				<Icon name="download" size={15} />
			</button>
		</div>
	</div>
</div>

<svelte:window onkeydown={handleKeydown} onkeyup={handleKeyup} />

<style>
	.wb-wrap {
		flex: 1;
		min-height: 0;
		position: relative;
		overflow: hidden;
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border);
		margin-bottom: 24px;
	}
	.wb-canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		cursor: default;
		touch-action: none;
	}
	.wb-canvas.panning { cursor: grab; }

	.wb-edit {
		position: absolute;
		z-index: 10;
		background: var(--color-surface);
		border: 1.5px solid var(--color-accent);
		border-radius: 4px;
		color: var(--color-text);
		font: 500 14px Inter, sans-serif;
		padding: 4px 6px;
		resize: none;
		outline: none;
		box-shadow: var(--shadow-md);
	}

	.wb-empty {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: none;
		color: var(--color-text-faint);
		font-size: 14px;
	}

	.wb-toolbar {
		position: absolute;
		top: 12px;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		gap: 6px;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: 5px;
		box-shadow: var(--shadow-md);
		z-index: 5;
	}
	.tool-group {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 0 4px;
	}
	.tool-group + .tool-group { border-left: 1px solid var(--color-border); }
	.tool-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 14px;
		padding: 0;
		transition: background 0.1s, color 0.1s;
	}
	.tool-btn:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.tool-btn.active { background: var(--color-accent-subtle); color: var(--color-accent); }

	.swatch { display: block; border-radius: 3px; }
	.swatch.sticky { width: 14px; height: 14px; background: #f0b429; }
	.swatch.rect { width: 14px; height: 14px; border: 2px solid var(--color-accent); }
	.swatch.ellipse { width: 14px; height: 14px; border: 2px solid var(--color-accent); border-radius: 50%; }

	.zoom-label { font-size: 12px; color: var(--color-text-muted); min-width: 38px; text-align: center; }
</style>
