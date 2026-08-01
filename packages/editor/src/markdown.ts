import TurndownService from 'turndown';

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
