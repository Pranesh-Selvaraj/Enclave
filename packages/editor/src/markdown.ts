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
