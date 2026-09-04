<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { Icon } from '@enclave/ui';

	let {
		editor,
	}: {
		editor: Editor | undefined;
	} = $props();

	let visible = $state(false);
	let position = $state({ x: 0, y: 0 });
	let isMouseOverMenu = $state(false);
	let active = $state({ bold: false, italic: false, strike: false, code: false });

	function updateMenu() {
		if (!editor) return;
		const { from, to, empty } = editor.state.selection;
		if (empty) {
			visible = false;
			return;
		}
		// Inside a table the TableMenu (row/column controls) takes over.
		if (editor.isActive('table')) {
			visible = false;
			return;
		}

		// Hoist isActive reads out of the template: with the reactive editor
		// proxy they'd re-subscribe on every transaction and rerender on each
		// keystroke.
		active = {
			bold: editor.isActive('bold'),
			italic: editor.isActive('italic'),
			strike: editor.isActive('strike'),
			code: editor.isActive('code'),
		};

		const start = editor.view.coordsAtPos(from);
		const end = editor.view.coordsAtPos(to);

		// Fixed + viewport coords (see SlashMenu) — never detached from the
		// selection, clamped to the screen.
		position = {
			x: Math.min(Math.max((start.left + end.right) / 2 - 120, 8), window.innerWidth - 248),
			y: Math.min(Math.max(start.top - 48, 8), window.innerHeight - 56),
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
		onpointerdown={() => isMouseOverMenu = true}
		onmouseenter={() => isMouseOverMenu = true}
		onmouseleave={() => { isMouseOverMenu = false; visible = false; }}
		role="toolbar"
		aria-label="Text formatting"
	>
		<button
			class="bubble-btn"
			class:active={active.bold}
			onclick={toggleBold}
			aria-label="Bold"
		>
			<Icon name="bold" size={16} />
		</button>
		<button
			class="bubble-btn"
			class:active={active.italic}
			onclick={toggleItalic}
			aria-label="Italic"
		>
			<Icon name="italic" size={16} />
		</button>
		<button
			class="bubble-btn"
			class:active={active.strike}
			onclick={toggleStrike}
			aria-label="Strikethrough"
		>
			<Icon name="strike" size={16} />
		</button>
		<div class="bubble-divider"></div>
		<button
			class="bubble-btn"
			class:active={active.code}
			onclick={toggleCode}
			aria-label="Inline code"
		>
			<Icon name="code" size={16} />
		</button>
	</div>
{/if}

<style>
	.bubble-menu {
		position: fixed;
		z-index: 300;
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
		background-color: color-mix(in srgb, var(--color-accent) 16%, transparent);
	}

	.bubble-divider {
		width: 1px;
		height: 20px;
		background: var(--color-border);
		margin: 0 4px;
	}
</style>
