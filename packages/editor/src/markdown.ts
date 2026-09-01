import { marked } from 'marked';
import type { JSONContent } from '@tiptap/core';

// ── HTML → Markdown ─────────────────────────────────────────────────────────
// ponytail: hand-rolled serializer replaces turndown (dep dropped). Covers the
// editor's output subset — headings, lists (incl. task items), blockquote, code
// blocks, links, bold/italic/inline-code — plus the custom data-* blocks
// (mention, page embed, bookmark, database table). Unknown tags degrade to
// their inline text rather than erroring.

interface Token {
	kind: 'tag' | 'text';
	name: string; // tag name, lowercased; '/' prefix for closing tags
	attrs: Record<string, string>;
	selfClosing: boolean;
	text?: string;
}

const VOID = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr']);

function tokenize(html: string): Token[] {
	const tokens: Token[] = [];
	const tagRe = /<\/?[a-zA-Z][^<>]*>/g;
	let last = 0;
	for (const m of html.matchAll(tagRe)) {
		if (m.index! > last) tokens.push({ kind: 'text', name: '', attrs: {}, selfClosing: false, text: html.slice(last, m.index!) });
		const raw = m[0];
		const close = raw.startsWith('</');
		const inner = raw.replace(/^<\/?/, '').replace(/\/?>$/, '');
		const sp = inner.search(/\s/);
		const name = (sp === -1 ? inner : inner.slice(0, sp)).toLowerCase();
		const attrs: Record<string, string> = {};
		for (const am of (sp === -1 ? '' : inner.slice(sp + 1)).matchAll(/([a-zA-Z-]+)(?:="([^"]*)")?/g)) {
			attrs[am[1]] = am[2] ?? '';
		}
		tokens.push({ kind: 'tag', name: close ? '/' + name : name, attrs, selfClosing: !close && (VOID.has(name) || raw.endsWith('/>')) });
		last = m.index! + raw.length;
	}
	if (last < html.length) tokens.push({ kind: 'text', name: '', attrs: {}, selfClosing: false, text: html.slice(last) });
	return tokens;
}

