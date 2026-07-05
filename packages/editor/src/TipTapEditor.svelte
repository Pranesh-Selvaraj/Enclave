<script lang="ts">
	import { Editor } from '@tiptap/core';
	import StarterKit from '@tiptap/starter-kit';
	import Placeholder from '@tiptap/extension-placeholder';
	import TaskList from '@tiptap/extension-task-list';
	import TaskItem from '@tiptap/extension-task-item';
	import { SlashCommand } from './extensions/slash-command.js';
	import { Callout } from './extensions/callout.js';
	import { ToggleBlock, ToggleSummary } from './extensions/toggle-block.js';
	import { makeReactive } from './reactivity.js';

	let {
		content = undefined,
		placeholder = 'Type / for commands…',
		editable = true,
		autofocus = false,
		editor: boundEditor = $bindable(undefined as Editor | undefined),
		onChange,
	}: {
		content?: object | string;
		placeholder?: string;
		editable?: boolean;
		autofocus?: boolean;
		editor?: Editor | undefined;
		onChange?: (json: object, html: string) => void;
	} = $props();

	let element: HTMLElement | undefined = $state();
	let _editor: Editor | undefined = $state();

	$effect(() => {
		if (!element || _editor) return;

		const instance = new Editor({
			element,
			extensions: [
				StarterKit.configure({
					heading: { levels: [1, 2, 3] },
				}),
				Placeholder.configure({ placeholder }),
				TaskList,
				TaskItem.configure({ nested: true }),
				Callout,
				ToggleBlock,
				ToggleSummary,
				SlashCommand,
			],
			content: content as string | undefined,
			editable,
			autofocus,
			onUpdate: ({ editor: ed }) => {
				onChange?.(ed.getJSON(), ed.getHTML());
			},
		});

		_editor = makeReactive(instance);
		boundEditor = _editor;

		return () => {
			instance.destroy();
			_editor = undefined as unknown as typeof _editor;
			boundEditor = undefined as unknown as typeof boundEditor;
		};
	});
</script>

<div class="editor-container">
	<div bind:this={element} class="tiptap-editor"></div>
</div>

<style>
	.editor-container {
		position: relative;
		width: 100%;
	}

	.tiptap-editor {
		outline: none;
		min-height: 200px;
	}

	:global(.tiptap-editor .ProseMirror) {
		outline: none;
		min-height: 200px;
		padding: 8px 0;
	}

	:global(.tiptap-editor .ProseMirror p.is-editor-empty:first-child::before) {
		content: attr(data-placeholder);
		float: left;
		color: var(--color-text-muted);
		pointer-events: none;
		height: 0;
	}

	:global(.tiptap-editor h1) {
		font-size: 2em;
		font-weight: 700;
		margin: 0.5em 0 0.25em;
	}

	:global(.tiptap-editor h2) {
		font-size: 1.5em;
		font-weight: 600;
		margin: 0.5em 0 0.25em;
	}

	:global(.tiptap-editor h3) {
		font-size: 1.25em;
		font-weight: 600;
		margin: 0.4em 0 0.2em;
	}

	:global(.tiptap-editor p) {
		margin: 0.25em 0;
		line-height: 1.7;
	}

	:global(.tiptap-editor ul, .tiptap-editor ol) {
		padding-left: 1.5em;
		margin: 0.25em 0;
	}

	:global(.tiptap-editor li) {
		margin: 0.15em 0;
	}

	:global(.tiptap-editor blockquote) {
		border-left: 3px solid var(--color-accent);
		padding-left: 1em;
		margin: 0.5em 0;
		color: var(--color-text-muted);
	}

	:global(.tiptap-editor code) {
		background: var(--color-surface);
		padding: 0.15em 0.4em;
		border-radius: 4px;
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
		font-size: 0.9em;
	}

	:global(.tiptap-editor pre) {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 1em;
		margin: 0.5em 0;
		overflow-x: auto;
	}

	:global(.tiptap-editor pre code) {
		background: none;
		padding: 0;
	}

	:global(.tiptap-editor hr) {
		border: none;
		border-top: 1px solid var(--color-border);
		margin: 1em 0;
	}

	/* ── Task Lists ── */
	:global(.tiptap-editor ul[data-type="taskList"]) {
		list-style: none;
		padding-left: 0;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li) {
		display: flex;
		align-items: flex-start;
		gap: 8px;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li label) {
		margin-top: 2px;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li[data-checked="true"] > div > p) {
		text-decoration: line-through;
		color: var(--color-text-muted);
	}

	/* ── Callouts ── */
	:global(.tiptap-editor [data-callout]) {
		border-left: 4px solid var(--color-accent);
		background: var(--color-accent-subtle);
		border-radius: var(--radius-md);
		padding: 12px 16px;
		margin: 0.75em 0;
	}

	/* ── Toggle Blocks ── */
	:global(.tiptap-editor details[data-toggle]) {
		margin: 0.5em 0;
	}

	:global(.tiptap-editor details[data-toggle] > summary) {
		cursor: pointer;
		font-weight: 600;
		padding: 4px 0;
		outline: none;
	}

	:global(.tiptap-editor details[data-toggle] > summary::marker) {
		color: var(--color-text-muted);
	}
</style>
