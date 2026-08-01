// Page templates — plain ProseMirror JSON content blocks (no database
// nodes, so any doc can hold them). Written from scratch, AFFiNE-inspired.

export interface Template {
	id: string;
	name: string;
	icon: string;
	content: object;
}

const p = (text: string): object => ({ type: 'paragraph', content: [{ type: 'text', text }] });
const h1 = (text: string): object => ({ type: 'heading', attrs: { level: 1 }, content: [{ type: 'text', text }] });
const h2 = (text: string): object => ({ type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text }] });
const bullet = (text: string): object => ({
	type: 'bulletList',
	content: [{ type: 'listItem', content: [p(text)] }],
});
const task = (text: string): object => ({
	type: 'taskList',
	content: [{ type: 'taskItem', attrs: { checked: false }, content: [p(text)] }],
});

export const templates: Template[] = [
	{
		id: 'meeting',
		name: 'Meeting Notes',
		icon: '📋',
		content: {
			type: 'doc',
			content: [
				h1('Meeting Notes'),
				p('Date:  •  Attendees: '),
				h2('Agenda'),
				bullet(''),
				bullet(''),
				h2('Discussion'),
				p(''),
				h2('Action Items'),
				task(''),
				task(''),
			],
		},
	},
	{
		id: 'project',
		name: 'Project Plan',
		icon: '🚀',
		content: {
			type: 'doc',
			content: [
				h1('Project Plan'),
				p('Goal: '),
				h2('Scope'),
				bullet(''),
				bullet(''),
				h2('Milestones'),
				bullet(''),
				bullet(''),
				h2('Risks'),
				p(''),
			],
		},
	},
	{
		id: 'journal',
		name: 'Daily Journal',
		icon: '✍️',
		content: {
			type: 'doc',
			content: [
				h1('Journal'),
				p(''),
				h2('What went well?'),
				p(''),
				h2('What could be better?'),
				p(''),
				h2('Tomorrow'),
				task(''),
			],
		},
	},
	{
		id: 'book',
		name: 'Book Notes',
		icon: '📚',
		content: {
			type: 'doc',
			content: [
				h1('Book Notes'),
				p('Author:  •  Rating: '),
				h2('Summary'),
				p(''),
				h2('Key Quotes'),
				bullet(''),
				bullet(''),
				h2('My Takeaways'),
				p(''),
			],
		},
	},
];
