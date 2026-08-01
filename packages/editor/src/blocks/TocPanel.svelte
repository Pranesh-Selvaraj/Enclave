<script lang="ts">
	import type { Editor } from '@tiptap/core';

	let {
		editor,
	}: {
		editor: Editor | undefined;
	} = $props();

	let headings = $state<{ level: number; text: string; pos: number }[]>([]);
	let activePos = $state<number | null>(null);

	let timer: ReturnType<typeof setTimeout>;

	function refresh() {
		const ed = editor;
		if (!ed) return;
		const out: { level: number; text: string; pos: number }[] = [];
		ed.state.doc.descendants((node, pos) => {
			if (node.type.name === 'heading') {
				const text = node.textContent.trim();
				if (text) out.push({ level: node.attrs.level as number, text, pos });
			}
			return true;
		});
		headings = out;
	}

	$effect(() => {
		const ed = editor;
		if (!ed) return;
		refresh();
		const onTx = () => {
			// ponytail: recompute on debounce, not per transaction — O(doc)
			// per keystroke would be a return of the freeze era.
			clearTimeout(timer);
			timer = setTimeout(refresh, 250);
		};
		ed.on('transaction', onTx);
		return () => {
			ed.off('transaction', onTx);
			clearTimeout(timer);
		};
	});

	// Track the heading under the cursor for a light scroll-spy feel.
	$effect(() => {
		const ed = editor;
		if (!ed) return;
		const onSel = () => {
			const { from } = ed.state.selection;
			let current: number | null = null;
			for (const h of headings) {
				if (h.pos <= from) current = h.pos;
				else break;
			}
			activePos = current;
		};
		ed.on('selectionUpdate', onSel);
		return () => ed.off('selectionUpdate', onSel);
	});

	function jump(h: { pos: number }) {
		const ed = editor;
		if (!ed) return;
		ed.chain().focus().setTextSelection(h.pos).run();
		const dom = ed.view.nodeDOM(h.pos) as HTMLElement | null;
		dom?.scrollIntoView({ block: 'start', behavior: 'smooth' });
	}
</script>

{#if headings.length > 1}
	<aside class="toc-panel">
		<div class="toc-header">On this page</div>
		<nav class="toc-list">
			{#each headings as h (h.pos)}
				<button
					class="toc-item"
					class:active={activePos === h.pos}
					class:toc-l2={h.level === 2}
					class:toc-l3={h.level === 3}
					onclick={() => jump(h)}
					title={h.text}
				>
					{h.text}
				</button>
			{/each}
		</nav>
	</aside>
{/if}

<style>
	.toc-panel {
		width: 180px;
		flex-shrink: 0;
		border-left: 1px solid var(--color-border);
		padding-left: 16px;
		padding-top: 12px;
		overflow-y: auto;
	}

	.toc-header {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
		margin-bottom: 10px;
	}

	.toc-list {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.toc-item {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		font-family: inherit;
		text-align: left;
		padding: 3px 8px;
		border-radius: 5px;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		transition: background 0.1s, color 0.1s;
	}

	.toc-item:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	.toc-item.active {
		color: var(--color-accent);
	}

	.toc-l2 {
		padding-left: 18px;
	}

	.toc-l3 {
		padding-left: 30px;
	}
</style>
