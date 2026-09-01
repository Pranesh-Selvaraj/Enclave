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
		font-size: 1.8em;
		font-weight: 700;
		margin: 0.4em 0 0.2em;
	}

	:global(.tiptap-editor h2) {
		font-size: 1.35em;
		font-weight: 600;
		margin: 0.4em 0 0.2em;
	}

	:global(.tiptap-editor h3) {
		font-size: 1.15em;
		font-weight: 600;
		margin: 0.3em 0 0.15em;
	}

	:global(.tiptap-editor p) {
		margin: 0.2em 0;
		line-height: 1.55;
	}

	:global(.tiptap-editor ul, .tiptap-editor ol) {
		padding-left: 1.4em;
		margin: 0.2em 0;
	}

	:global(.tiptap-editor li) {
		margin: 0.1em 0;
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
		padding: 0.8em;
		margin: 0.4em 0;
		overflow-x: auto;
	}

	:global(.tiptap-editor pre code) {
		background: none;
		padding: 0;
	}

	:global(.tiptap-editor hr) {
		border: none;
		border-top: 1px solid var(--color-border);
		margin: 0.7em 0;
	}

	/* ── Tables ── */
	:global(.tiptap-editor table) {
		border-collapse: collapse;
		width: 100%;
		margin: 0.4em 0;
	}

	:global(.tiptap-editor th),
	:global(.tiptap-editor td) {
		border: 1px solid var(--color-border);
		padding: 4px 8px;
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

	/* ── Task Lists (Obsidian-style checkboxes) ── */
	:global(.tiptap-editor ul[data-type="taskList"]) {
		list-style: none;
		padding-left: 0;
		margin: 0.25em 0;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li) {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		margin: 0.2em 0;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li > div) {
		flex: 1;
		min-width: 0;
	}

	/* Custom checkbox: the native input is invisible but keeps focus/click;
	   the label's span is the visible box. */
	:global(.tiptap-editor ul[data-type="taskList"] li label) {
		position: relative;
		width: 18px;
		height: 18px;
		margin-top: 3px;
		flex-shrink: 0;
		cursor: pointer;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li input[type="checkbox"]) {
		position: absolute;
		inset: 0;
		margin: 0;
		opacity: 0;
		cursor: pointer;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li label span) {
		position: absolute;
		inset: 0;
		border: 1.5px solid var(--color-border-strong);
		border-radius: 5px;
		background: var(--color-surface);
		transition: background 0.12s, border-color 0.12s;
		pointer-events: none;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li label span::after) {
		content: '';
		position: absolute;
		left: 5px;
		top: 2px;
		width: 5px;
		height: 9px;
		border: solid #fff;
		border-width: 0 2px 2px 0;
		transform: rotate(45deg) scale(0);
		transition: transform 0.12s;
	}

	:global(.tiptap-editor ul[data-type="taskList"] li input:checked + span) {
		background: var(--color-accent);
		border-color: var(--color-accent);
	}

	:global(.tiptap-editor ul[data-type="taskList"] li input:checked + span::after) {
		transform: rotate(45deg) scale(1);
	}

	:global(.tiptap-editor ul[data-type="taskList"] li:hover input + span) {
		border-color: var(--color-accent);
	}

	:global(.tiptap-editor ul[data-type="taskList"] li[data-checked="true"] > div) {
		text-decoration: line-through;
		color: var(--color-text-muted);
	}

	/* ── Callouts (Obsidian-style, colored per type) ── */
	:global(.tiptap-editor [data-callout]) {
		border-left: 4px solid var(--color-accent);
		background: color-mix(in srgb, var(--color-accent) 7%, transparent);
		border-radius: var(--radius-md);
		padding: 9px 14px;
		margin: 0.5em 0;
	}

	:global(.tiptap-editor [data-callout] > :first-child) { margin-top: 0; }
	:global(.tiptap-editor [data-callout] > :last-child) { margin-bottom: 0; }

	:global(.tiptap-editor [data-callout][data-type="tip"]) {
		border-color: var(--color-success);
		background: color-mix(in srgb, var(--color-success) 8%, transparent);
	}

	:global(.tiptap-editor [data-callout][data-type="warning"]) {
		border-color: var(--color-warning);
		background: color-mix(in srgb, var(--color-warning) 8%, transparent);
	}

	:global(.tiptap-editor [data-callout][data-type="danger"]) {
		border-color: var(--color-danger);
		background: color-mix(in srgb, var(--color-danger) 8%, transparent);
	}

	/* ── Blockquotes ── */
	:global(.tiptap-editor blockquote) {
		border-left: 3px solid var(--color-border-strong);
		padding: 1px 14px;
		margin: 0.5em 0;
		color: var(--color-text-muted);
	}

	/* ── Toggle Blocks ── */
	:global(.tiptap-editor .toggle-block) {
		display: flex;
		align-items: flex-start;
		gap: 4px;
		margin: 0.2em 0;
	}

	:global(.tiptap-editor .toggle-chevron) {
		flex-shrink: 0;
		width: 24px;
		height: 24px;
		margin-top: 1px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--color-text-faint);
		cursor: pointer;
		padding: 0;
		transition: background 0.1s, color 0.1s;
	}

	:global(.tiptap-editor .toggle-chevron:hover) {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	:global(.tiptap-editor .toggle-body) {
		flex: 1;
		min-width: 0;
	}

	:global(.tiptap-editor .toggle-summary) {
		font-weight: 600;
		padding: 3px 0;
		outline: none;
	}

	:global(.tiptap-editor .toggle-block.collapsed .toggle-body > :not(.toggle-summary)) {
		display: none;
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
