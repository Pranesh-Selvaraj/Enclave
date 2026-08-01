<script lang="ts">
	// Compact read-only-ish cell renderer for non-table views. With `onSet`
	// the checkbox becomes editable (kanban); display views pass no onSet.
	import type { DBColumn, DBRow } from '../extensions/database.js';

	let {
		col,
		row,
		onSet,
		titles,
	}: {
		col: DBColumn;
		row: DBRow;
		onSet?: (value: string | boolean | string[]) => void;
		titles?: Map<string, string>;
	} = $props();

	const TAG_COLORS = ['#e5484d', '#f0a020', '#46a758', '#2f9e9e', '#3b82f6', '#8b5cf6', '#d6409f', '#f0b429'];

	function tagColor(name: string): string {
		let h = 0;
		for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
		return TAG_COLORS[h % TAG_COLORS.length];
	}

	function v(): string | boolean | string[] {
		if (col.type === 'createdAt') return row.createdAt ?? '';
		if (col.type === 'updatedAt') return row.updatedAt ?? '';
		const val = row.cells[col.id];
		if (col.type === 'checkbox') return val === true;
		if (col.type === 'multiSelect') return Array.isArray(val) ? val : val ? [String(val)] : [];
		return typeof val === 'string' ? val : '';
	}

	function linkHref(val: string): string {
		return /^[a-z]+:/i.test(val) ? val : 'https://' + val;
	}
</script>

{#if col.type === 'checkbox'}
	{#if onSet}
		<input type="checkbox" checked={v() === true} onclick={(e) => onSet((e.currentTarget as HTMLInputElement).checked)} />
	{:else}
		<span class="cell muted">{v() === true ? '✓' : '—'}</span>
	{/if}
{:else if col.type === 'select' || col.type === 'multiSelect'}
	<span class="cell tags">
		{#each (col.type === 'multiSelect' ? v() as string[] : v() ? [String(v())] : []) as t}
			<span class="chip" style="background:{tagColor(t)}22; color:{tagColor(t)};">{t}</span>
		{/each}
	</span>
{:else if col.type === 'progress'}
	<span class="cell barwrap">
		<span class="bar"><span class="bar-fill" style="width:{Number(v()) || 0}%"></span></span>
		<span class="muted">{Number(v()) || 0}%</span>
	</span>
{:else if col.type === 'url' && v()}
	<span class="cell text"><a class="link" href={linkHref(String(v()))} target="_blank" rel="noopener noreferrer">{String(v())}</a></span>
{:else if col.type === 'email' && v()}
	<span class="cell text"><a class="link" href="mailto:{String(v())}">{String(v())}</a></span>
{:else if col.type === 'relation'}
	<span class="cell text">
		{#if v()}
			<a class="link" href="/{String(v())}" title="Open page">{titles?.get(String(v())) ?? 'Untitled'}</a>
		{:else}
			<span class="muted">—</span>
		{/if}
	</span>
{:else if col.type === 'createdAt' || col.type === 'updatedAt'}
	<span class="cell muted">{String(v()).slice(0, 10) || '—'}</span>
{:else}
	<span class="cell text">{String(v()) || '—'}</span>
{/if}

<style>
	.cell {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}

	.tags {
		flex-wrap: wrap;
	}

	.chip {
		border-radius: 999px;
		padding: 1px 8px;
		font-size: 11px;
		font-weight: 500;
		white-space: nowrap;
	}

	.text {
		color: var(--color-text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.muted {
		color: var(--color-text-muted);
		font-size: 12px;
	}

	.link {
		color: #7c6cf0;
		text-decoration: none;
	}

	.link:hover {
		text-decoration: underline;
	}

	.barwrap {
		white-space: nowrap;
	}

	.bar {
		display: inline-block;
		width: 80px;
		height: 6px;
		border-radius: 3px;
		background: var(--color-hover);
		overflow: hidden;
	}

	.bar-fill {
		display: block;
		height: 100%;
		background: #7c6cf0;
		border-radius: 3px;
	}
</style>
