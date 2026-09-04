<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { Icon } from '@enclave/ui';
	import { invoke, convertFileSrc } from '@tauri-apps/api/core';
	import { SlashCommandPluginKey } from '../extensions/slash-command.js';
	import { templates } from '../templates.js';
	import type { Template } from '../templates.js';
	import { listDatabases } from '../dbLink.js';
	import type { DatabaseRef } from '../dbLink.js';

	interface Command {
		id: string;
		label: string;
		icon: string;
		description: string;
		group: 'basic' | 'lists' | 'media' | 'advanced';
		action: (editor: Editor) => void;
	}

	let {
		editor,
	}: {
		editor: Editor | undefined;
	} = $props();

	let fileInput: HTMLInputElement | undefined = $state();

	async function importImage(file: File) {
		if (!editor) return;
		try {
			const bytes = new Uint8Array(await file.arrayBuffer());
			const abs = await invoke<string>('save_attachment', {
				documentId: editor.storage.image.docId,
				filename: file.name || `image-${Date.now()}.png`,
				data: Array.from(bytes),
			});
			editor.chain().focus().setImage({ src: convertFileSrc(abs), alt: file.name }).run();
		} catch (e) {
			console.error('Failed to import image:', e);
		}
	}

	async function importFile(file: File) {
		if (!editor) return;
		try {
			const bytes = new Uint8Array(await file.arrayBuffer());
			const abs = await invoke<string>('save_attachment', {
				documentId: editor.storage.image.docId,
				filename: file.name || `file-${Date.now()}.pdf`,
				data: Array.from(bytes),
			});
			editor.chain().focus().setFile({ path: abs, name: file.name }).run();
		} catch (e) {
			console.error('Failed to import file:', e);
		}
	}

	function pickImage(ed: Editor) {
		fileInput?.click();
	}

	let fileInput2: HTMLInputElement | undefined = $state();
	function pickFile(ed: Editor) {
		fileInput2?.click();
	}

	const commands: Command[] = [
		{
			id: 'paragraph',
			label: 'Text',
			icon: 'text',
			group: 'basic',
			description: 'Start with plain text',
			action: (ed) => ed.chain().focus().setParagraph().run(),
		},
		{
			id: 'heading1',
			label: 'Heading 1',
			icon: 'heading1',
			group: 'basic',
			description: 'Large section heading',
			action: (ed) => ed.chain().focus().setHeading({ level: 1 }).run(),
		},
		{
			id: 'heading2',
			label: 'Heading 2',
			icon: 'heading2',
			group: 'basic',
			description: 'Medium section heading',
			action: (ed) => ed.chain().focus().setHeading({ level: 2 }).run(),
		},
		{
			id: 'heading3',
			label: 'Heading 3',
			icon: 'heading3',
			group: 'basic',
			description: 'Small section heading',
			action: (ed) => ed.chain().focus().setHeading({ level: 3 }).run(),
		},
		{
			id: 'bulletList',
			label: 'Bullet List',
			icon: 'list',
			group: 'lists',
			description: 'Create a bulleted list',
			action: (ed) => ed.chain().focus().toggleBulletList().run(),
		},
		{
			id: 'orderedList',
			label: 'Numbered List',
			icon: 'listOrdered',
			group: 'lists',
			description: 'Create a numbered list',
			action: (ed) => ed.chain().focus().toggleOrderedList().run(),
		},
		{
			id: 'taskList',
			label: 'Task List',
			icon: 'listChecks',
			group: 'lists',
			description: 'Track tasks with checkboxes',
			action: (ed) => ed.chain().focus().toggleTaskList().run(),
		},
		{
			id: 'toggleBlock',
			label: 'Toggle',
			icon: 'toggle',
			group: 'lists',
			description: 'Collapsible section',
			action: (ed) => ed.chain().focus().setToggleBlock().run(),
		},
		{
			id: 'blockquote',
			label: 'Quote',
			icon: 'quote',
			group: 'advanced',
			description: 'Capture a blockquote',
			action: (ed) => ed.chain().focus().toggleBlockquote().run(),
		},
		{
			id: 'callout',
			label: 'Callout',
			icon: 'callout',
			group: 'advanced',
			description: 'Highlighted info box',
			action: (ed) => ed.chain().focus().toggleCallout().run(),
		},
		{
			id: 'markdownTable',
			label: 'Markdown Table',
			icon: 'table',
			group: 'advanced',
			description: 'Insert a simple markdown table',
			action: (ed) => ed.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
		},
		{
			id: 'database',
			label: 'Database',
			icon: 'database',
			group: 'advanced',
			description: 'Insert a typed table database',
			action: (ed) => ed.chain().focus().setDatabase().run(),
		},
		{
			id: 'linkedDatabase',
			label: 'Linked Database',
			icon: 'layout',
			group: 'advanced',
			description: 'Mirror another database on this page',
			action: () => {
				showingLinkedDb = !showingLinkedDb;
				refreshLinked();
			},
		},
		{
			id: 'pageEmbed',
			label: 'Embed Page',
			icon: 'page',
			group: 'media',
			description: 'Embed a link to another page',
			action: (ed) => ed.chain().focus().setPageEmbed().run(),
		},
		{
			id: 'image',
			label: 'Image',
			icon: 'image',
			group: 'media',
			description: 'Insert an image from your device',
			action: (ed) => pickImage(ed),
		},
		{
			id: 'file',
			label: 'PDF',
			icon: 'page',
			group: 'media',
			description: 'Insert a PDF file and view it here',
			action: (ed) => pickFile(ed),
		},
		{
			id: 'templates',
			label: 'Template',
			icon: 'grid',
			group: 'advanced',
			description: 'Insert a starter template',
			action: () => { showingTemplates = !showingTemplates; },
		},
		{
			id: 'bookmark',
			label: 'Bookmark',
			icon: 'bookmark',
			group: 'media',
			description: 'Insert a link card',
			// ponytail: window.prompt for the URL — a small inline form is
			// nicer but costs a menu state; paste-URL already inserts directly.
			action: (ed) => {
				const url = window.prompt('Paste a URL:');
				if (url) ed.chain().focus().setBookmark(url.trim()).run();
			},
		},
		{
			id: 'codeBlock',
			label: 'Code Block',
			icon: 'codeBlock',
			group: 'advanced',
			description: 'Insert a code snippet',
			action: (ed) => ed.chain().focus().toggleCodeBlock().run(),
		},
		{
			id: 'horizontalRule',
			label: 'Divider',
			icon: 'divider',
			group: 'advanced',
			description: 'Insert a horizontal divider',
			action: (ed) => ed.chain().focus().setHorizontalRule().run(),
		},
	];

	const GROUP_LABELS: Record<Command['group'], string> = {
		basic: 'Basic blocks',
		lists: 'Lists',
		media: 'Media',
		advanced: 'Advanced',
	};

	let query = $state('');
	let selectedIndex = $state(0);
	let visible = $state(false);
	let showingTemplates = $state(false);
	let showingLinkedDb = $state(false);
	let linkedDbs = $state<DatabaseRef[]>([]);
	let position = $state({ x: 0, y: 0 });

	let filtered = $derived(
		query
			? commands.filter((c) =>
					c.label.toLowerCase().includes(query.toLowerCase())
				)
			: commands
	);

	// Grouped rows keep their flat index so keyboard navigation (selectedIndex)
	// and the hover/arrow highlight stay in sync.
	let filteredGroups = $derived.by(() => {
		const out: { group: Command['group']; label: string; items: Command[] }[] = [];
		for (const cmd of filtered) {
			const g = out.find((x) => x.group === cmd.group);
			if (g) g.items.push(cmd);
			else out.push({ group: cmd.group, label: GROUP_LABELS[cmd.group], items: [cmd] });
		}
		return out;
	});
	let flatIndex = $derived.by(() => {
		const m = new Map<string, number>();
		let i = 0;
		for (const g of filteredGroups) for (const c of g.items) m.set(c.id, i++);
		return m;
	});

	function selectCommand(cmd: Command) {
		if (!editor) return;
		// Delete the "/" trigger text before executing
		deleteSlashTrigger(editor);
		cmd.action(editor);
		visible = false;
		query = '';
	}

	function deleteSlashTrigger(ed: Editor) {
		const { from } = ed.state.selection;
		const pluginState = SlashCommandPluginKey.getState(ed.state);
		if (pluginState) {
			ed
				.chain()
				.focus()
				.deleteRange({ from: pluginState.range.from, to: pluginState.range.to })
				.run();
		}
	}

	function pickTemplate(t: Template) {
		if (!editor) return;
		deleteSlashTrigger(editor);
		editor.chain().focus().insertContent(t.content).run();
		visible = false;
		query = '';
		showingTemplates = false;
	}

	function refreshLinked() {
		if (!editor) return;
		linkedDbs = listDatabases(editor.state.doc.toJSON());
	}

	function genId(): string {
		return Math.random().toString(36).slice(2, 10);
	}

	function pickLinkedDb(ref: DatabaseRef) {
		if (!editor) return;
		deleteSlashTrigger(editor);
		// Re-resolve the node after the trigger deletion shifted positions.
		const fresh =
			(listDatabases(editor.state.doc.toJSON()).find(
				(r) => (ref.id && r.id === ref.id) || (r.name === ref.name && r.rowCount === ref.rowCount)
			) ?? ref);
		let sourceId = fresh.id;
		let data = fresh.data;
		if (!sourceId) {
			// Older databases have no id — stamp one so the link can resolve.
			sourceId = genId();
			data = { ...fresh.data, id: sourceId };
			editor.view.dispatch(editor.state.tr.setNodeMarkup(fresh.pos, undefined, { data: JSON.stringify(data) }));
		}
		editor
			.chain()
			.focus()
			.setLinkedDatabase(sourceId, {
				columns: data.columns ?? [],
				rows: data.rows ?? [],
				view: data.view,
				groupBy: data.groupBy ?? null,
				sort: data.sort ?? null,
				filters: data.filters ?? {},
			})
			.run();
		visible = false;
		query = '';
		showingLinkedDb = false;
	}

	function updatePosition() {
		if (!editor) return;
		const { from } = editor.state.selection;
		const coords = editor.view.coordsAtPos(from);
		// Fixed positioning + viewport coords: the menu stays where the caret
		// is regardless of scroll containers, and can't land "somewhere in the
		// app" when an ancestor isn't a positioned element.
		position = {
			x: Math.min(Math.max(coords.left, 8), window.innerWidth - 296),
			y: Math.min(coords.bottom + 8, window.innerHeight - 340),
		};
	}

	// ── Scroll tracking ──
	let scrollContainer: Element | null = null;

	$effect(() => {
		const ed = editor;
		if (!ed) return;

		const checkState = () => {
			const state = SlashCommandPluginKey.getState(ed.state);
			if (state) {
				query = state.query;
				selectedIndex = 0;
				visible = true;
				updatePosition();
				if (showingLinkedDb) refreshLinked();
			} else {
				visible = false;
				showingTemplates = false;
				showingLinkedDb = false;
			}
		};

		// Find scrollable ancestor
		scrollContainer = ed.view.dom.closest('.main-pane') || ed.view.dom.parentElement;
		const onScroll = () => { if (visible) updatePosition(); };
		scrollContainer?.addEventListener('scroll', onScroll, { passive: true });

		ed.on('transaction', checkState);
		ed.on('selectionUpdate', () => {
			if (visible) updatePosition();
		});

		return () => {
			ed.off('transaction', checkState);
			scrollContainer?.removeEventListener('scroll', onScroll);
			scrollContainer = null;
		};
	});

	function handleKeydown(e: KeyboardEvent) {
		if (!visible) return;

		if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			if (showingTemplates) {
				const t = templates[selectedIndex];
				if (t) pickTemplate(t);
			} else if (showingLinkedDb) {
				const ref = linkedDbs[selectedIndex];
				if (ref) pickLinkedDb(ref);
			} else {
				const cmd = filtered[selectedIndex];
				if (cmd) selectCommand(cmd);
			}
		} else if (e.key === 'Escape') {
			visible = false;
			showingTemplates = false;
			showingLinkedDb = false;
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<input
	bind:this={fileInput}
	type="file"
	accept="image/*"
	class="hidden-file-input"
	onchange={(e: Event) => {
		const f = (e.currentTarget as HTMLInputElement).files?.[0];
		(e.currentTarget as HTMLInputElement).value = '';
		if (f && editor) {
			deleteSlashTrigger(editor);
			visible = false;
			void importImage(f);
		}
	}}
	aria-hidden="true"
/>
<input
	bind:this={fileInput2}
	type="file"
	accept="application/pdf,.pdf"
	class="hidden-file-input"
	onchange={(e: Event) => {
		const f = (e.currentTarget as HTMLInputElement).files?.[0];
		(e.currentTarget as HTMLInputElement).value = '';
		if (f && editor) {
			deleteSlashTrigger(editor);
			visible = false;
			void importFile(f);
		}
	}}
	aria-hidden="true"
/>


{#if visible && editor}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="slash-menu"
		style="left: {position.x}px; top: {position.y}px;"
	>
		{#if showingTemplates}
			<div class="slash-menu-header">Templates</div>
			<div class="tpl-grid">
				{#each templates as t (t.id)}
					<button class="tpl-item" onclick={() => pickTemplate(t)}>
						<span class="tpl-icon">{t.icon}</span>
						<span class="tpl-name">{t.name}</span>
					</button>
				{/each}
			</div>
		{:else if showingLinkedDb}
			<div class="slash-menu-header">Linked Database</div>
			{#if linkedDbs.length === 0}
				<div class="slash-empty">No databases on this page yet</div>
			{:else}
				{#each linkedDbs as ref}
					<button class="slash-item" onclick={() => pickLinkedDb(ref)}>
						<span class="slash-item-icon">⧉</span>
						<div class="slash-item-text">
							<span class="slash-item-label">{ref.name}</span>
							<span class="slash-item-desc">{ref.rowCount} {ref.rowCount === 1 ? 'row' : 'rows'}</span>
						</div>
					</button>
				{/each}
			{/if}
		{:else}
			{#each filteredGroups as g (g.group)}
				<div class="slash-menu-header">{g.label}</div>
				{#each g.items as cmd (cmd.id)}
					{@const idx = flatIndex.get(cmd.id) ?? 0}
					<button
						class="slash-item"
						class:selected={idx === selectedIndex}
						onclick={() => selectCommand(cmd)}
					>
						<span class="slash-item-icon"><Icon name={cmd.icon} size={16} /></span>
						<div class="slash-item-text">
							<span class="slash-item-label">{cmd.label}</span>
							<span class="slash-item-desc">{cmd.description}</span>
						</div>
					</button>
				{/each}
			{/each}
		{/if}
	</div>
{/if}

<style>
	.slash-menu {
		position: fixed;
		z-index: 300;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 6px;
		width: 280px;
		max-height: 320px;
		overflow-y: auto;
	}

	.slash-menu-header {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		padding: 6px 10px 4px;
	}

	.slash-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 8px 10px;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--color-text);
		cursor: pointer;
		font-size: 14px;
		text-align: left;
		transition: background-color 0.1s;
	}

	.slash-item:hover,
	.slash-item.selected {
		background-color: color-mix(in srgb, var(--color-accent) 14%, transparent);
	}

	.slash-item-icon {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 8px;
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
		flex-shrink: 0;
		transition: color 0.1s;
	}
	.slash-item:hover .slash-item-icon,
	.slash-item.selected .slash-item-icon {
		color: var(--color-accent);
		background: var(--color-accent-subtle);
	}

	.slash-item-text {
		display: flex;
		flex-direction: column;
	}

	.slash-item-label {
		font-size: 14px;
		font-weight: 500;
	}

	.slash-item-desc {
		font-size: 12px;
		color: var(--color-text-muted);
	}

	.hidden-file-input {
		display: none;
	}

	.slash-empty {
		padding: 8px 10px;
		font-size: 13px;
		color: var(--color-text-muted);
	}

	/* Template gallery — two-column grid of icon + name tiles. */
	.tpl-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 6px;
		padding: 2px;
	}
	.tpl-item {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		border: 1px solid var(--color-border);
		border-radius: 10px;
		background: none;
		color: var(--color-text);
		cursor: pointer;
		padding: 12px 8px;
		font-family: inherit;
		transition: border-color 0.1s, background 0.1s;
	}
	.tpl-item:hover,
	.tpl-item.selected {
		border-color: var(--color-accent);
		background: var(--color-accent-subtle);
	}
	.tpl-icon {
		font-size: 20px;
		line-height: 1;
	}
	.tpl-name {
		font-size: 12px;
		font-weight: 500;
		text-align: center;
	}
</style>
