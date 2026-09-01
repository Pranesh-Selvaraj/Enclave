<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { Icon } from '@enclave/ui';

	interface MenuItem {
		label: string;
		icon: string;
		action: (ed: Editor) => void;
		active?: (ed: Editor) => boolean;
	}

	let {
		editor,
		allPages = [],
	}: {
		editor: Editor | undefined;
		allPages: { id: string; title: string }[];
	} = $props();

	let visible = $state(false);
	let position = $state({ x: 0, y: 0 });
	// Flyout submenu (Insert / Paragraph / Format).
	let submenu = $state<{ label: string; items: MenuItem[]; x: number; y: number } | null>(null);
	// Inline input panels: 'link' = internal page picker, 'external' = URL input.
	let panel = $state<'link' | 'external' | null>(null);
	let linkQuery = $state('');
	let linkIndex = $state(0);
	let externalUrl = $state('');
	let canUndo = $state(false);
	let canRedo = $state(false);
	let canEdit = $state(false); // non-empty selection → Cut/Copy enabled
	let panelInput: HTMLInputElement | undefined = $state();

	const filteredPages = $derived(
		linkQuery ? allPages.filter((p) => p.title.toLowerCase().includes(linkQuery.toLowerCase())) : allPages
	);

	const INSERT_ITEMS: MenuItem[] = [
		{ label: 'Markdown Table', icon: 'table', action: (ed) => ed.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run() },
		{ label: 'Bullet List', icon: 'list', action: (ed) => ed.chain().focus().toggleBulletList().run() },
		{ label: 'Numbered List', icon: 'listOrdered', action: (ed) => ed.chain().focus().toggleOrderedList().run() },
		{ label: 'Task List', icon: 'listChecks', action: (ed) => ed.chain().focus().toggleTaskList().run() },
		{ label: 'Toggle', icon: 'toggle', action: (ed) => ed.chain().focus().setToggleBlock().run() },
		{ label: 'Quote', icon: 'quote', action: (ed) => ed.chain().focus().toggleBlockquote().run() },
		{ label: 'Callout', icon: 'callout', action: (ed) => ed.chain().focus().toggleCallout().run() },
		{ label: 'Divider', icon: 'divider', action: (ed) => ed.chain().focus().setHorizontalRule().run() },
		{ label: 'Code Block', icon: 'codeBlock', action: (ed) => ed.chain().focus().toggleCodeBlock().run() },
	];

	// "Paragraph" = turn-into block types (Text + headings).
	const PARAGRAPH_ITEMS: MenuItem[] = [
		{ label: 'Text', icon: 'text', action: (ed) => ed.chain().focus().setParagraph().run() },
		{ label: 'Heading 1', icon: 'heading1', action: (ed) => ed.chain().focus().setHeading({ level: 1 }).run() },
		{ label: 'Heading 2', icon: 'heading2', action: (ed) => ed.chain().focus().setHeading({ level: 2 }).run() },
		{ label: 'Heading 3', icon: 'heading3', action: (ed) => ed.chain().focus().setHeading({ level: 3 }).run() },
	];

	const FORMAT_ITEMS: MenuItem[] = [
		{ label: 'Bold', icon: 'bold', action: (ed) => ed.chain().focus().toggleBold().run(), active: (ed) => ed.isActive('bold') },
		{ label: 'Italic', icon: 'italic', action: (ed) => ed.chain().focus().toggleItalic().run(), active: (ed) => ed.isActive('italic') },
		{ label: 'Strikethrough', icon: 'strike', action: (ed) => ed.chain().focus().toggleStrike().run(), active: (ed) => ed.isActive('strike') },
		{ label: 'Inline code', icon: 'code', action: (ed) => ed.chain().focus().toggleCode().run(), active: (ed) => ed.isActive('code') },
	];

	const SUBMENUS: { label: string; items: MenuItem[] }[] = [
		{ label: 'Insert', items: INSERT_ITEMS },
		{ label: 'Paragraph', items: PARAGRAPH_ITEMS },
		{ label: 'Format', items: FORMAT_ITEMS },
	];

	function close() {
		visible = false;
		submenu = null;
		panel = null;
	}

	function openAt(e: MouseEvent) {
		const ed = editor;
		if (!ed) return;
		// Move the caret to the click point so Insert/Paragraph/Format act
		// where the user right-clicked (browsers don't move the caret on
		// right-click in contenteditable). Don't disturb an existing selection.
		const coords = ed.view.posAtCoords({ left: e.clientX, top: e.clientY });
		if (coords) {
			const { from, to } = ed.state.selection;
			if (from >= coords.pos || coords.pos >= to) {
				ed.chain().setTextSelection(coords.pos).run();
			}
		}
		const { empty, from: selFrom, to: selTo } = ed.state.selection;
		canEdit = !empty && selFrom !== selTo && ed.state.doc.textBetween(selFrom, selTo, ' ').trim().length > 0;
		canUndo = ed.can().undo();
		canRedo = ed.can().redo();
		panel = null;
		submenu = null;
		position = {
			x: Math.min(Math.max(e.clientX, 8), window.innerWidth - 220),
			// The menu scrolls (max-height) on short screens; keep its top on-screen.
			y: Math.min(Math.max(e.clientY, 8), Math.max(window.innerHeight - 60, 8)),
		};
		visible = true;
	}

	function openSubmenu(label: string, items: MenuItem[], anchor: HTMLElement) {
		const rect = anchor.getBoundingClientRect();
		const width = 190;
		const height = items.length * 32 + 12;
		// Flip to the left when the flyout would cross the right edge.
		let x = rect.right + 4;
		if (x + width > window.innerWidth - 4) x = rect.left - width - 4;
		x = Math.max(x, 4);
		const y = Math.min(Math.max(rect.top, 8), Math.max(window.innerHeight - height - 8, 8));
		submenu = { label, items, x, y };
	}

	function toggleSubmenu(label: string, items: MenuItem[], anchor: HTMLElement) {
		if (submenu?.label === label) {
			submenu = null;
			return;
		}
		openSubmenu(label, items, anchor);
	}

	function run(action: (ed: Editor) => void) {
		const ed = editor;
		if (ed) action(ed);
		close();
	}

	function addLinkPanel() {
		panel = 'link';
		submenu = null;
		linkQuery = '';
		linkIndex = 0;
	}

	function addExternalPanel() {
		panel = 'external';
		submenu = null;
		externalUrl = '';
	}

	function selectPage(page: { id: string; title: string }) {
		const ed = editor;
		if (!ed) return;
		// insertContent replaces a non-empty selection with the link text.
		ed.chain().focus().insertContent(`[[${page.title}]]`).run();
		close();
	}

	function commitExternalLink() {
		const ed = editor;
		if (!ed) return;
		let url = externalUrl.trim();
		if (!url) {
			close();
			return;
		}
		if (!/^[a-z][a-z0-9+.-]*:/i.test(url)) url = 'https://' + url;
		if (!ed.state.selection.empty) {
			ed.chain().focus().extendMarkRange('link').setLink({ href: url }).run();
		} else {
			// Empty selection: insert the URL itself as linked text.
			ed.chain()
				.focus()
				.insertContent({ type: 'text', text: url, marks: [{ type: 'link', attrs: { href: url } }] })
				.run();
		}
		close();
	}

	// ponytail: execCommand is deprecated but is the only clipboard path that
	// works uniformly in the Tauri webviews; navigator.clipboard is the
	// fallback when execCommand is denied. Full-fidelity paste (files/images)
	// needs the tauri clipboard-manager plugin — add it if paste gaps show up.
	async function doCopy(cut: boolean) {
		const ed = editor;
		if (!ed || !canEdit) return;
		const { from, to } = ed.state.selection;
		const text = ed.state.doc.textBetween(from, to, ' ');
		ed.view.focus();
		let ok = false;
		try { ok = document.execCommand(cut ? 'cut' : 'copy'); } catch { ok = false; }
		if (!ok) {
			try { await navigator.clipboard.writeText(text); } catch { /* clipboard unavailable */ }
			if (cut) ed.chain().focus().deleteSelection().run();
		}
		close();
	}

	async function doPaste() {
		const ed = editor;
		if (!ed) return;
		ed.view.focus();
		let ok = false;
		try { ok = document.execCommand('paste'); } catch { ok = false; }
		if (!ok) {
			try {
				const text = await navigator.clipboard.readText();
				if (text) ed.chain().focus().insertContent(text).run();
			} catch { /* clipboard read unavailable */ }
		}
		close();
	}

	function onLinkKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') { e.preventDefault(); linkIndex = Math.min(linkIndex + 1, filteredPages.length - 1); }
		else if (e.key === 'ArrowUp') { e.preventDefault(); linkIndex = Math.max(linkIndex - 1, 0); }
		else if (e.key === 'Enter') { e.preventDefault(); const p = filteredPages[linkIndex]; if (p) selectPage(p); }
	}

	$effect(() => {
		const ed = editor;
		if (!ed) return;
		const dom = ed.view.dom;
		const onCtx = (e: MouseEvent) => {
			e.preventDefault();
			openAt(e);
		};
		dom.addEventListener('contextmenu', onCtx);
		const onScroll = () => { if (visible) close(); };
		const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };
		window.addEventListener('scroll', onScroll, true);
		window.addEventListener('keydown', onKey);
		return () => {
			dom.removeEventListener('contextmenu', onCtx);
			window.removeEventListener('scroll', onScroll, true);
			window.removeEventListener('keydown', onKey);
		};
	});

	$effect(() => {
		if (panel) panelInput?.focus();
	});
