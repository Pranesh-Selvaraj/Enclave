<script lang="ts">
	import type { DBColumn, DBRow } from '../extensions/database.js';
	import DBCell from './DBCell.svelte';

	let {
		data,
		readOnly = false,
		onData,
	}: {
		data: string;
		readOnly?: boolean;
		onData: (json: string) => void;
	} = $props();

	const TYPES = [
		'text',
		'number',
		'checkbox',
		'date',
		'select',
		'multiSelect',
		'url',
		'email',
		'progress',
		'createdAt',
		'updatedAt',
	] as const;
	const TAG_COLORS = ['#e5484d', '#f0a020', '#46a758', '#2f9e9e', '#3b82f6', '#8b5cf6', '#d6409f', '#f0b429'];
	const VIEWS = ['table', 'kanban', 'list', 'gallery', 'timeline'] as const;
	type View = (typeof VIEWS)[number];

	let columns = $state<DBColumn[]>([]);
	let rows = $state<DBRow[]>([]);
	let view = $state<View>('table');
	let groupBy = $state<string | null>(null);
	let sort = $state<{ colId: string; dir: 'asc' | 'desc' } | null>(null);
	let filters = $state<Record<string, string>>({});
	let filterOpen = $state(false);
	let menu = $state<{ colId: string; rowId: string; x: number; y: number } | null>(null);
	let newOption = $state('');

	function uid(): string {
		return Math.random().toString(36).slice(2, 10);
	}

	function emit() {
		if (readOnly) return;
		onData(JSON.stringify({ columns, rows, view, groupBy, sort, filters }));
	}

	// Sync from node attribute (load, undo/redo, linked-db mirror) — skip when
	// it's our own echo.
	$effect(() => {
		let parsed: Partial<{ columns: DBColumn[]; rows: DBRow[]; view: View; groupBy: string | null; sort: typeof sort; filters: Record<string, string> }>;
		try {
			parsed = JSON.parse(data);
		} catch {
			parsed = {};
		}
		const d = parsed;
		const sig = JSON.stringify({ columns, rows, view, groupBy, sort, filters });
		const incoming = JSON.stringify({
			columns: d.columns ?? [],
			rows: d.rows ?? [],
			view: d.view ?? 'table',
			groupBy: d.groupBy ?? null,
			sort: d.sort ?? null,
			filters: d.filters ?? {},
		});
		if (sig !== incoming) {
			columns = (d.columns ?? []).map((c) => ({ ...c, options: [...(c.options ?? [])] }));
			rows = (d.rows ?? []).map((r) => ({ ...r, cells: { ...r.cells } }));
			view = d.view ?? 'table';
			groupBy = d.groupBy ?? null;
			sort = d.sort ?? null;
			filters = { ...(d.filters ?? {}) };
		}
	});

	function cellValue(row: DBRow, col: DBColumn): string | boolean | string[] {
		if (col.type === 'createdAt') return row.createdAt ?? '';
		if (col.type === 'updatedAt') return row.updatedAt ?? '';
		const v = row.cells[col.id];
		if (col.type === 'checkbox') return v === true;
		if (col.type === 'multiSelect') return Array.isArray(v) ? v : v ? [String(v)] : [];
		return typeof v === 'string' ? v : '';
	}

	function setCell(rowId: string, col: DBColumn, value: string | boolean | string[]) {
		rows = rows.map((r) => {
			if (r.id !== rowId) return r;
			const cells = { ...r.cells };
			if (col.type === 'checkbox') cells[col.id] = !!value;
			else if (col.type === 'multiSelect') cells[col.id] = Array.isArray(value) ? value : [];
			else cells[col.id] = String(value);
			return { ...r, cells, updatedAt: new Date().toISOString() };
		});
		emit();
	}

	function tagColor(name: string): string {
		let h = 0;
		for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
		return TAG_COLORS[h % TAG_COLORS.length];
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
		if (groupBy === colId) groupBy = null;
		emit();
	}

	function renameColumn(colId: string, name: string) {
		columns = columns.map((c) => (c.id === colId ? { ...c, name } : c));
		emit();
	}

	function changeType(colId: string, type: DBColumn['type']) {
		columns = columns.map((c) => (c.id === colId ? { ...c, type } : c));
		if (type !== 'select') {
			// multiSelect values are arrays; other types are scalars
			rows = rows.map((r) => {
				const cells = { ...r.cells };
				const v = cells[colId];
				if (type === 'multiSelect') cells[colId] = typeof v === 'string' && v ? [v] : [];
				else if (Array.isArray(v)) cells[colId] = v[0] ?? '';
				return { ...r, cells };
			});
		}
		if (groupBy === colId && type !== 'select') groupBy = null;
		emit();
	}

	function addRow() {
		const now = new Date().toISOString();
		rows = [...rows, { id: uid(), cells: {}, createdAt: now, updatedAt: now }];
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
		emit();
	}

	function compare(a: string | boolean | string[], b: string | boolean | string[], col: DBColumn): number {
		const av = Array.isArray(a) ? a.join(', ') : String(a);
		const bv = Array.isArray(b) ? b.join(', ') : String(b);
		if (col.type === 'number' || col.type === 'progress') return (parseFloat(av) || 0) - (parseFloat(bv) || 0);
		if (col.type === 'date' || col.type === 'createdAt' || col.type === 'updatedAt')
			return (Date.parse(av) || 0) - (Date.parse(bv) || 0);
		return av.localeCompare(bv);
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

	function openMenu(e: MouseEvent | HTMLElement, col: DBColumn, rowId: string) {
		if (readOnly) return;
		const el = e instanceof HTMLElement ? e : (e.currentTarget as HTMLElement);
		if (!col.options?.length && col.type === 'select') return;
		const rect = el.getBoundingClientRect();
		menu = { colId: col.id, rowId, x: rect.left, y: rect.bottom + 4 };
	}

	function menuCol(): DBColumn | null {
		return menu ? columns.find((c) => c.id === menu!.colId) ?? null : null;
	}

	function toggleOption(colId: string, rowId: string, opt: string) {
		const col = columns.find((c) => c.id === colId);
		if (!col) return;
		if (col.type === 'select') {
			setCell(rowId, col, opt);
			menu = null;
		} else {
			const cur = cellValue(rows.find((r) => r.id === rowId)!, col);
			const arr = Array.isArray(cur) ? [...cur] : [];
			const i = arr.indexOf(opt);
			if (i >= 0) arr.splice(i, 1);
			else arr.push(opt);
			setCell(rowId, col, arr);
		}
	}

	function removeOption(colId: string, opt: string) {
		columns = columns.map((c) => (c.id === colId ? { ...c, options: (c.options ?? []).filter((o) => o !== opt) } : c));
		rows = rows.map((r) => {
			const v = r.cells[colId];
			if (v === opt || (Array.isArray(v) && v.includes(opt))) {
				const cells = { ...r.cells };
				if (Array.isArray(v)) cells[colId] = v.filter((x) => x !== opt);
				else cells[colId] = '';
				return { ...r, cells };
			}
			return r;
		});
		emit();
	}

	function addOption(colId: string, opt: string) {
		const name = opt.trim();
		if (!name) return;
		columns = columns.map((c) => (c.id === colId ? { ...c, options: [...(c.options ?? []), name] } : c));
		newOption = '';
		// Auto-select the new option like Notion/AFFiNE
		if (menu) {
			const col = columns.find((c) => c.id === colId);
			if (col) toggleOption(colId, menu.rowId, name);
		} else {
			emit();
		}
	}

	// ── Kanban ────────────────────────────────────────────────────────────────

	let groupCol = $derived(columns.find((c) => c.id === groupBy && c.type === 'select') ?? null);

	let kanbanGroups = $derived.by(() => {
		const col = groupCol;
		const groups: { value: string | null; label: string; rows: DBRow[] }[] = [
			{ value: null, label: 'No status', rows: [] },
		];
		if (!col) return groups;
		for (const opt of col.options ?? []) groups.push({ value: opt, label: opt, rows: [] });
		for (const r of visibleRows) {
			const v = r.cells[col.id];
			const label = typeof v === 'string' ? v : '';
			const g = groups.find((g) => g.value === label) ?? groups[0];
			g.rows.push(r);
		}
		return groups;
	});

	function moveCard(e: DragEvent, value: string | null) {
		e.preventDefault();
		const rowId = e.dataTransfer?.getData('text/row');
		if (!rowId || !groupCol) return;
		setCell(rowId, groupCol, value ?? '');
	}

	function setView(v: View) {
		view = v;
		if (v === 'kanban' && !groupBy) {
			const sel = columns.find((c) => c.type === 'select');
			groupBy = sel?.id ?? null;
		}
		emit();
	}

	// ── List / Gallery / Timeline ─────────────────────────────────────────────

	let timelineCol = $derived(columns.find((c) => c.type === 'date') ?? null);

	let timelineGroups = $derived.by(() => {
		const get = (r: DBRow): string => {
			if (timelineCol) return String(r.cells[timelineCol.id] ?? '');
			return r.updatedAt ?? r.createdAt ?? '';
		};
		const withDate = visibleRows.filter((r) => get(r));
		const rest = visibleRows.filter((r) => !get(r));
		const groups: { key: string; items: { row: DBRow; date: string }[] }[] = [];
		for (const r of withDate) {
			const date = get(r);
			const key = date.slice(0, 7);
			let g = groups.find((g) => g.key === key);
			if (!g) {
				g = { key, items: [] };
				groups.push(g);
			}
			g.items.push({ row: r, date });
		}
		groups.sort((a, b) => b.key.localeCompare(a.key));
		for (const g of groups) g.items.sort((a, b) => b.date.localeCompare(a.date));
		return { groups, rest };
	});

	function monthLabel(key: string): string {
		return new Date(key + '-01T00:00:00').toLocaleDateString(undefined, { month: 'long', year: 'numeric' });
	}
</script>

<div class="db" data-database>
	{#if readOnly}
		<div class="db-linked-banner">Linked database — mirrored from the source block. Edit it there.</div>
	{/if}
	<div class="db-toolbar">
		<div class="db-view-switch" role="group" aria-label="View">
			{#each VIEWS as v (v)}
				<button class="db-view-btn" class:active={view === v} onclick={() => setView(v)}>{v}</button>
			{/each}
		</div>
		{#if view === 'kanban'}
			<select class="db-groupby" value={groupBy ?? ''} aria-label="Group by column" onchange={(e) => { groupBy = (e.currentTarget as HTMLSelectElement).value || null; emit(); }}>
				<option value="">Group by…</option>
				{#each columns.filter((c) => c.type === 'select') as col (col.id)}
					<option value={col.id}>{col.name}</option>
				{/each}
			</select>
		{/if}
		<div class="db-toolbar-spacer"></div>
		<button class="db-filter-toggle" onclick={() => (filterOpen = !filterOpen)}>Filter</button>
	</div>

	{#if filterOpen}
		<div class="db-filter-row">
			{#each columns as col (col.id)}
				<input
					class="db-filter-input"
					placeholder="Filter {col.name}…"
					value={filters[col.id] ?? ''}
					oninput={(e) => { filters = { ...filters, [col.id]: (e.currentTarget as HTMLInputElement).value }; emit(); }}
				/>
			{/each}
		</div>
	{/if}

	{#if view === 'table'}
		<div class="db-header-row">
			{#each columns as col (col.id)}
				<div class="db-header">
					<input
						class="db-header-name"
						value={col.name}
						disabled={readOnly}
						aria-label="Column name"
						oninput={(e) => renameColumn(col.id, (e.currentTarget as HTMLInputElement).value)}
					/>
					{#if !readOnly}
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
					{/if}
				</div>
			{/each}
			{#if !readOnly}
				<button class="db-add-col" onclick={addColumn}>+ Column</button>
			{/if}
		</div>

		{#each visibleRows as row (row.id)}
			<div class="db-row">
				{#each columns as col (col.id)}
					{#if col.type === 'checkbox'}
						<div class="db-cell db-cell-check">
							<input type="checkbox" disabled={readOnly} checked={cellValue(row, col) === true} onclick={(e) => setCell(row.id, col, (e.currentTarget as HTMLInputElement).checked)} />
						</div>
					{:else if col.type === 'select' || col.type === 'multiSelect'}
						<div
							class="db-cell db-cell-tags"
							role="button"
							tabindex="0"
							onclick={(e) => openMenu(e, col, row.id)}
							onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') { e.preventDefault(); openMenu(e.currentTarget as HTMLElement, col, row.id); } }}
						>
							{#each (col.type === 'multiSelect' ? cellValue(row, col) as string[] : cellValue(row, col) ? [String(cellValue(row, col))] : []) as t}
								<span class="db-chip" style="background:{tagColor(t)}22; color:{tagColor(t)};">{t}</span>
							{/each}
							{#if (col.type === 'select' ? cellValue(row, col) === '' : (cellValue(row, col) as string[]).length === 0)}
								<span class="db-cell-empty">+</span>
							{/if}
						</div>
					{:else if col.type === 'progress'}
						<div class="db-cell db-cell-progress">
							<input type="range" min="0" max="100" disabled={readOnly} value={Number(cellValue(row, col)) || 0} aria-label={col.name} oninput={(e) => setCell(row.id, col, (e.currentTarget as HTMLInputElement).value)} />
							<span class="db-pct">{Number(cellValue(row, col)) || 0}%</span>
						</div>
					{:else if col.type === 'createdAt' || col.type === 'updatedAt'}
						<input class="db-cell" type="text" disabled value={String(cellValue(row, col)).slice(0, 10)} aria-label={col.name} />
					{:else}
						<input
							class="db-cell"
							type={col.type === 'number' ? 'number' : col.type === 'date' ? 'date' : col.type === 'url' ? 'url' : col.type === 'email' ? 'email' : 'text'}
							disabled={readOnly}
							value={String(cellValue(row, col))}
							oninput={(e) => setCell(row.id, col, (e.currentTarget as HTMLInputElement).value)}
						/>
					{/if}
				{/each}
				{#if !readOnly}
					<button class="db-remove" aria-label="Delete row" onclick={() => removeRow(row.id)}>✕</button>
				{/if}
			</div>
		{/each}
	{:else if view === 'kanban'}
		<div class="db-kanban">
			{#each kanbanGroups as g (g.value)}
				<div class="db-kb-col" role="group" aria-label={g.label} ondragover={(e) => e.preventDefault()} ondrop={(e) => moveCard(e, g.value)}>
					<div class="db-kb-head">
						<span class="db-kb-dot" style="background:{g.value ? tagColor(g.value) : 'var(--color-text-muted)'}"></span>
						{g.label}
						<span class="db-kb-count">{g.rows.length}</span>
					</div>
					<div class="db-kb-body">
						{#each g.rows as row (row.id)}
							<div class="db-kb-card" role="listitem" draggable="true" ondragstart={(e) => e.dataTransfer?.setData('text/row', row.id)}>
								{#each columns as col (col.id)}
									{#if col.type === 'select' || col.type === 'multiSelect'}
										<div class="db-kb-tags" role="button" tabindex="0" onclick={(e) => openMenu(e, col, row.id)} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') { e.preventDefault(); openMenu(e.currentTarget as HTMLElement, col, row.id); } }}>
											{#each (col.type === 'multiSelect' ? cellValue(row, col) as string[] : cellValue(row, col) ? [String(cellValue(row, col))] : []) as t}
												<span class="db-chip" style="background:{tagColor(t)}22; color:{tagColor(t)};">{t}</span>
											{/each}
										</div>
									{:else if col.id !== groupBy}
										<div class="db-kb-cell"><DBCell {col} {row} onSet={(v) => setCell(row.id, col, v)} /></div>
									{/if}
								{/each}
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{:else if view === 'list'}
		<div class="db-list">
			{#each visibleRows as row (row.id)}
				<div class="db-list-row">
					{#each columns as col, i (col.id)}
						<div class="db-list-item" class:db-list-title={i === 0}><DBCell {col} {row} /></div>
					{/each}
					{#if !readOnly}
						<button class="db-remove" aria-label="Delete row" onclick={() => removeRow(row.id)}>✕</button>
					{/if}
				</div>
			{/each}
		</div>
	{:else if view === 'gallery'}
		<div class="db-gallery">
			{#each visibleRows as row (row.id)}
				<div class="db-card">
					{#each columns as col, i (col.id)}
						<div class="db-card-item" class:db-card-title={i === 0}><DBCell {col} {row} /></div>
					{/each}
					{#if !readOnly}
						<button class="db-remove db-card-del" aria-label="Delete row" onclick={() => removeRow(row.id)}>✕</button>
					{/if}
				</div>
			{/each}
		</div>
	{:else if view === 'timeline'}
		<div class="db-tl">
			{#each timelineGroups.groups as g (g.key)}
				<div class="db-tl-group">
					<div class="db-tl-month">{monthLabel(g.key)}</div>
					{#each g.items as item (item.row.id)}
						<div class="db-tl-item">
							<span class="db-tl-date">{item.date.slice(8, 10)}</span>
							<div class="db-card db-tl-card">
								{#each columns as col, i (col.id)}
									<div class="db-card-item" class:db-card-title={i === 0}><DBCell col={col} row={item.row} /></div>
								{/each}
								{#if !readOnly}
									<button class="db-remove db-card-del" aria-label="Delete row" onclick={() => removeRow(item.row.id)}>✕</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/each}
			{#if timelineGroups.rest.length > 0}
				<div class="db-tl-group">
					<div class="db-tl-month">No date</div>
					{#each timelineGroups.rest as row (row.id)}
						<div class="db-tl-item">
							<span class="db-tl-date">—</span>
							<div class="db-card db-tl-card">
								{#each columns as col, i (col.id)}
									<div class="db-card-item" class:db-card-title={i === 0}><DBCell {col} {row} /></div>
								{/each}
								{#if !readOnly}
									<button class="db-remove db-card-del" aria-label="Delete row" onclick={() => removeRow(row.id)}>✕</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	{#if !readOnly}
		<div class="db-footer">
			<button class="db-add-row" onclick={addRow}>+ Row</button>
		</div>
	{/if}
</div>

{#if menu}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="db-menu-backdrop" onclick={() => (menu = null)}></div>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="db-menu" role="listbox" aria-label="Options" tabindex="-1" style="left:{menu.x}px; top:{menu.y}px;" onclick={(e: MouseEvent) => e.stopPropagation()}>
		<div class="db-menu-title">{(menuCol()?.type === 'select' ? 'Select' : 'Multi-select')} options</div>
		{#each menuCol()?.options ?? [] as opt}
			<div
				class="db-menu-opt"
				role="option"
				tabindex="0"
				aria-selected={menuCol()?.type === 'multiSelect' && (cellValue(rows.find((r) => r.id === menu!.rowId)!, menuCol()!) as string[]).includes(opt)}
				onclick={() => toggleOption(menu!.colId, menu!.rowId, opt)}
				onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleOption(menu!.colId, menu!.rowId, opt); } }}
			>
				<span class="db-chip" style="background:{tagColor(opt)}22; color:{tagColor(opt)};">{opt}</span>
				<span class="db-menu-check">{menuCol()?.type === 'multiSelect' && (cellValue(rows.find((r) => r.id === menu!.rowId)!, menuCol()!) as string[]).includes(opt) ? '✓' : ''}</span>
				<button class="db-remove db-menu-del" aria-label="Delete option" onclick={(e: MouseEvent) => { e.stopPropagation(); removeOption(menu!.colId, opt); }}>✕</button>
			</div>
		{/each}
		<div class="db-menu-add">
			<input
				class="db-menu-input"
				placeholder="Add option…"
				bind:value={newOption}
				onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') addOption(menu!.colId, newOption); }}
			/>
		</div>
	</div>
{/if}

<style>
	.db {
		border: 1px solid var(--color-border);
		border-radius: 10px;
		background: var(--color-surface);
		margin: 8px 0;
		overflow: hidden;
	}

	.db-linked-banner {
		padding: 6px 10px;
		font-size: 12px;
		color: var(--color-text-muted);
		background: rgba(124, 111, 240, 0.08);
		border-bottom: 1px solid var(--color-border);
	}

	.db-toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-bottom: 1px solid var(--color-border);
	}

	.db-view-switch {
		display: flex;
		border: 1px solid var(--color-border);
		border-radius: 8px;
		overflow: hidden;
	}

	.db-view-btn {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		padding: 4px 10px;
		cursor: pointer;
	}

	.db-view-btn.active {
		background: var(--color-hover);
		color: var(--color-text);
	}

	.db-groupby {
		border: 1px solid var(--color-border);
		border-radius: 8px;
		background: var(--color-bg);
		color: var(--color-text);
		font-size: 12px;
		padding: 3px 6px;
	}

	.db-toolbar-spacer {
		flex: 1;
	}

	.db-filter-toggle,
	.db-add-row {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		cursor: pointer;
		padding: 2px 6px;
		border-radius: 4px;
	}

	.db-filter-toggle:hover,
	.db-add-row:hover {
		background: var(--color-hover);
		color: var(--color-text);
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
		min-width: 180px;
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

	.db-header-name:disabled {
		opacity: 1;
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
	.db-row:hover .db-remove,
	.db-list-row:hover .db-remove,
	.db-card:hover .db-card-del,
	.db-tl-card:hover .db-card-del {
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
		min-width: 180px;
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
		min-width: 180px;
		outline: none;
	}

	.db-cell:focus {
		background: var(--color-hover);
	}

	.db-cell:disabled {
		opacity: 0.55;
	}

	.db-cell-check {
		display: flex;
		align-items: center;
		padding: 6px 8px;
	}

	.db-cell-progress {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		min-width: 180px;
		border-right: 1px solid var(--color-border);
	}

	.db-cell-progress input {
		flex: 1;
		min-width: 90px;
		accent-color: #7c6cf0;
	}

	.db-pct {
		font-size: 11px;
		color: var(--color-text-muted);
		min-width: 34px;
		text-align: right;
	}

	.db-cell-tags {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 4px;
		padding: 5px 8px;
		min-width: 180px;
		cursor: pointer;
		border-right: 1px solid var(--color-border);
	}

	.db-cell-empty {
		color: var(--color-text-muted);
		font-size: 14px;
	}

	.db-chip {
		border-radius: 999px;
		padding: 1px 8px;
		font-size: 11px;
		font-weight: 500;
		white-space: nowrap;
	}

	.db-footer {
		display: flex;
		gap: 8px;
		padding: 6px 8px;
		border-top: 1px solid var(--color-border);
	}

	/* ── Kanban ── */
	.db-kanban {
		display: flex;
		gap: 10px;
		padding: 10px;
		overflow-x: auto;
		align-items: flex-start;
	}

	.db-kb-col {
		flex: 0 0 230px;
		border-radius: 8px;
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		min-height: 80px;
	}

	.db-kb-head {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 10px;
		font-size: 13px;
		font-weight: 600;
		color: var(--color-text);
		border-bottom: 1px solid var(--color-border);
	}

	.db-kb-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.db-kb-count {
		margin-left: auto;
		font-size: 11px;
		color: var(--color-text-muted);
		background: var(--color-hover);
		border-radius: 999px;
		padding: 0 8px;
	}

	.db-kb-body {
		padding: 8px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-height: 40px;
	}

	.db-kb-card {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 8px 10px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		cursor: grab;
		font-size: 13px;
	}

	.db-kb-card:hover {
		border-color: rgba(124, 111, 240, 0.5);
	}

	.db-kb-cell {
		overflow: hidden;
	}

	.db-kb-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	/* ── List ── */
	.db-list {
		padding: 4px 10px;
	}

	.db-list-row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 7px 4px;
		border-bottom: 1px dashed var(--color-border);
		font-size: 13px;
	}

	.db-list-item {
		min-width: 0;
		flex: 1;
		overflow: hidden;
	}

	.db-list-title {
		flex: 2;
		font-weight: 600;
	}

	/* ── Gallery / timeline cards ── */
	.db-gallery {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 10px;
		padding: 10px;
	}

	.db-card {
		position: relative;
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-size: 13px;
		min-width: 0;
	}

	.db-card:hover {
		border-color: rgba(124, 111, 240, 0.5);
	}

	.db-card-item {
		min-width: 0;
		overflow: hidden;
	}

	.db-card-title {
		font-weight: 600;
	}

	.db-card-del {
		position: absolute;
		top: 6px;
		right: 6px;
	}

	/* ── Timeline ── */
	.db-tl {
		padding: 12px 14px;
	}

	.db-tl-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding-left: 14px;
		border-left: 2px solid var(--color-border);
		margin-left: 8px;
		padding-bottom: 14px;
	}

	.db-tl-month {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin-left: -14px;
		padding-left: 14px;
		background: var(--color-surface);
	}

	.db-tl-item {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	.db-tl-date {
		font-size: 12px;
		color: var(--color-text-muted);
		width: 26px;
		text-align: center;
		flex-shrink: 0;
		line-height: 22px;
	}

	.db-tl-card {
		flex: 1;
		background: var(--color-surface);
	}

	/* ── Option popover ── */
	.db-menu-backdrop {
		position: fixed;
		inset: 0;
		z-index: 200;
	}

	.db-menu {
		position: fixed;
		z-index: 201;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 6px;
		min-width: 180px;
		max-width: 260px;
		max-height: 260px;
		overflow-y: auto;
	}

	.db-menu-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		padding: 4px 8px;
	}

	.db-menu-opt {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		padding: 5px 8px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
		text-align: left;
	}

	.db-menu-opt:hover {
		background: var(--color-hover);
	}

	.db-menu-opt:hover .db-menu-del {
		opacity: 1;
	}

	.db-menu-check {
		margin-left: auto;
		color: #7c6cf0;
		font-size: 13px;
	}

	.db-menu-del {
		opacity: 0;
	}

	.db-menu-add {
		border-top: 1px solid var(--color-border);
		margin-top: 4px;
		padding-top: 4px;
	}

	.db-menu-input {
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		padding: 5px 8px;
		outline: none;
	}
</style>
