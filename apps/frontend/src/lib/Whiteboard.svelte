<script lang="ts">
	// Enclave Whiteboard — infinite canvas ("edgeless" mode).
	// v2: frames + presentation, page embeds, mindmaps on top of the v1 base.
	// ponytail: no connectors, no resize handles, no undo — add when real use
	// demands it. Mindmap children don't follow a dragged parent; frames own
	// elements by containment (center inside), not by grouping.
	// Persistence: one 'whiteboard' block per doc (no schema change).
	import { invoke } from '$lib/backend.js';
	import { Icon } from '@enclave/ui';
	import { mindmapChildPos, centerInside, orderFrames, fitCam, type Rect } from '$lib/wbLayout';

	let { docId, title = 'untitled' }: { docId: string; title?: string } = $props();

	type WBTool = 'select' | 'pan' | 'sticky' | 'rect' | 'ellipse' | 'arrow' | 'text' | 'frame' | 'embed' | 'mindmap';

	interface WBEl {
		id: string;
		type: 'sticky' | 'rect' | 'ellipse' | 'arrow' | 'text' | 'frame' | 'embed' | 'mm-node';
		x: number;
		y: number;
		w: number;
		h: number;
		text: string;
		parentId?: string;
		docId?: string;
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

	// Page picker for embeds
	let pages = $state<{ id: string; title: string }[]>([]);
	let embedPicker = $state<{ x: number; y: number; elId: string } | null>(null);
	let embedQuery = $state('');
	let presCanvas = $state<HTMLCanvasElement | undefined>();
	let presenting = $state<{ index: number } | null>(null);

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
	let presRaf = 0;

	const COLORS = {
		sticky: '#f0b429',
		shape: '#7c6cf0',
		bg: '#141416',
		grid: '#2a2a30',
		text: '#e6e6ea',
		border: '#2a2a30',
		accent: '#7c6cf0',
	};

	// CSS custom properties don't resolve in canvas fills — resolve once per
	// render so theme changes apply without a remount.
	function readTheme(): Record<string, string> {
		const s = getComputedStyle(document.documentElement);
		const v = (n: string, fb: string) => s.getPropertyValue(n).trim() || fb;
		return {
			bg: v('--color-bg', COLORS.bg),
			grid: v('--color-border', COLORS.grid),
			text: v('--color-text', COLORS.text),
			accent: v('--color-accent', COLORS.accent),
			surface: v('--color-surface', '#1e1e24'),
			muted: v('--color-text-muted', '#888'),
		};
	}

	function screenToWorld(sx: number, sy: number): { x: number; y: number } {
		return { x: sx / cam.zoom + cam.x, y: sy / cam.zoom + cam.y };
	}
	function worldToScreen(wx: number, wy: number): { x: number; y: number } {
		return { x: (wx - cam.x) * cam.zoom, y: (wy - cam.y) * cam.zoom };
	}

	function uid(): string {
		return Math.random().toString(36).slice(2, 10);
	}

	function newElement(type: string, x: number, y: number, w = 0, h = 0): WBEl {
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
				parentId: e.parentId,
				docId: e.docId,
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
		if (embedPicker) embedPicker = null;
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
		if (tool === 'embed') {
			// Place the embed and open the page picker right away.
			const el = newElement('embed', wx, wy);
			elements = [...elements, el];
			selected = new Set([el.id]);
			embedPicker = { x: e.clientX, y: e.clientY, elId: el.id };
			embedQuery = '';
			return;
		}
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
			// A dragged frame carries its contained elements along.
			const draggedFrame = elements.find(el => el.type === 'frame' && d.orig!.has(el.id));
			const frameNow = draggedFrame
				? { ...draggedFrame, x: d.orig!.get(draggedFrame.id)!.x + dx, y: d.orig!.get(draggedFrame.id)!.y + dy }
				: null;
			elements = elements.map(el => {
				const o = d.orig!.get(el.id);
				if (o) return { ...el, x: o.x + dx, y: o.y + dy };
				if (frameNow && centerInside(el, frameNow)) return { ...el, x: el.x + dx, y: el.y + dy };
				return el;
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
					frame: { w: 400, h: 240 },
					'mm-node': { w: 140, h: 44 },
				};
				const d = defaults[el.type] ?? { w: 160, h: 100 };
				elements = elements.map(x => (x.id === id ? { ...x, w: d.w, h: d.h } : x));
				if (el.type === 'text' || el.type === 'sticky' || el.type === 'mm-node') {
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
		if (!hit) return;
		if (hit.type === 'embed' && hit.docId) {
			location.href = `/${hit.docId}`;
			return;
		}
		if (hit.type === 'text' || hit.type === 'sticky' || hit.type === 'rect' || hit.type === 'mm-node') {
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
			if (el.type === 'mm-node' && text) {
				const w = Math.max(140, Math.ceil(text.length / 1.2) * 8 + 28);
				return { ...el, text, w };
			}
			return { ...el, text };
		});
		markDirty();
	}

	// ── Mindmap ──
	function mmChildren(id: string): WBEl[] {
		return elements.filter(el => el.parentId === id);
	}

	function mmAddChild(parentId: string) {
		const parent = elements.find(el => el.id === parentId);
		if (!parent) return;
		const siblings = mmChildren(parentId);
		const pos = mindmapChildPos(parent, siblings);
		const node = { ...newElement('mm-node', pos.x, pos.y, 140, 44), parentId };
		elements = [...elements, node];
		selected = new Set([node.id]);
		markDirty();
		beginEdit(node);
	}

	function mmSubtree(ids: Set<string>): Set<string> {
		const out = new Set(ids);
		let grew = true;
		while (grew) {
			grew = false;
			for (const el of elements) {
				if (el.parentId && out.has(el.parentId) && !out.has(el.id)) {
					out.add(el.id);
					grew = true;
				}
			}
		}
		return out;
	}

	function deleteSelection() {
		if (editingId || selected.size === 0) return;
		let ids = selected;
		if ([...ids].some(id => elements.find(el => el.id === id)?.type === 'mm-node')) {
			ids = mmSubtree(ids);
		}
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
		else if (e.key === 'Tab') {
			const sel = [...selected];
			if (sel.length === 1 && elements.find(el => el.id === sel[0])?.type === 'mm-node') {
				e.preventDefault();
				mmAddChild(sel[0]);
			}
		}
	}
	function handleKeyup(e: KeyboardEvent) {
		if (e.key === ' ') spaceDown = false;
	}

	// ── Rendering ──
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

	function drawElement(ctx: CanvasRenderingContext2D, el: WBEl, zoom: number, theme: Record<string, string>) {
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
			drawWrapped(ctx, el.text, x + 8, y + 8, w - 16, zoom, theme.text);
		} else if (el.type === 'ellipse') {
			ctx.strokeStyle = COLORS.shape;
			ctx.lineWidth = 2;
			ctx.beginPath();
			ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
			ctx.stroke();
			drawWrapped(ctx, el.text, x + 10, y + h / 2 - 10, w - 20, zoom, theme.text);
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
		} else if (el.type === 'frame') {
			ctx.fillStyle = theme.accent + '14';
			roundRect(ctx, x, y, w, h, 8);
			ctx.fill();
			ctx.strokeStyle = theme.accent + '66';
			ctx.lineWidth = 1.5;
			roundRect(ctx, x, y, w, h, 8);
			ctx.stroke();
			// Title pill
			const tw = Math.max(40, ctx.measureText(el.text || 'Frame').width + 18 * zoom);
			ctx.fillStyle = theme.accent;
			roundRect(ctx, x + 8 * zoom, y + 8 * zoom, tw, 18 * zoom, 4 * zoom);
			ctx.fill();
			ctx.fillStyle = '#fff';
			ctx.font = `600 ${Math.max(10, 11 * zoom)}px Inter, sans-serif`;
			ctx.fillText(el.text || 'Frame', x + (8 + 9) * zoom, y + (8 + 4) * zoom);
			ctx.font = `500 ${Math.max(12, 14 * zoom)}px Inter, sans-serif`;
		} else if (el.type === 'embed') {
			ctx.fillStyle = theme.surface;
			roundRect(ctx, x, y, w, h, 8);
			ctx.fill();
			ctx.strokeStyle = theme.border;
			ctx.lineWidth = 1;
			roundRect(ctx, x, y, w, h, 8);
			ctx.stroke();
			ctx.font = `500 ${Math.max(12, 14 * zoom)}px Inter, sans-serif`;
			drawWrapped(ctx, '📄', x + 10, y + h / 2 - 9 * zoom, 20, zoom, theme.text);
			drawWrapped(ctx, el.text || 'Untitled', x + 30, y + h / 2 - 9 * zoom, w - 40, zoom, theme.text);
		} else if (el.type === 'mm-node') {
			ctx.fillStyle = theme.surface;
			roundRect(ctx, x, y, w, h, 8);
			ctx.fill();
			ctx.strokeStyle = theme.accent;
			ctx.lineWidth = 1.5;
			roundRect(ctx, x, y, w, h, 8);
			ctx.stroke();
			ctx.textBaseline = 'middle';
			ctx.fillStyle = theme.text;
			const ty = y + h / 2 - 7 * zoom;
			if (ctx.measureText(el.text).width > w - 16) {
				drawWrapped(ctx, el.text, x + 8, ty, w - 16, zoom, theme.text);
			} else {
				ctx.fillText(el.text, x + w / 2 - ctx.measureText(el.text).width / 2, ty);
			}
			ctx.textBaseline = 'top';
		} else {
			ctx.fillStyle = theme.text;
			drawWrapped(ctx, el.text, x, y, w, zoom, theme.text);
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

	function drawMindmapEdges(ctx: CanvasRenderingContext2D, zoom: number, theme: Record<string, string>) {
		ctx.strokeStyle = theme.muted;
		ctx.lineWidth = 1.5;
		ctx.beginPath();
		for (const el of elements) {
			if (el.type !== 'mm-node' || !el.parentId) continue;
			const parent = elements.find(p => p.id === el.parentId);
			if (!parent) continue;
			const x1 = (parent.x + parent.w - cam.x) * zoom;
			const y1 = (parent.y + parent.h / 2 - cam.y) * zoom;
			const x2 = (el.x - cam.x) * zoom;
			const y2 = (el.y + el.h / 2 - cam.y) * zoom;
			const mx = (x1 + x2) / 2;
			ctx.moveTo(x1, y1);
			ctx.bezierCurveTo(mx, y1, mx, y2, x2, y2);
		}
		ctx.stroke();
	}

	function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
		ctx.beginPath();
		ctx.roundRect(x, y, w, h, r);
	}

	function drawScene(ctx: CanvasRenderingContext2D, w: number, h: number, dpr: number, clipFrame?: Rect) {
		if (canvasEl!.width !== w * dpr || canvasEl!.height !== h * dpr) {
			canvasEl!.width = w * dpr;
			canvasEl!.height = h * dpr;
		}
		ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
		ctx.clearRect(0, 0, w, h);
		const theme = readTheme();
		ctx.fillStyle = theme.bg;
		ctx.fillRect(0, 0, w, h);

		// Grid dots (world-aligned, spaced 32 units)
		const grid = 32 * cam.zoom;
		if (grid > 8) {
			ctx.fillStyle = theme.grid;
			for (let gx = Math.floor(cam.x / 32) * 32; gx * cam.zoom - cam.x * cam.zoom < w + grid; gx += 32) {
				for (let gy = Math.floor(cam.y / 32) * 32; gy * cam.zoom - cam.y * cam.zoom < h + grid; gy += 32) {
					ctx.fillRect((gx - cam.x) * cam.zoom - 1, (gy - cam.y) * cam.zoom - 1, 2, 2);
				}
			}
		}

		if (clipFrame) {
			const fx = (clipFrame.x - cam.x) * cam.zoom;
			const fy = (clipFrame.y - cam.y) * cam.zoom;
			ctx.save();
			ctx.beginPath();
			ctx.rect(fx - 8, fy - 8, clipFrame.w * cam.zoom + 16, clipFrame.h * cam.zoom + 16);
			ctx.clip();
		}
		drawMindmapEdges(ctx, cam.zoom, theme);
		for (const el of elements) drawElement(ctx, el, cam.zoom, theme);
		if (clipFrame) ctx.restore();
	}

	$effect(() => {
		const el = canvasEl;
		if (!el) return;
		const ctx = el.getContext('2d')!;
		const loop = () => {
			const w = el.clientWidth;
			const h = el.clientHeight;
			if (w > 0 && h > 0) drawScene(ctx, w, h, Math.max(1, window.devicePixelRatio || 1));
			raf = requestAnimationFrame(loop);
		};
		loop();
		return () => cancelAnimationFrame(raf);
	});

	// Presentation render loop
	$effect(() => {
		const el = presCanvas;
		if (!el || !presenting) return;
		const ctx = el.getContext('2d')!;
		const loop = () => {
			const frames = orderFrames(elements.filter(e => e.type === 'frame'));
			const frame = frames[presenting!.index];
			const w = el.clientWidth;
			const h = el.clientHeight;
			if (frame && w > 0 && h > 0) {
				cam = fitCam(frame, w, h);
				drawScene(ctx, w, h, Math.max(1, window.devicePixelRatio || 1), frame);
			}
			presRaf = requestAnimationFrame(loop);
		};
		loop();
		return () => cancelAnimationFrame(presRaf);
	});

	// Load on mount / doc change
	$effect(() => {
		load();
		invoke<{ id: string; title: string }[]>('get_page_list')
			.then(list => { pages = list; })
			.catch(() => {});
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
		const theme = readTheme();
		ctx.fillStyle = theme.bg;
		ctx.fillRect(0, 0, c.width, c.height);
		ctx.translate(-minX, -minY);
		const saveCam = cam;
		cam = { x: 0, y: 0, zoom: 1 };
		ctx.save();
		drawMindmapEdges(ctx, 1, theme);
		for (const el of elements) {
			drawElement(ctx, el, 1, theme);
		}
		ctx.restore();
		cam = saveCam;
		const blob = await new Promise<Blob | null>(res => c.toBlob(res, 'image/png'));
		if (!blob) return;
		const safe = `${title.replace(/[^\p{L}\p{N}\-_ .]/gu, '_') || 'whiteboard'}-whiteboard.png`;
		try {
			const data = Array.from(new Uint8Array(await blob.arrayBuffer()));
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

	// ── Presentation ──
	let frameList = $derived(orderFrames(elements.filter(e => e.type === 'frame')));

	function startPresentation() {
		if (frameList.length === 0) return;
		presenting = { index: 0 };
	}

	function presNav(delta: number) {
		if (!presenting) return;
		presenting = { index: (presenting.index + delta + frameList.length) % frameList.length };
	}

	function presKeydown(e: KeyboardEvent) {
		if (!presenting) return;
		if (e.key === 'Escape') { presenting = null; return; }
		if (e.key === 'ArrowRight' || e.key === 'ArrowDown' || e.key === ' ') { e.preventDefault(); presNav(1); }
		else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') { e.preventDefault(); presNav(-1); }
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
			<button class="tool-btn" class:active={tool === 'frame'} onclick={() => setTool('frame')} title="Frame — group elements for presentation">
				<span class="swatch frame"></span>
			</button>
			<button class="tool-btn" class:active={tool === 'embed'} onclick={() => setTool('embed')} title="Embed a page">
				<span class="swatch embed">📄</span>
			</button>
			<button class="tool-btn" class:active={tool === 'mindmap'} onclick={() => setTool('mindmap')} title="Mindmap — click for a node, Tab adds a child">
				<span class="swatch mindmap">☰</span>
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
			{#if frameList.length > 0}
				<button class="tool-btn pres-btn" onclick={startPresentation} title="Present frames">
					<span class="pres-dot"></span>
					<span>Present</span>
				</button>
			{/if}
			<button class="tool-btn" onclick={exportPNG} title="Export as PNG">
				<Icon name="download" size={15} />
			</button>
		</div>
	</div>
</div>

{#if embedPicker}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="wb-backdrop" onclick={() => (embedPicker = null)}></div>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="wb-picker" role="listbox" aria-label="Embed a page" tabindex="-1" style="left:{embedPicker.x}px; top:{embedPicker.y}px;" onclick={(e: MouseEvent) => e.stopPropagation()}>
		<div class="wb-picker-title">Embed a page</div>
		<input class="wb-picker-search" placeholder="Search pages…" bind:value={embedQuery} />
		{#each (embedQuery ? pages.filter(p => p.title.toLowerCase().includes(embedQuery.toLowerCase())) : pages) as p (p.id)}
			<div
				class="wb-picker-item"
				role="option"
				aria-selected={false}
				tabindex="0"
				onclick={() => {
					elements = elements.map(el => el.id === embedPicker!.elId ? { ...el, docId: p.id, text: p.title || 'Untitled' } : el);
					embedPicker = null;
					selected = new Set();
					markDirty();
				}}
			>
				<span>📄</span>
				<span>{p.title || 'Untitled'}</span>
			</div>
		{/each}
		{#if pages.length === 0}
			<div class="wb-picker-title">No pages yet</div>
		{/if}
	</div>
{/if}

{#if presenting}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="wb-present" onkeydown={presKeydown}>
		<!-- svelte-ignore a11y_autofocus -->
		<canvas bind:this={presCanvas} class="wb-pres-canvas" tabindex="-1" autofocus></canvas>
		<div class="wb-pres-title">{frameList[presenting.index]?.text || 'Frame'}</div>
		<div class="wb-pres-count">{presenting.index + 1} / {frameList.length}</div>
		<div class="wb-pres-nav">
			<button class="tool-btn" onclick={() => presNav(-1)} title="Previous (←)">←</button>
			<button class="tool-btn" onclick={() => presNav(1)} title="Next (→)">→</button>
		</div>
		<button class="wb-pres-exit" onclick={() => (presenting = null)}>Exit (Esc)</button>
	</div>
{/if}

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
		max-width: calc(100% - 24px);
		flex-wrap: wrap;
		justify-content: center;
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

	.swatch { display: block; border-radius: 3px; font-size: 12px; line-height: 14px; }
	.swatch.sticky { width: 14px; height: 14px; background: #f0b429; }
	.swatch.rect { width: 14px; height: 14px; border: 2px solid var(--color-accent); }
	.swatch.ellipse { width: 14px; height: 14px; border: 2px solid var(--color-accent); border-radius: 50%; }
	.swatch.frame { width: 14px; height: 14px; border: 1.5px dashed var(--color-accent); border-radius: 3px; }
	.swatch.embed { display: flex; align-items: center; justify-content: center; width: 16px; height: 14px; }
	.swatch.mindmap { display: flex; align-items: center; justify-content: center; width: 16px; height: 14px; font-weight: 700; }

	.zoom-label { font-size: 12px; color: var(--color-text-muted); min-width: 38px; text-align: center; }

	.pres-btn { width: auto; padding: 0 10px; gap: 6px; font-size: 12px; font-weight: 600; color: var(--color-accent); }
	.pres-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--color-accent); }

	/* ── Embed picker ── */
	.wb-backdrop {
		position: fixed;
		inset: 0;
		z-index: 200;
	}
	.wb-picker {
		position: fixed;
		z-index: 201;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: var(--shadow-lg);
		padding: 6px;
		width: 260px;
		max-height: 280px;
		overflow-y: auto;
	}
	.wb-picker-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		padding: 4px 8px;
	}
	.wb-picker-search {
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		padding: 6px 8px;
		outline: none;
		border-bottom: 1px solid var(--color-border);
		margin-bottom: 4px;
	}
	.wb-picker-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
	}
	.wb-picker-item:hover { background: var(--color-hover); }

	/* ── Presentation ── */
	.wb-present {
		position: fixed;
		inset: 0;
		z-index: 300;
		background: #000;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.wb-pres-canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		outline: none;
	}
	.wb-pres-title {
		position: absolute;
		top: 16px;
		left: 50%;
		transform: translateX(-50%);
		color: rgba(255, 255, 255, 0.7);
		font-size: 14px;
		font-weight: 600;
		text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
	}
	.wb-pres-count {
		position: absolute;
		bottom: 16px;
		left: 50%;
		transform: translateX(-50%);
		color: rgba(255, 255, 255, 0.5);
		font-size: 12px;
	}
	.wb-pres-nav {
		position: absolute;
		bottom: 14px;
		right: 16px;
		display: flex;
		gap: 6px;
	}
	.wb-pres-nav .tool-btn {
		background: rgba(255, 255, 255, 0.08);
		color: #fff;
		width: 36px;
		height: 36px;
		font-size: 16px;
	}
	.wb-pres-nav .tool-btn:hover { background: rgba(255, 255, 255, 0.18); }
	.wb-pres-exit {
		position: absolute;
		top: 14px;
		right: 16px;
		border: none;
		background: rgba(255, 255, 255, 0.08);
		color: rgba(255, 255, 255, 0.7);
		border-radius: 8px;
		padding: 6px 12px;
		cursor: pointer;
		font-size: 12px;
	}
	.wb-pres-exit:hover { background: rgba(255, 255, 255, 0.18); color: #fff; }
</style>