</script>

{#if visible && editor}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="ctx-backdrop" onclick={close} oncontextmenu={(e: MouseEvent) => e.preventDefault()}></div>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="ctx-menu"
		style="left: {position.x}px; top: {position.y}px;"
		role="menu"
		aria-label="Editor menu"
		tabindex="-1"
		onclick={(e: MouseEvent) => e.stopPropagation()}
		oncontextmenu={(e: MouseEvent) => e.preventDefault()}
	>
		{#if panel === 'link'}
			<div class="ctx-panel">
				<input
					class="ctx-input"
					bind:value={linkQuery}
					bind:this={panelInput}
					placeholder="Search pages…"
					aria-label="Link to page"
					oninput={() => (linkIndex = 0)}
					onkeydown={onLinkKeydown}
				/>
				<div class="ctx-panel-list">
					{#each filteredPages.slice(0, 8) as page, i (page.id)}
						<button class="ctx-item" class:selected={i === linkIndex} onclick={() => selectPage(page)} role="menuitem">
							<Icon name="page" size={14} />
							<span class="ctx-label">{page.title}</span>
						</button>
					{/each}
					{#if filteredPages.length === 0}
						<div class="ctx-empty">No pages found</div>
					{/if}
				</div>
			</div>
		{:else if panel === 'external'}
			<div class="ctx-panel">
				<input
					class="ctx-input"
					bind:value={externalUrl}
					bind:this={panelInput}
					placeholder="https://…"
					aria-label="External URL"
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') { e.preventDefault(); commitExternalLink(); } }}
				/>
				<button class="ctx-item" onclick={commitExternalLink} role="menuitem">
					<Icon name="externalLink" size={14} />
					<span class="ctx-label">Add link</span>
				</button>
			</div>
		{:else}
			<button class="ctx-item" onclick={addLinkPanel} role="menuitem">
				<Icon name="link" size={14} />
				<span class="ctx-label">Add link</span>
			</button>
			<button class="ctx-item" onclick={addExternalPanel} role="menuitem">
				<Icon name="externalLink" size={14} />
				<span class="ctx-label">Add external link</span>
			</button>
			<div class="ctx-sep"></div>
			<button class="ctx-item" class:disabled={!canEdit} onclick={() => doCopy(true)} role="menuitem">
				<Icon name="cut" size={14} />
				<span class="ctx-label">Cut</span>
			</button>
			<button class="ctx-item" class:disabled={!canEdit} onclick={() => doCopy(false)} role="menuitem">
				<Icon name="copy" size={14} />
				<span class="ctx-label">Copy</span>
			</button>
			<button class="ctx-item" onclick={doPaste} role="menuitem">
				<Icon name="paste" size={14} />
				<span class="ctx-label">Paste</span>
			</button>
			<div class="ctx-sep"></div>
			<button class="ctx-item" class:disabled={!canUndo} onclick={() => run((ed) => ed.chain().focus().undo().run())} role="menuitem">
				<Icon name="undo" size={14} />
				<span class="ctx-label">Undo</span>
			</button>
			<button class="ctx-item" class:disabled={!canRedo} onclick={() => run((ed) => ed.chain().focus().redo().run())} role="menuitem">
				<Icon name="redo" size={14} />
				<span class="ctx-label">Redo</span>
			</button>
			<button class="ctx-item" onclick={() => run((ed) => ed.chain().focus().selectAll().run())} role="menuitem">
				<Icon name="selectAll" size={14} />
				<span class="ctx-label">Select all</span>
			</button>
			<div class="ctx-sep"></div>
			{#each SUBMENUS as sub (sub.label)}
				<button
					class="ctx-item ctx-parent"
					role="menuitem"
					onmouseenter={(e: MouseEvent) => openSubmenu(sub.label, sub.items, e.currentTarget as HTMLElement)}
					onclick={(e: MouseEvent) => toggleSubmenu(sub.label, sub.items, e.currentTarget as HTMLElement)}
				>
					<span class="ctx-label">{sub.label}</span>
					<Icon name="chevronRight" size={13} />
				</button>
			{/each}
		{/if}
	</div>

	{#if submenu && !panel}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="ctx-submenu" style="left: {submenu.x}px; top: {submenu.y}px;" role="menu" tabindex="-1" onmouseleave={() => (submenu = null)}>
			{#each submenu.items as item (item.label)}
				<button
					class="ctx-item"
					class:selected={item.active?.(editor) ?? false}
					onclick={() => run(item.action)}
					role="menuitem"
				>
					<Icon name={item.icon} size={14} />
					<span class="ctx-label">{item.label}</span>
				</button>
			{/each}
		</div>
	{/if}
{/if}

<style>
	.ctx-backdrop {
		position: fixed;
		inset: 0;
		z-index: 240;
	}

	.ctx-menu {
		position: fixed;
		z-index: 310;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 5px;
		min-width: 200px;
		max-height: calc(100vh - 16px);
		overflow-y: auto;
	}

	.ctx-submenu {
		position: fixed;
		z-index: 311;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 5px;
		min-width: 180px;
	}

	.ctx-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		font-family: inherit;
		text-align: left;
		padding: 7px 10px;
		border-radius: 6px;
		cursor: pointer;
	}
	.ctx-item :global(svg) { color: var(--color-text-faint); }
	.ctx-item:hover :global(svg) { color: var(--color-text); }

	.ctx-item:hover {
		background: var(--color-surface-hover);
	}

	.ctx-item.disabled {
		opacity: 0.4;
		cursor: default;
	}
	.ctx-item.disabled:hover {
		background: none;
	}

	.ctx-item.selected {
		color: var(--color-accent);
	}

	.ctx-parent {
		justify-content: space-between;
	}

	.ctx-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ctx-sep {
		height: 1px;
		background: var(--color-border);
		margin: 4px 6px;
	}

	.ctx-panel {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 2px;
	}
	.ctx-input {
		width: 100%;
		box-sizing: border-box;
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-text);
		font-size: 13px;
		font-family: inherit;
		padding: 6px 8px;
		outline: none;
	}
	.ctx-input:focus {
		border-color: var(--color-accent);
	}
	.ctx-panel-list {
		max-height: 220px;
		overflow-y: auto;
	}
	.ctx-empty {
		font-size: 12px;
		color: var(--color-text-faint);
		padding: 8px 10px;
		text-align: center;
	}
</style>