function decode(s: string): string {
	return s
		.replace(/&nbsp;/g, ' ')
		.replace(/&quot;/g, '"')
		.replace(/&#39;/g, "'")
		.replace(/&apos;/g, "'")
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>')
		.replace(/&amp;/g, '&');
}

function convert(tokens: Token[], i: number): { out: string; next: number } {
	let out = '';
	while (i < tokens.length) {
		const t = tokens[i];
		if (t.kind === 'text') {
			out += decode(t.text!);
			i++;
			continue;
		}
		const name = t.name;
		if (name.startsWith('/')) break;
		if (name === 'pre') {
			const r = renderPre(tokens, i);
			if (out && !out.endsWith('\n')) out += '\n';
			out += r.out;
			i = r.next;
			continue;
		}
		if (name === 'ul' || name === 'ol') {
			const r = renderList(tokens, i, name === 'ol');
			if (out && !out.endsWith('\n')) out += '\n';
			out += r.out;
			i = r.next;
			continue;
		}
		if (name === 'table') {
			const r = renderTableTag(tokens, i);
			if (out && !out.endsWith('\n')) out += '\n';
			out += r.out;
			i = r.next;
			continue;
		}
		if (t.selfClosing) {
			if (name === 'br') out += '\n';
			else if (name === 'hr') out += '\n---\n';
			else if (name === 'img') out += `![${decode(t.attrs.alt ?? '')}](${t.attrs.src ?? ''})`;
			// input and other voids are dropped (turndown parity)
			i++;
			continue;
		}
		const inner = convert(tokens, i + 1);
		out += renderElement(name, t.attrs, inner.out);
		i = inner.next;
		if (i < tokens.length) i++;
	}
	return { out, next: i };
}

/// TipTap table → GFM markdown table. First row is the header (TipTap inserts
/// one by default); thead/tbody wrappers are skipped while walking rows.
function renderTableTag(tokens: Token[], i: number): { out: string; next: number } {
	const close = '/table';
	let rows: string[][] = [];
	let j = i + 1;
	while (j < tokens.length && tokens[j].name !== close) {
		const t = tokens[j];
		if (t.kind === 'tag' && t.name === 'tr') {
			const row: string[] = [];
			let k = j + 1;
			while (k < tokens.length && tokens[k].name !== '/tr') {
				const c = tokens[k];
				if (c.kind === 'tag' && (c.name === 'td' || c.name === 'th')) {
					const inner = convert(tokens, k + 1);
					row.push(inner.out.trim());
					k = inner.next + 1;
				} else {
					k++;
				}
			}
			if (row.length) rows.push(row);
			j = k < tokens.length ? k + 1 : k;
		} else {
			j++;
		}
	}
	const next = j < tokens.length ? j + 1 : j;
	if (rows.length === 0) return { out: '', next };
	// Pad ragged rows to the widest one so the GFM output stays rectangular.
	const width = Math.max(...rows.map((r) => r.length));
	const pad = (r: string[]) => [...r, ...Array(width - r.length).fill('')];
	const esc = (s: string) => s.replace(/\|/g, '\\|');
	const line = (cells: string[]) => '| ' + cells.map(esc).join(' | ') + ' |';
	const out = '\n' + line(pad(rows[0])) + '\n' + line(pad(rows[0].map(() => '---'))) + (rows.length > 1 ? '\n' + rows.slice(1).map((r) => line(pad(r))).join('\n') : '') + '\n';
	return { out, next };
}

function renderPre(tokens: Token[], i: number): { out: string; next: number } {
	let codeText = '';
	let lang = '';
	let j = i + 1;
	while (j < tokens.length && tokens[j].name !== '/pre') {
		const t = tokens[j];
		if (t.kind === 'text') codeText += t.text!;
		else if (t.name === 'code') lang = /(?:^|\s)language-([\w-]+)/.exec(t.attrs['class'] ?? '')?.[1] ?? '';
		j++;
	}
	return { out: '\n```' + lang + '\n' + decode(codeText).replace(/\n$/, '') + '\n```\n', next: j < tokens.length ? j + 1 : j };
}

function renderList(tokens: Token[], i: number, ordered: boolean): { out: string; next: number } {
	const close = '/' + tokens[i].name;
	let out = '';
	let n = 1;
	let j = i + 1;
	while (j < tokens.length && tokens[j].name !== close) {
		const t = tokens[j];
		if (t.kind === 'tag' && t.name === 'li') {
			const checked = t.attrs['data-checked'];
			const inner = convert(tokens, j + 1);
			const itemMd = inner.out.trim();
			j = inner.next + 1; // consume </li>
			const marker = checked !== undefined ? `- [${checked === 'true' ? 'x' : ' '}] ` : ordered ? `${n++}. ` : '- ';
			const lines = itemMd.split('\n');
			out += [marker + lines[0], ...lines.slice(1).map((l) => '  ' + l)].join('\n') + '\n';
		} else if (t.kind === 'tag' && (t.name === 'ul' || t.name === 'ol')) {
			const r = renderList(tokens, j, t.name === 'ol');
			out += r.out;
			j = r.next;
		} else {
			j++;
		}
	}
	return { out, next: j < tokens.length ? j + 1 : j };
}

function renderElement(name: string, attrs: Record<string, string>, content: string): string {
	switch (name) {
		case 'h1': case 'h2': case 'h3': case 'h4': case 'h5': case 'h6':
			return '\n' + '#'.repeat(Number(name[1])) + ' ' + content.trim() + '\n';
		case 'p':
			return '\n' + content.trim() + '\n';
		case 'strong': case 'b':
			return '**' + content + '**';
		case 'em': case 'i':
			return '*' + content + '*';
		case 's': case 'del': case 'strike':
			return '~~' + content + '~~';
		case 'code':
			return '`' + content + '`';
		case 'a':
			return attrs.href ? `[${content.trim() || attrs.href}](${attrs.href})` : content;
		case 'span':
			return 'data-mention' in attrs ? (attrs['data-title'] ? `[[${attrs['data-title']}]]` : '') : content;
		case 'blockquote':
			return '\n' + content.trim().split('\n').map((l) => '> ' + l).join('\n') + '\n';
		case 'div':
			if ('data-database' in attrs) return renderTable(attrs['data-database']);
			if ('data-bookmark' in attrs) return renderBookmark(attrs);
			if ('data-page-embed' in attrs) return renderPageEmbed(attrs);
			return '\n' + content.trim() + '\n';
		case 'li':
			return content;
		default:
			return content;
	}
}

function renderTable(jsonStr: string): string {
	try {
		const d = JSON.parse(decode(jsonStr));
		const cols: { id: string; name: string }[] = d.columns ?? [];
		const rows: { cells: Record<string, string | boolean> }[] = d.rows ?? [];
		if (cols.length === 0) return '';
		const header = '| ' + cols.map((c) => c.name).join(' | ') + ' |';
		const sep = '| ' + cols.map(() => '---').join(' | ') + ' |';
		const body = rows.map((r) => '| ' + cols.map((c) => String(r.cells?.[c.id] ?? '')).join(' | ') + ' |').join('\n');
		return '\n\n' + header + '\n' + sep + (body ? '\n' + body : '') + '\n\n';
	} catch {
		return '';
	}
}

function renderBookmark(attrs: Record<string, string>): string {
	const url = attrs['data-url'] ?? '';
	const title = attrs['data-title'] ?? '';
	if (!url) return '';
	return `\n\n[${title || url}](<${url}>)\n\n`;
}

function renderPageEmbed(attrs: Record<string, string>): string {
	const docId = attrs['data-doc-id'] ?? '';
	const title = attrs['data-title'] ?? '';
	if (!docId) return '';
	return `\n\n[${title || 'Open page'}](/doc/${docId})\n\n`;
}

/** Convert an HTML string to Markdown. */
export function htmlToMarkdown(html: string): string {
	return convert(tokenize(html), 0).out;
}

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
