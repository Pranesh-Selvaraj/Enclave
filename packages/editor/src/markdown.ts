import TurndownService from 'turndown';
import { marked } from 'marked';
import type { JSONContent } from '@tiptap/core';

const turndown = new TurndownService({
	headingStyle: 'atx',
	codeBlockStyle: 'fenced',
	emDelimiter: '*',
	bulletListMarker: '-',
});

/** Convert an HTML string to Markdown. */
export function htmlToMarkdown(html: string): string {
	return turndown.turndown(html);
}

// Mentions export as wikilinks so backlinks/graph still see them.
turndown.addRule('mention', {
	filter: (node) => node.nodeName === 'SPAN' && (node as HTMLElement).hasAttribute('data-mention'),
	replacement: (_content, node) => {
		const el = node as HTMLElement;
		const title = el.getAttribute('data-title') ?? '';
		return title ? `[[${title}]]` : '';
	},
});

// Render page embeds as links in exports.
turndown.addRule('pageEmbed', {
	filter: (node) => node.nodeName === 'DIV' && (node as HTMLElement).hasAttribute('data-page-embed'),
	replacement: (_content, node) => {
		const el = node as HTMLElement;
		const docId = el.getAttribute('data-doc-id') ?? '';
		const title = el.getAttribute('data-title') ?? '';
		if (!docId) return '';
		return `\n\n[${title || 'Open page'}](/doc/${docId})\n\n`;
	},
});

// Render bookmarks as links in exports.
turndown.addRule('bookmark', {
	filter: (node) => node.nodeName === 'DIV' && (node as HTMLElement).hasAttribute('data-bookmark'),
	replacement: (_content, node) => {
		const el = node as HTMLElement;
		const url = el.getAttribute('data-url') ?? '';
		const title = el.getAttribute('data-title') ?? '';
		if (!url) return '';
		return `\n\n[${title || url}](<${url}>)\n\n`;
	},
});

// Render database blocks as Markdown tables in exports.
turndown.addRule('database', {
	filter: (node) => node.nodeName === 'DIV' && (node as HTMLElement).hasAttribute('data-database'),
	replacement: (_content, node) => {
		try {
			const d = JSON.parse((node as HTMLElement).getAttribute('data-database') || '{}');
			const cols: { id: string; name: string }[] = d.columns ?? [];
			const rows: { cells: Record<string, string | boolean> }[] = d.rows ?? [];
			if (cols.length === 0) return '';
			const header = '| ' + cols.map((c) => c.name).join(' | ') + ' |';
			const sep = '| ' + cols.map(() => '---').join(' | ') + ' |';
			const body = rows
				.map((r) => '| ' + cols.map((c) => String(r.cells?.[c.id] ?? '')).join(' | ') + ' |')
				.join('\n');
			return '\n\n' + header + '\n' + sep + (body ? '\n' + body : '') + '\n\n';
		} catch {
			return '';
		}
	},
});

/**
 * TipTap's TaskItem matches li[data-type=taskItem], marked only emits bare
 * checkboxes — rewrite so `- [ ]` imports as interactive tasks.
 */
export function rewriteTaskItems(html: string): string {
	return html.replace(
		/<li><input (checked="" )?disabled="" type="checkbox">/g,
		(_m: string, checked: string) => `<li data-type="taskItem" data-checked="${checked ? 'true' : 'false'}">`,
	);
}

// The extension list drags in .svelte node views, so it's imported lazily —
// keeps this module loadable in plain node (tests) and in the browser.
async function withExtensions<T>(fn: (extensions: ReturnType<typeof import('./extensions.js')['editorExtensions']>) => T): Promise<T> {
	const { editorExtensions } = await import('./extensions.js');
	return fn(editorExtensions());
}

/** Convert a Markdown string to ProseMirror JSON. */
export async function markdownToJson(md: string): Promise<JSONContent> {
	const { generateJSON } = await import('@tiptap/html');
	const html = rewriteTaskItems(marked.parse(md, { async: false }) as string);
	return withExtensions((ext) => generateJSON(html, ext));
}

/** Convert ProseMirror JSON back to Markdown. */
export async function jsonToMarkdown(json: JSONContent): Promise<string> {
	const { generateHTML } = await import('@tiptap/html');
	return withExtensions((ext) => htmlToMarkdown(generateHTML(json, ext)));
}
