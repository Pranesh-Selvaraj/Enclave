export interface GraphLink {
	source: string;
	target: string;
}

function add(links: Map<string, GraphLink>, source: string, target: string) {
	if (source === target) return;
	const key = `${source}\u0000${target}`;
	if (!links.has(key)) links.set(key, { source, target });
}

function walk(node: unknown, titleToId: Map<string, string>, links: Map<string, GraphLink>, sourceId: string) {
	if (!node || typeof node !== 'object') return;
	if (Array.isArray(node)) {
		for (const c of node) walk(c, titleToId, links, sourceId);
		return;
	}
	const n = node as Record<string, unknown>;
	if (n.type === 'pageEmbed' && typeof (n.attrs as Record<string, unknown> | undefined)?.docId === 'string') {
		add(links, sourceId, (n.attrs as { docId: string }).docId);
	}
	if (Array.isArray(n.marks)) {
		for (const m of n.marks) {
			const href = (m as { attrs?: { href?: string } }).attrs?.href;
			const match = typeof href === 'string' ? href.match(/^\/doc\/([^/]+)/) : null;
			if (match) add(links, sourceId, match[1]);
		}
	}
	if (typeof n.text === 'string') {
		for (const match of n.text.matchAll(/\[\[([^\]]+)\]\]/g)) {
			const targetId = titleToId.get(match[1]);
			if (targetId) add(links, sourceId, targetId);
		}
	}
	for (const [k, v] of Object.entries(n)) {
		if (k === 'marks' || k === 'attrs') continue;
		walk(v, titleToId, links, sourceId);
	}
}

/**
 * All page connections inside one block's ProseMirror JSON:
 * page-embed nodes, /doc/ link marks, and [[wiki]] titles.
 */
export function extractLinks(content: unknown, titleToId: Map<string, string>, sourceId: string): GraphLink[] {
	const links = new Map<string, GraphLink>();
	walk(content, titleToId, links, sourceId);
	return [...links.values()];
}
