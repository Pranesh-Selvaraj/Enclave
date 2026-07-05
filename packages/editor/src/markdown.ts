// Markdown bridge — converts between HTML (TipTap's lingua franca) and Markdown.
// turndown (HTML→MD), markdown-it (MD→HTML). Both handle the full CommonMark spec.

import TurndownService from 'turndown';
import MarkdownIt from 'markdown-it';

const turndown = new TurndownService({
	headingStyle: 'atx',
	codeBlockStyle: 'fenced',
	emDelimiter: '*',
	bulletListMarker: '-',
});

const md = new MarkdownIt({ html: false, breaks: true, linkify: false });

/** Convert an HTML string to Markdown. */
export function htmlToMarkdown(html: string): string {
	return turndown.turndown(html);
}

/** Convert a Markdown string to HTML. */
export function markdownToHtml(markdown: string): string {
	return md.render(markdown);
}
