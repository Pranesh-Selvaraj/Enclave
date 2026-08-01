<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { invoke, convertFileSrc } from '@tauri-apps/api/core';
	import { SlashCommandPluginKey } from '../extensions/slash-command.js';
	import { templates } from '../templates.js';
	import type { Template } from '../templates.js';

	interface Command {
		id: string;
		label: string;
		icon: string;
		description: string;
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

	function pickImage(ed: Editor) {
		fileInput?.click();
	}

	const commands: Command[] = [
		{
			id: 'paragraph',
			label: 'Text',
			icon: '¶',
			description: 'Start with plain text',
			action: (ed) => ed.chain().focus().setParagraph().run(),
		},
		{
			id: 'heading1',
			label: 'Heading 1',
			icon: 'H1',
			description: 'Large section heading',
			action: (ed) => ed.chain().focus().setHeading({ level: 1 }).run(),
		},
		{
			id: 'heading2',
			label: 'Heading 2',
			icon: 'H2',
			description: 'Medium section heading',
			action: (ed) => ed.chain().focus().setHeading({ level: 2 }).run(),
		},
		{
			id: 'heading3',
			label: 'Heading 3',
			icon: 'H3',
			description: 'Small section heading',
			action: (ed) => ed.chain().focus().setHeading({ level: 3 }).run(),
		},
		{
			id: 'bulletList',
			label: 'Bullet List',
			icon: '•',
			description: 'Create a bulleted list',
			action: (ed) => ed.chain().focus().toggleBulletList().run(),
		},
		{
			id: 'orderedList',
			label: 'Numbered List',
			icon: '1.',
			description: 'Create a numbered list',
			action: (ed) => ed.chain().focus().toggleOrderedList().run(),
		},
		{
			id: 'taskList',
			label: 'Task List',
			icon: '☑',
			description: 'Track tasks with checkboxes',
			action: (ed) => ed.chain().focus().toggleTaskList().run(),
		},
		{
			id: 'blockquote',
			label: 'Quote',
			icon: '"',
			description: 'Capture a blockquote',
			action: (ed) => ed.chain().focus().toggleBlockquote().run(),
		},
		{
			id: 'callout',
			label: 'Callout',
			icon: '💡',
			description: 'Highlighted info box',
			action: (ed) => ed.chain().focus().toggleCallout().run(),
		},
		{
			id: 'toggleBlock',
			label: 'Toggle',
			icon: '▶',
			description: 'Collapsible section',
			action: (ed) => ed.chain().focus().setToggleBlock().run(),
		},
		{
			id: 'database',
			label: 'Database',
			icon: '▦',
			description: 'Insert a typed table database',
			action: (ed) => ed.chain().focus().setDatabase().run(),
		},
		{
			id: 'pageEmbed',
			label: 'Embed Page',
			icon: '🔗',
			description: 'Embed a link to another page',
			action: (ed) => ed.chain().focus().setPageEmbed().run(),
		},
		{
			id: 'image',
			label: 'Image',
			icon: '🖼',
			description: 'Insert an image from your device',
			action: (ed) => pickImage(ed),
		},
		{
			id: 'templates',
			label: 'Template',
			icon: '📄',
			description: 'Insert a starter template',
			action: () => { showingTemplates = !showingTemplates; },
		},
		{
			id: 'bookmark',
			label: 'Bookmark',
			icon: '🔖',
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
			icon: '</>',
			description: 'Insert a code snippet',
			action: (ed) => ed.chain().focus().toggleCodeBlock().run(),
		},
		{
			id: 'horizontalRule',
			label: 'Divider',
			icon: '—',
			description: 'Insert a horizontal divider',
			action: (ed) => ed.chain().focus().setHorizontalRule().run(),
		},
	];

	let query = $state('');
	let selectedIndex = $state(0);
	let visible = $state(false);
	let showingTemplates = $state(false);
	let position = $state({ x: 0, y: 0 });

	let filtered = $derived(
		query
			? commands.filter((c) =>
					c.label.toLowerCase().includes(query.toLowerCase())
				)
			: commands
	);

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

	function updatePosition() {
		if (!editor) return;
		const { from } = editor.state.selection;
		const coords = editor.view.coordsAtPos(from);
		const editorEl = editor.view.dom.closest('.editor-container');
		const editorRect = editorEl?.getBoundingClientRect();

		position = {
			x: coords.left - (editorRect?.left ?? 0),
			y: coords.bottom - (editorRect?.top ?? 0) + 8,
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
			} else {
				visible = false;
				showingTemplates = false;
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
			} else {
				const cmd = filtered[selectedIndex];
				if (cmd) selectCommand(cmd);
			}
		} else if (e.key === 'Escape') {
			visible = false;
			showingTemplates = false;
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

{#if visible && editor}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="slash-menu"
		style="left: {position.x}px; top: {position.y}px;"
	>
		{#if showingTemplates}
			<div class="slash-menu-header">Templates</div>
			{#each templates as t}
				<button class="slash-item" onclick={() => pickTemplate(t)}>
					<span class="slash-item-icon">{t.icon}</span>
					<div class="slash-item-text">
						<span class="slash-item-label">{t.name}</span>
					</div>
				</button>
			{/each}
		{:else}
			<div class="slash-menu-header">Basic Blocks</div>
			{#each filtered as cmd, i}
				<button
					class="slash-item"
					class:selected={i === selectedIndex}
					onclick={() => selectCommand(cmd)}
				>
					<span class="slash-item-icon">{cmd.icon}</span>
					<div class="slash-item-text">
						<span class="slash-item-label">{cmd.label}</span>
						<span class="slash-item-desc">{cmd.description}</span>
					</div>
				</button>
			{/each}
		{/if}
	</div>
{/if}

<style>
	.slash-menu {
		position: absolute;
		z-index: 100;
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
		background-color: rgba(124, 111, 240, 0.12);
	}

	.slash-item-icon {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 6px;
		background: rgba(255, 255, 255, 0.04);
		font-size: 14px;
		font-weight: 600;
		flex-shrink: 0;
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
</style>
