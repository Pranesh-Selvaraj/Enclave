<script lang="ts">
	import type { Editor } from '@tiptap/core';

	let {
		editor,
	}: {
		editor: Editor | undefined;
	} = $props();

	let visible = $state(false);
	let position = $state({ x: 0, y: 0 });
	let isMouseOverMenu = $state(false);

	function updateMenu() {
		if (!editor) return;
		const { from, to, empty } = editor.state.selection;
		if (empty) {
			visible = false;
			return;
		}

		const start = editor.view.coordsAtPos(from);
		const end = editor.view.coordsAtPos(to);
		const editorEl = editor.view.dom.closest('.editor-container');
		const editorRect = editorEl?.getBoundingClientRect();

		position = {
			x: (start.left + end.right) / 2 - (editorRect?.left ?? 0) - 120,
			y: start.top - (editorRect?.top ?? 0) - 48,
		};
		visible = true;
	}

	function toggleBold() {
		editor?.chain().focus().toggleBold().run();
	}

	function toggleItalic() {
		editor?.chain().focus().toggleItalic().run();
	}

	function toggleStrike() {
		editor?.chain().focus().toggleStrike().run();
	}

	function toggleCode() {
		editor?.chain().focus().toggleCode().run();
	}

	$effect(() => {
		const ed = editor;
		if (!ed) return;

		function onBlur() {
			if (!isMouseOverMenu) {
				visible = false;
			}
		}

		ed.on('selectionUpdate', updateMenu);
		ed.on('blur', onBlur);

		return () => {
			ed.off('selectionUpdate', updateMenu);
			ed.off('blur', onBlur);
		};
	});
</script>

{#if visible && editor}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="bubble-menu"
		style="left: {position.x}px; top: {position.y}px;"
		onmouseenter={() => isMouseOverMenu = true}
		onmouseleave={() => { isMouseOverMenu = false; visible = false; }}
		role="toolbar"
		aria-label="Text formatting"
	>
		<button
			class="bubble-btn"
			class:active={editor.isActive('bold')}
			onclick={toggleBold}
			aria-label="Bold"
		>
			<strong>B</strong>
		</button>
		<button
			class="bubble-btn"
			class:active={editor.isActive('italic')}
			onclick={toggleItalic}
			aria-label="Italic"
		>
			<em>I</em>
		</button>
		<button
			class="bubble-btn"
			class:active={editor.isActive('strike')}
			onclick={toggleStrike}
			aria-label="Strikethrough"
		>
			<s>S</s>
		</button>
		<div class="bubble-divider"></div>
		<button
			class="bubble-btn"
			class:active={editor.isActive('code')}
			onclick={toggleCode}
			aria-label="Inline code"
		>
			{'</>'}
		</button>
	</div>
{/if}

<style>
	.bubble-menu {
		position: absolute;
		z-index: 100;
		display: flex;
		align-items: center;
		gap: 2px;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 4px;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
	}

	.bubble-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: 5px;
		background: none;
		color: var(--color-text);
		cursor: pointer;
		font-size: 14px;
		transition: background-color 0.1s;
	}

	.bubble-btn:hover {
		background-color: rgba(255, 255, 255, 0.06);
	}

	.bubble-btn.active {
		color: var(--color-accent);
		background-color: rgba(124, 111, 240, 0.15);
	}

	.bubble-divider {
		width: 1px;
		height: 20px;
		background: var(--color-border);
		margin: 0 4px;
	}
</style>
