import { invoke, isTauri } from '$lib/backend.js';
import { open, save } from '@tauri-apps/plugin-dialog';
import { join } from '@tauri-apps/api/path';
import { markdownToJson, jsonToMarkdown } from '@enclave/editor';
import type { Document, Block } from '@enclave/ui';

interface Frontmatter {
	title: string;
	tags: string[];
	body: string;
}

/** Split Obsidian/AFFiNE-style YAML frontmatter (title + tags) from the body. */
export function parseFrontmatter(md: string): Frontmatter {
	const m = md.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
	if (!m) return { title: '', tags: [], body: md };
	const fm = m[1];
	const title = fm.match(/^title:\s*(.+)$/m)?.[1]?.trim() ?? '';
	const tagsLine = fm.match(/^tags:\s*(.+)$/m)?.[1]?.trim() ?? '';
	let tags: string[] = [];
	if (tagsLine) {
		if (tagsLine.startsWith('[')) {
			tags = [...tagsLine.matchAll(/['"`]?([^,'"`\]]+)['"`]?/g)].map((x) => x[1].trim()).filter(Boolean);
		} else {
			tags = tagsLine.split(',').map((t) => t.trim().replace(/^#/, '')).filter(Boolean);
		}
	}
	return { title, tags, body: md.slice(m[0].length) };
}

/** Create a document from markdown source (shared by the Tauri + web paths). */
async function importOne(md: string, fallbackName: string): Promise<void> {
	const { title, tags, body } = parseFrontmatter(md);
	const doc = await invoke<Document>('create_document', { title: title || fallbackName });
	const json = await markdownToJson(body);
	await invoke('upsert_block', {
		id: `${doc.id}-content`,
		documentId: doc.id,
		blockType: 'doc',
		content: json,
		sortOrder: 0,
	});
	if (tags.length) {
		await invoke('upsert_block', {
			id: `${doc.id}-tags`,
			documentId: doc.id,
			blockType: 'tags',
			content: { tags },
			sortOrder: 2,
		});
	}
}

/** Pick .md files (native dialog in Tauri, <input type=file> in the browser)
 *  and import each as a new document. Returns import count. */
export async function importMarkdownFiles(onImported?: (count: number) => void): Promise<number> {
	let count = 0;
	if (isTauri()) {
		const paths = await open({ multiple: true, filters: [{ name: 'Markdown', extensions: ['md'] }] });
		if (!paths) return 0;
		const list = Array.isArray(paths) ? paths : [paths];
		for (const p of list) {
			const md = await invoke<string>('import_markdown', { path: p });
			const fallback = p.split(/[\\/]/).pop()?.replace(/\.md$/, '') || 'Imported';
			await importOne(md, fallback);
			count++;
		}
		onImported?.(count);
		return count;
	}
	const files = await pickMarkdownFiles();
	if (!files?.length) return 0;
	for (const f of files) {
		await importOne(await f.text(), f.name.replace(/\.md$/i, '') || 'Imported');
		count++;
	}
	onImported?.(count);
	return count;
}

function pickMarkdownFiles(): Promise<File[] | null> {
	return new Promise((resolve) => {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = '.md,text/markdown';
		input.multiple = true;
		input.onchange = () => resolve(input.files ? Array.from(input.files) : null);
		input.click();
	});
}

/** Export every document to a user-chosen folder as .md with frontmatter. */
export async function exportVaultAsMarkdown(): Promise<number> {
	if (!isTauri()) {
		// Folder export needs a directory picker + native writes; per-page
		// Markdown/HTML export (below) works in the browser instead.
		alert('Exporting the whole vault to a folder is not available in the web build yet — use per-page Markdown/HTML export.');
		return 0;
	}
	const dir = await open({ directory: true });
	if (!dir) return 0;
	const docs = await invoke<Document[]>('get_document_list');
	let count = 0;
	for (const d of docs) {
		const blocks = await invoke<Block[]>('get_blocks', { documentId: d.id });
		const contentBlock = blocks.find((b) => b.type === 'doc');
		const tagsBlock = blocks.find((b) => b.type === 'tags');
		const md = contentBlock?.content ? await jsonToMarkdown(contentBlock.content as never) : '';
		const tags: string[] = (tagsBlock?.content as { tags?: string[] } | undefined)?.tags ?? [];
		const fm = [
			'---',
			`title: ${d.title ?? ''}`,
			...(tags.length ? [`tags: [${tags.join(', ')}]`] : []),
			'---',
			'',
		].join('\n');
		const safe = (d.title || 'untitled').replace(/[\\/:*?"<>|]/g, '_');
		await invoke('write_file', {
			path: await join(dir, `${safe}.md`),
			data: Array.from(new TextEncoder().encode(fm + md)),
		});
		count++;
	}
	return count;
}

/** Export the current page to a user-chosen location (browser: download). */
export async function exportMarkdownDialog(title: string, md: string): Promise<boolean> {
	if (isTauri()) {
		const path = await save({
			defaultPath: `${title || 'untitled'}.md`,
			filters: [{ name: 'Markdown', extensions: ['md'] }],
		});
		if (!path) return false;
		await invoke('write_file', { path, data: Array.from(new TextEncoder().encode(md)) });
		return true;
	}
	downloadBlob(`${title || 'untitled'}.md`, new Blob([md], { type: 'text/markdown' }));
	return true;
}

/** Export the current page as a self-contained HTML file (browser: download). */
export async function exportHtmlDialog(title: string, html: string): Promise<boolean> {
	if (isTauri()) {
		const path = await save({
			defaultPath: `${title || 'untitled'}.html`,
			filters: [{ name: 'HTML', extensions: ['html'] }],
		});
		if (!path) return false;
		const safe = (title || 'Untitled').replace(/[<>&"]/g, (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;' })[c]!);
		const doc =
			'<!DOCTYPE html><html><head><meta charset="utf-8"><title>' + safe + '</title>' +
			'<style>body{max-width:720px;margin:40px auto;padding:0 20px;font-family:system-ui,sans-serif;line-height:1.7;color:#1a1a1a}pre{background:#f4f4f4;padding:12px;border-radius:6px;overflow-x:auto}blockquote{border-left:3px solid #999;padding-left:12px;color:#555;margin-left:0}img{max-width:100%}</style>' +
			'</head><body>' + html + '</body></html>';
		await invoke('write_file', { path, data: Array.from(new TextEncoder().encode(doc)) });
		return true;
	}
	const safe = (title || 'Untitled').replace(/[<>&"]/g, (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;' })[c]!);
	const doc =
		'<!DOCTYPE html><html><head><meta charset="utf-8"><title>' + safe + '</title>' +
		'<style>body{max-width:720px;margin:40px auto;padding:0 20px;font-family:system-ui,sans-serif;line-height:1.7;color:#1a1a1a}pre{background:#f4f4f4;padding:12px;border-radius:6px;overflow-x:auto}blockquote{border-left:3px solid #999;padding-left:12px;color:#555;margin-left:0}img{max-width:100%}</style>' +
		'</head><body>' + html + '</body></html>';
	downloadBlob(`${title || 'untitled'}.html`, new Blob([doc], { type: 'text/html' }));
	return true;
}

/** Browser download of a Blob (native file-save dialog is Tauri-only). */
function downloadBlob(name: string, blob: Blob): void {
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = name;
	a.click();
	URL.revokeObjectURL(url);
}
