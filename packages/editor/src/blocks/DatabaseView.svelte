<script lang="ts">
	import type { DBColumn, DBRow } from '../extensions/database.js';

	let {
		data,
		onData,
	}: {
		data: string;
		onData: (json: string) => void;
	} = $props();

	const TYPES = ['text', 'number', 'checkbox', 'date'] as const;

	let columns = $state<DBColumn[]>([]);
	let rows = $state<DBRow[]>([]);
	let sort = $state<{ colId: string; dir: 'asc' | 'desc' } | null>(null);
	let filters = $state<Record<string, string>>({});
	let filterOpen = $state(false);

	function uid(): string {
		return Math.random().toString(36).slice(2, 10);
	}

	function emit() {
		onData(JSON.stringify({ columns, rows }));
	}

	// Sync from node attribute (load, undo/redo) — skip when it's our own echo.
	$effect(() => {
		let parsed: { columns?: DBColumn[]; rows?: DBRow[] };
		try {
			parsed = JSON.parse(data);
		} catch {
			parsed = {};
		}
		const d = parsed as { columns?: DBColumn[]; rows?: DBRow[] };
		if (JSON.stringify({ columns, rows }) !== JSON.stringify({ columns: d.columns ?? [], rows: d.rows ?? [] })) {
			columns = (d.columns ?? []).map((c) => ({ ...c }));
			rows = (d.rows ?? []).map((r) => ({ ...r, cells: { ...r.cells } }));
		}
	});

	function cellValue(row: DBRow, col: DBColumn): string | boolean {
		const v = row.cells[col.id];
		if (col.type === 'checkbox') return v === true;
		return typeof v === 'string' ? v : '';
	}

	function setCell(rowId: string, col: DBColumn, value: string | boolean) {
		rows = rows.map((r) =>
			r.id === rowId ? { ...r, cells: { ...r.cells, [col.id]: col.type === 'checkbox' ? !!value : String(value) } } : r
		);
		emit();
	}

	function addColumn() {
		const col: DBColumn = { id: uid(), name: 'Column', type: 'text' };
		columns = [...columns, col];
		rows = rows.map((r) => ({ ...r, cells: { ...r.cells } }));
		emit();
	}

	function removeColumn(colId: string) {
		columns = columns.filter((c) => c.id !== colId);
		rows = rows.map((r) => {
			const cells = { ...r.cells };
			delete cells[colId];
			return { ...r, cells };
		});
		emit();
	}

	function renameColumn(colId: string, name: string) {
		columns = columns.map((c) => (c.id === colId ? { ...c, name } : c));
		emit();
	}

	function changeType(colId: string, type: DBColumn['type']) {
		columns = columns.map((c) => (c.id === colId ? { ...c, type } : c));
		emit();
	}

	function addRow() {
		rows = [...rows, { id: uid(), cells: {} }];
		emit();
	}

	function removeRow(rowId: string) {
		rows = rows.filter((r) => r.id !== rowId);
		emit();
	}

	function cycleSort(colId: string) {
		if (!sort || sort.colId !== colId) sort = { colId, dir: 'asc' };
		else if (sort.dir === 'asc') sort = { colId, dir: 'desc' };
		else sort = null;
	}

	function compare(a: string | boolean, b: string | boolean, col: DBColumn): number {
		if (col.type === 'number') return (parseFloat(String(a)) || 0) - (parseFloat(String(b)) || 0);
		if (col.type === 'date') return (Date.parse(String(a)) || 0) - (Date.parse(String(b)) || 0);
		return String(a).localeCompare(String(b));
	}

	let visibleRows = $derived.by(() => {
		let out = rows;
		const active = Object.entries(filters).filter(([, v]) => v.trim() !== '');
		if (active.length > 0) {
			out = out.filter((r) =>
				active.every(([colId, q]) => {
					const col = columns.find((c) => c.id === colId);
					return String(r.cells[colId] ?? '').toLowerCase().includes(q.toLowerCase());
				})
			);
		}
		if (sort) {
			const col = columns.find((c) => c.id === sort!.colId);
			if (col) {
				const dir = sort!.dir === 'asc' ? 1 : -1;
				out = [...out].sort((a, b) => compare(a.cells[col.id] ?? '', b.cells[col.id] ?? '', col) * dir);
			}
		}
		return out;
	});

	function sortLabel(colId: string): string {
		if (!sort || sort.colId !== colId) return '';
		return sort.dir === 'asc' ? '↑' : '↓';
	}
</script>

