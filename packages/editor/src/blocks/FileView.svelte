<script lang="ts">
	// File block — a stored vault attachment (PDFs today) with an inline
	// preview where the webview supports it and an "Open" button that hands
	// the file to the system viewer (Android uses the ACTION_VIEW intent via
	// tauri-plugin-opener).
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { openPath } from '@tauri-apps/plugin-opener';
	import { Icon } from '@enclave/ui';

	let {
		path,
		name,
	}: {
		path: string;
		name: string;
	} = $props();

	let src = $derived(convertFileSrc(path));
	let opening = $state(false);

	function fmtName() {
		return name || path.split(/[\\/]/).pop() || 'file';
	}

	async function open() {
		if (opening) return;
		opening = true;
		try {
			await openPath(path);
		} catch (e) {
			console.error('Failed to open file:', e);
		} finally {
			setTimeout(() => (opening = false), 800);
		}
	}
</script>

<div class="file-block">
	<div class="file-bar">
		<span class="file-icon"><Icon name="page" size={16} /></span>
		<span class="file-name">{fmtName()}</span>
		<button class="file-open" onclick={open} title="Open in your system viewer">
			<Icon name="externalLink" size={13} />
			{opening ? 'Opening…' : 'Open'}
		</button>
	</div>
	<iframe class="pdf-frame" src={src} title={fmtName()} loading="lazy"></iframe>
</div>

<style>
	.file-block {
		margin: 0.75em 0;
		border: 1px solid var(--color-border);
		border-radius: 12px;
		overflow: hidden;
		background: var(--color-surface);
	}
	.file-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface-hover);
	}
	.file-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border-radius: 8px;
		background: color-mix(in srgb, var(--color-accent) 14%, transparent);
		color: var(--color-accent);
		flex-shrink: 0;
	}
	.file-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 13px;
		font-weight: 500;
		color: var(--color-text);
	}
	.file-open {
		display: flex;
		align-items: center;
		gap: 5px;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		font-family: inherit;
		padding: 4px 10px;
		cursor: pointer;
		flex-shrink: 0;
	}
	.file-open:hover {
		color: var(--color-text);
		background: var(--color-surface-active);
	}
	.pdf-frame {
		width: 100%;
		height: 560px;
		border: none;
		display: block;
		background: var(--color-bg);
	}
	@media (max-width: 768px) {
		.pdf-frame { height: 420px; }
	}
</style>
