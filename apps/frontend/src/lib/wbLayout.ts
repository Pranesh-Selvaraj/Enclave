// Pure whiteboard layout helpers — node-safe so the test suite can run them.

export interface Rect {
	x: number;
	y: number;
	w: number;
	h: number;
}

/** Slot for a new mindmap child: right of the parent, below its siblings. */
export function mindmapChildPos(parent: Rect, siblings: Rect[]): { x: number; y: number } {
	const x = parent.x + parent.w + 24;
	const y = siblings.length === 0 ? parent.y : Math.max(...siblings.map((s) => s.y + s.h)) + 24;
	return { x, y };
}

/** True when the element's center lies inside the frame. */
export function centerInside(el: Rect, frame: Rect): boolean {
	const cx = el.x + el.w / 2;
	const cy = el.y + el.h / 2;
	return cx >= frame.x && cx <= frame.x + frame.w && cy >= frame.y && cy <= frame.y + frame.h;
}

/** Presentation order: top-to-bottom, then left-to-right. */
export function orderFrames<T extends Rect>(frames: T[]): T[] {
	return [...frames].sort((a, b) => a.y - b.y || a.x - b.x);
}

/** Camera that fits a frame in a viewport, preserving the zoom clamp. */
export function fitCam(frame: Rect, cw: number, ch: number): { x: number; y: number; zoom: number } {
	const zoom = Math.min(Math.min(cw / frame.w, ch / frame.h) * 0.92, 4);
	return { zoom, x: frame.x + frame.w / 2 - cw / 2 / zoom, y: frame.y + frame.h / 2 - ch / 2 / zoom };
}
