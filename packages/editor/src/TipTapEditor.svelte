<script lang="ts">
import { Editor } from '@tiptap/core';
import { editorExtensions } from './extensions.js';
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
		onChange?: () => void;
	} = $props();

	// Plain (non-reactive) handle to the tiptap instance. The editor is
	// created from a use:action, not a reactive $effect — an effect that reads
	// and writes bound state re-created the Editor endlessly
	// (effect_update_depth_exceeded in prod; a 50/s remount storm in dev).
	let _editor: Editor | undefined;
	let contentApplied = false;

	/** use:action — runs imperatively when the element mounts, destroy() on removal. */
	function mountEditor(node: HTMLElement) {
		const instance = new Editor({
			element: node,
			extensions: editorExtensions(),
			content: content as string | undefined,
			editable,
			autofocus,
			onUpdate: () => {
				// Signal-only: serializing (getJSON) here allocates a full doc
				// tree per keystroke; the page serializes once at save time.
				onChange?.();
			},
		});

		_editor = makeReactive(instance);
		boundEditor = _editor;

		// If content wasn't available at init time, apply it now
		if (content && !contentApplied) {
			const contentStr = JSON.stringify(content);
			if (contentStr !== '{"type":"doc","content":[]}') {
				instance.commands.setContent(content as any);
			}
			contentApplied = true;
		}

		return {
			destroy() {
				instance.destroy();
				_editor = undefined;
				boundEditor = undefined as unknown as typeof boundEditor;
				contentApplied = false;
			},
		};
	}

	// Apply async content that arrives after the editor has mounted. This
	// effect only reads `content` — it never writes reactive state it reads,
	// so it cannot self-invalidate.
	$effect(() => {
		if (!_editor || !content || contentApplied) return;
		const contentStr = JSON.stringify(content);
		if (contentStr !== '{"type":"doc","content":[]}') {
			_editor.commands.setContent(content as any);
		}
		contentApplied = true;
	});
</script>

<div class="editor-container">
	<div use:mountEditor class="tiptap-editor"></div>
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
		position: relative;
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

	/* ── Tables ── */
	:global(.tiptap-editor table) {
		border-collapse: collapse;
		width: 100%;
		margin: 0.5em 0;
	}

	:global(.tiptap-editor th),
	:global(.tiptap-editor td) {
		border: 1px solid var(--color-border);
		padding: 6px 10px;
		text-align: left;
		vertical-align: top;
	}

	:global(.tiptap-editor th) {
		background: var(--color-surface-hover);
		font-weight: 600;
	}

	:global(.tiptap-editor .selectedCell) {
		background: var(--color-accent-subtle);
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

	/* ── Mention chips ── */
	:global(.tiptap-editor .mention-chip) {
		display: inline;
		background: var(--color-accent-subtle);
		color: var(--color-accent);
		border-radius: 999px;
		padding: 1px 8px;
		font-weight: 500;
		cursor: pointer;
		transition: background 0.1s;
	}
	:global(.tiptap-editor .mention-chip:hover) {
		background: var(--color-accent);
		color: #fff;
	}
	:global(.tiptap-editor .mention-chip.selected) {
		background: var(--color-accent);
		color: #fff;
	}
</style>