<div class="db" data-database>
	<div class="db-header-row">
		{#each columns as col (col.id)}
			<div class="db-header">
				<input
					class="db-header-name"
					value={col.name}
					aria-label="Column name"
					oninput={(e) => renameColumn(col.id, (e.currentTarget as HTMLInputElement).value)}
				/>
				<select
					class="db-header-type"
					value={col.type}
					aria-label="Column type"
					onchange={(e) => changeType(col.id, (e.currentTarget as HTMLSelectElement).value as DBColumn['type'])}
				>
					{#each TYPES as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
				<button class="db-header-sort" aria-label="Sort column" onclick={() => cycleSort(col.id)}>
					{sortLabel(col.id)}
				</button>
				<button class="db-remove" aria-label="Delete column" onclick={() => removeColumn(col.id)}>✕</button>
			</div>
		{/each}
		<button class="db-add-col" onclick={addColumn}>+ Column</button>
	</div>

	{#if filterOpen}
		<div class="db-filter-row">
			{#each columns as col (col.id)}
				<input
					class="db-filter-input"
					placeholder="Filter {col.name}…"
					value={filters[col.id] ?? ''}
					oninput={(e) => (filters = { ...filters, [col.id]: (e.currentTarget as HTMLInputElement).value })}
				/>
			{/each}
		</div>
	{/if}

	{#each visibleRows as row (row.id)}
		<div class="db-row">
			{#each columns as col (col.id)}
				{#if col.type === 'checkbox'}
					<div class="db-cell db-cell-check">
						<input type="checkbox" checked={cellValue(row, col) === true} onclick={(e) => setCell(row.id, col, (e.currentTarget as HTMLInputElement).checked)} />
					</div>
				{:else}
					<input
						class="db-cell"
						type={col.type === 'number' ? 'number' : col.type === 'date' ? 'date' : 'text'}
						value={String(cellValue(row, col))}
						oninput={(e) => setCell(row.id, col, (e.currentTarget as HTMLInputElement).value)}
					/>
				{/if}
			{/each}
			<button class="db-remove" aria-label="Delete row" onclick={() => removeRow(row.id)}>✕</button>
		</div>
	{/each}

	<div class="db-footer">
		<button class="db-add-row" onclick={addRow}>+ Row</button>
		<button class="db-filter-toggle" onclick={() => (filterOpen = !filterOpen)}>Filter</button>
	</div>
</div>

<style>
	.db {
		border: 1px solid var(--color-border);
		border-radius: 10px;
		background: var(--color-surface);
		margin: 8px 0;
		overflow: hidden;
	}

	.db-header-row {
		display: flex;
		align-items: stretch;
		background: var(--color-hover);
		border-bottom: 1px solid var(--color-border);
		overflow-x: auto;
	}

	.db-header {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 6px 8px;
		border-right: 1px solid var(--color-border);
		min-width: 160px;
	}

	.db-header-name {
		flex: 1;
		min-width: 0;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		font-weight: 600;
		outline: none;
	}

	.db-header-type {
		border: 1px solid var(--color-border);
		border-radius: 6px;
		background: var(--color-bg);
		color: var(--color-text-muted);
		font-size: 11px;
		padding: 2px 4px;
	}

	.db-header-sort {
		border: none;
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 12px;
		width: 18px;
	}

	.db-remove {
		border: none;
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 11px;
		padding: 2px 4px;
		border-radius: 4px;
		opacity: 0;
	}

	.db-header:hover .db-remove,
	.db-row:hover .db-remove {
		opacity: 1;
	}

	.db-remove:hover {
		background: var(--color-hover);
		color: #e5484d;
	}

	.db-add-col {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		cursor: pointer;
		padding: 0 12px;
		white-space: nowrap;
	}

	.db-filter-row {
		display: flex;
		align-items: center;
		gap: 0;
		border-bottom: 1px solid var(--color-border);
		overflow-x: auto;
	}

	.db-filter-input {
		border: none;
		border-right: 1px solid var(--color-border);
		background: none;
		color: var(--color-text);
		font-size: 12px;
		padding: 5px 8px;
		min-width: 160px;
		outline: none;
	}

	.db-row {
		display: flex;
		align-items: stretch;
		border-bottom: 1px solid var(--color-border);
		overflow-x: auto;
	}

	.db-row:last-of-type {
		border-bottom: none;
	}

	.db-cell {
		border: none;
		border-right: 1px solid var(--color-border);
		background: none;
		color: var(--color-text);
		font-size: 13px;
		padding: 6px 8px;
		min-width: 160px;
		outline: none;
	}

	.db-cell:focus {
		background: var(--color-hover);
	}

	.db-cell-check {
		display: flex;
		align-items: center;
		padding: 6px 8px;
	}

	.db-footer {
		display: flex;
		gap: 8px;
		padding: 6px 8px;
	}

	.db-add-row,
	.db-filter-toggle {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		cursor: pointer;
		padding: 2px 6px;
		border-radius: 4px;
	}

	.db-add-row:hover,
	.db-filter-toggle:hover {
		background: var(--color-hover);
		color: var(--color-text);
	}
</style>
