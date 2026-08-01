import { invoke } from '@tauri-apps/api/core';
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

/** Pick .md files and import each as a new document. Returns import count. */
export async function importMarkdownFiles(onImported?: (count: number) => void): Promise<number> {
	const paths = await open({ multiple: true, filters: [{ name: 'Markdown', extensions: ['md'] }] });
	if (!paths) return 0;
	const list = Array.isArray(paths) ? paths : [paths];
	let count = 0;
	for (const p of list) {
		const md = await invoke<string>('import_markdown', { path: p });
		const { title, tags, body } = parseFrontmatter(md);
		const fallback = p.split(/[\\/]/).pop()?.replace(/\.md$/, '') || 'Imported';
		const doc = await invoke<Document>('create_document', { title: title || fallback });
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
		count++;
	}
	onImported?.(count);
	return count;
}

/** Export every document to a user-chosen folder as .md with frontmatter. */
export async function exportVaultAsMarkdown(): Promise<number> {
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

/** Export the current page to a user-chosen location. */
export async function exportMarkdownDialog(title: string, md: string): Promise<boolean> {
	const path = await save({
		defaultPath: `${title || 'untitled'}.md`,
		filters: [{ name: 'Markdown', extensions: ['md'] }],
	});
	if (!path) return false;
	await invoke('write_file', { path, data: Array.from(new TextEncoder().encode(md)) });
	return true;
}
