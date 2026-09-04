<script lang="ts">
	// Custom in-app window chrome (desktop only, main window). The native
	// decorations are off (src-tauri/tauri.conf.json) so this bar owns
	// dragging + min/max/close. Middle is a data-tauri-drag-region; double
	// click toggles maximize. Web/preview falls back to inert controls.
	import { Icon, Logo } from '@enclave/ui';
	import { isTauri } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	// Resize direction identifiers accepted by startResizeDragging.
	type ResizeDir =
		| 'ResizeNorth' | 'ResizeSouth' | 'ResizeEast' | 'ResizeWest'
		| 'ResizeNorthEast' | 'ResizeNorthWest' | 'ResizeSouthEast' | 'ResizeSouthWest';

	let { onSearch }: { onSearch: () => void } = $props();

	const win = isTauri() ? getCurrentWindow() : null;
	let maximized = $state(false);

	function minimize() { win?.minimize(); }
	function toggleMax() { win?.toggleMaximize(); }
	function close() { win?.close(); }

	// Frameless windows don't get OS resize borders — these invisible edges
	// call into the native window manager so drag-resizing still works.
	function startResize(dir: ResizeDir) {
		(win as unknown as { startResizeDragging: (d: ResizeDir) => Promise<void> })?.startResizeDragging(dir).catch(() => {});
	}

	$effect(() => {
		if (!win) return;
		const check = () => { win.isMaximized().then((m) => (maximized = m)).catch(() => {}); };
		check();
		let un: (() => void) | undefined;
		win.onResized(check).then((fn) => (un = fn));
		return () => un?.();
	});

	// macOS convention ⌘K, elsewhere Ctrl K.
	const modKey = $derived(typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform) ? '⌘K' : 'Ctrl K');
</script>

<header class="titlebar">
	<a class="tb-brand" href="/" aria-label="Enclave home" title="Enclave">
		<span class="tb-logo"><Logo size={17} /></span>
		<span class="tb-name">Enclave</span>
	</a>

	<!-- Draggable chrome: double-click toggles maximize/restore. -->
	<div class="tb-drag" data-tauri-drag-region ondblclick={toggleMax} aria-hidden="true"></div>

	<button class="tb-search" onclick={onSearch} title="Search pages (Ctrl K)">
		<Icon name="search" size={13} />
		<span class="tb-search-txt">Search</span>
		<kbd class="tb-kbd">{modKey}</kbd>
	</button>

	<div class="tb-controls">
		<button class="tb-win" onclick={minimize} aria-label="Minimize window" title="Minimize">
			<Icon name="minus" size={15} />
		</button>
		<button class="tb-win" onclick={toggleMax} aria-label={maximized ? 'Restore window' : 'Maximize window'} title={maximized ? 'Restore' : 'Maximize'}>
			<Icon name={maximized ? 'corners' : 'square'} size={12} />
		</button>
		<button class="tb-win tb-close" onclick={close} aria-label="Close window" title="Close">
			<Icon name="x" size={15} />
		</button>
	</div>
</header>

{#if win}
	<!-- Invisible resize edges/corners for the frameless main window. -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="rz"
		role="presentation"
		aria-hidden="true"
		onpointerdown={(e: PointerEvent) => {
			const dir = (e.target as HTMLElement | null)?.dataset?.dir as ResizeDir | undefined;
			if (dir) { e.preventDefault(); startResize(dir); }
		}}
	>
		<span class="rz-t" data-dir="ResizeNorth"></span>
		<span class="rz-b" data-dir="ResizeSouth"></span>
		<span class="rz-l" data-dir="ResizeWest"></span>
		<span class="rz-r" data-dir="ResizeEast"></span>
		<span class="rz-nw" data-dir="ResizeNorthWest"></span>
		<span class="rz-ne" data-dir="ResizeNorthEast"></span>
		<span class="rz-sw" data-dir="ResizeSouthWest"></span>
		<span class="rz-se" data-dir="ResizeSouthEast"></span>
	</div>
{/if}

<style>
	.titlebar {
		flex: 0 0 42px;
		display: flex;
		align-items: stretch;
		background: var(--color-titlebar);
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text-muted);
		user-select: none;
		-webkit-user-select: none;
	}

	.tb-brand {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 14px 0 16px;
		color: var(--color-text);
		text-decoration: none;
		font-size: 13px;
		font-weight: 600;
		letter-spacing: -0.01em;
		flex-shrink: 0;
	}
	.tb-brand:hover .tb-name { color: var(--color-accent); }
	.tb-logo { display: flex; color: var(--color-accent); }
	.tb-name { transition: color 0.12s; }

	/* Whole middle is draggable; double click = maximize. */
	.tb-drag { flex: 1; min-width: 40px; }

	.tb-search {
		display: flex;
		align-items: center;
		gap: 7px;
		align-self: center;
		margin: 0 10px 0 0;
		padding: 0 9px;
		height: 26px;
		min-width: 120px;
		border: 1px solid var(--color-border);
		border-radius: 7px;
		background: var(--color-inset);
		color: var(--color-text-faint);
		font-size: 12px;
		font-family: inherit;
		cursor: pointer;
		transition: border-color 0.12s, background 0.12s, color 0.12s;
	}
	.tb-search:hover {
		border-color: var(--color-border-strong);
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
	}
	.tb-kbd {
		margin-left: auto;
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-text-faint);
		background: var(--color-surface-active);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 1px 5px;
	}

	.tb-controls {
		display: flex;
		align-items: stretch;
		flex-shrink: 0;
	}
	.tb-win {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 44px;
		border: none;
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0;
		transition: background 0.1s, color 0.1s;
	}
	.tb-win:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.tb-win.tb-close:hover { background: var(--color-danger); color: #fff; }

	/* Narrow-but-still-desktop windows: tighten the search affordance. */
	@media (max-width: 1150px) {
		.tb-kbd { display: none; }
	}
	@media (max-width: 920px) {
		.tb-search { min-width: 30px; width: 30px; justify-content: center; padding: 0; }
		.tb-search-txt { display: none; }
	}

	/* ── Frameless resize edges (native OS resize borders are gone) ── */
	.rz { position: fixed; inset: 0; z-index: 9999; pointer-events: none; }
	.rz span { position: fixed; pointer-events: auto; }
	.rz-t { top: 0; left: 8px; right: 8px; height: 5px; cursor: ns-resize; }
	.rz-b { bottom: 0; left: 8px; right: 8px; height: 6px; cursor: ns-resize; }
	.rz-l { left: 0; top: 8px; bottom: 8px; width: 5px; cursor: ew-resize; }
	.rz-r { right: 0; top: 8px; bottom: 8px; width: 6px; cursor: ew-resize; }
	.rz-nw { top: 0; left: 0; width: 12px; height: 12px; cursor: nwse-resize; }
	.rz-ne { top: 0; right: 0; width: 12px; height: 12px; cursor: nesw-resize; }
	.rz-sw { bottom: 0; left: 0; width: 14px; height: 14px; cursor: nesw-resize; }
	.rz-se { bottom: 0; right: 0; width: 14px; height: 14px; cursor: nwse-resize; }
</style>
