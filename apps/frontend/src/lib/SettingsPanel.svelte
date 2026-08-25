<script lang="ts">
	import { invoke } from '$lib/backend.js';
	import { Button } from '@enclave/ui';
	import { theme, ACCENTS, FONTS, DENSITIES } from '@enclave/ui';
	import { loadAISettings, saveAISettings, listModels, type AISettings } from './ai.js';
	import { loadUpdatePrefs, saveUpdatePrefs } from './updates.js';
	import UpdateDialog from './UpdateDialog.svelte';

	let {
		open = $bindable(false),
		onlock,
	}: {
		open?: boolean;
		onlock?: () => void;
	} = $props();

	let vaultPath = $state('~/.local/share/com.enclave.app/enclave.db');
	let appVersion = $state('');

	$effect(() => {
		if (open && !appVersion) {
			invoke<string>('app_version').then((v) => (appVersion = v)).catch(() => {});
		}
	});

	// ── Updates: opt-in, offline-first ──
	// Enclave never checks for updates on its own. The user must (1) allow
	// checks here, and (2) approve each update in the changelog dialog.
	let checkUpdates = $state(loadUpdatePrefs());
	let updateDialogOpen = $state(false);

	function onCheckUpdatesChange() {
		saveUpdatePrefs(checkUpdates);
	}

	async function lockVault() {
		try {
			await invoke('lock_vault');
			open = false;
			onlock?.();
		} catch { /* ignore */ }
	}

	let backingUp = $state(false);
	let backupMsg = $state('');

	async function backupVault() {
		backingUp = true;
		backupMsg = '';
		try {
			const path = await invoke<string>('backup_vault');
			backupMsg = `Backup saved to ${path}`;
		} catch (e: any) {
			backupMsg = `Backup failed: ${e?.message || e}`;
		} finally {
			backingUp = false;
		}
	}

	// ── Local AI (any OpenAI-compatible endpoint, optional offline embeddings) ──
	let ai = $state<AISettings>(defaultAiSettings());
	let installedModels = $state<string[]>([]);
	let aiStatus = $state('');

	function defaultAiSettings(): AISettings {
		return { enabled: false, url: 'http://localhost:11434', model: 'llama3.2', apiKey: '', rag: true, builtinEmbeddings: false };
	}

	$effect(() => {
		loadAISettings().then((s) => {
			ai = s;
			if (s.enabled) refreshModels();
		});
	});

	async function refreshModels() {
		aiStatus = '';
		try {
			installedModels = await listModels(ai.url, ai.apiKey);
			if (installedModels.length === 0) aiStatus = 'Connected — no models available on this endpoint.';
			else if (!installedModels.includes(ai.model)) aiStatus = `Tip: model "${ai.model}" not available here.`;
		} catch (e: any) {
			installedModels = [];
			aiStatus = `LLM not reachable at ${ai.url} — start it and retry. (${e?.message || e})`;
		}
	}

	function onAiChange() {
		void saveAISettings(ai);
		if (ai.enabled) refreshModels();
	}

	function handleBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Settings" onclick={() => (open = false)} onkeydown={handleBackdropKeydown}>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="settings-panel" role="document" onclick={(e: MouseEvent) => e.stopPropagation()}>
			<div class="settings-header">
				<h2>Settings</h2>
				<button class="settings-close" onclick={() => (open = false)}>✕</button>
			</div>

			<div class="settings-section">
				<h3>Appearance</h3>
				<div class="setting-row">
					<span>Theme</span>
					<button class="theme-toggle" onclick={() => theme.toggle()}>
						{theme.value === 'dark' ? '☀ Light' : '🌙 Dark'}
					</button>
				</div>
				<div class="setting-row">
					<span>Accent color</span>
					<div class="swatch-row">
						{#each ACCENTS as a (a.id)}
							<button
								class="swatch"
								class:active={theme.accent === a.id}
								style={`background: ${a.color}`}
								title={a.id}
								aria-label={`Accent ${a.id}`}
								onclick={() => (theme.accent = a.id)}
							></button>
						{/each}
					</div>
				</div>
				<div class="setting-row">
					<span>Font</span>
					<div class="seg-row">
						{#each FONTS as f (f)}
							<button class="seg" class:active={theme.font === f} onclick={() => (theme.font = f)}>{f}</button>
						{/each}
					</div>
				</div>
				<div class="setting-row">
					<span>Sidebar width</span>
					<div class="seg-row">
						{#each DENSITIES as d (d)}
							<button class="seg" class:active={theme.density === d} onclick={() => (theme.density = d)}>{d}</button>
						{/each}
					</div>
				</div>
			</div>

			<div class="settings-section">
				<h3>Security</h3>
				<div class="setting-row">
					<span>Lock vault</span>
					<button class="danger-btn" onclick={lockVault}>Lock now</button>
				</div>
			</div>

			<div class="settings-section">
				<h3>Updates</h3>
				<div class="setting-row">
					<span>Allow checking for updates?</span>
					<label class="switch">
						<input type="checkbox" bind:checked={checkUpdates} onchange={onCheckUpdatesChange} />
						<span class="switch-slider"></span>
					</label>
				</div>
				<div class="setting-row">
					<span>Check now</span>
					<Button onclick={() => (updateDialogOpen = true)} disabled={!checkUpdates}>Check</Button>
				</div>
				{#if checkUpdates}
					<div class="backup-hint">
						Checking contacts GitHub Releases once. You'll review the changelog and
						approve <b>every</b> update before anything is downloaded — nothing is ever
						installed automatically.
					</div>
				{:else}
					<div class="backup-hint">
						Off by default: Enclave is offline-first and never phones home.
					</div>
				{/if}

				<div class="sentinel-card">
					<div class="sentinel-title">🛰 Keep Enclave fully offline — monitor updates with <b>Sentinel</b></div>
					<div class="backup-hint">
						Sentinel is a free, open-source CLI (Python 3.11+, zero dependencies) that
						watches GitHub releases from your terminal, so Enclave itself never needs
						internet access:
						<pre class="sentinel-cmd">sentinel add github Pranesh-Selvaraj/Enclave --monitor release
sentinel check</pre>
						It remembers what it watched, tells you exactly what changed, and only you
						decide when (or whether) to update. See
						github.com/Pranesh-Selvaraj/Sentinel.
					</div>
				</div>
				<UpdateDialog bind:open={updateDialogOpen} />
			</div>

			<div class="settings-section">
				<h3>AI assistant</h3>
				<div class="setting-row">
					<span>Enable AI</span>
					<label class="switch">
						<input type="checkbox" bind:checked={ai.enabled} onchange={onAiChange} />
						<span class="switch-slider"></span>
					</label>
				</div>
				{#if ai.enabled}
					<div class="setting-row">
						<span>Endpoint URL</span>
						<input class="ai-input" bind:value={ai.url} onchange={onAiChange} aria-label="Endpoint URL" placeholder="http://localhost:11434" />
					</div>
					<div class="setting-row">
						<span>API key (optional)</span>
						<input class="ai-input" type="password" bind:value={ai.apiKey} onchange={onAiChange} aria-label="API key" placeholder="sk-… (frontier APIs only)" />
					</div>
					<div class="setting-row">
						<span>Vault-wide answers (RAG)</span>
						<label class="switch">
							<input type="checkbox" bind:checked={ai.rag} onchange={onAiChange} />
							<span class="switch-slider"></span>
						</label>
					</div>
					<div class="setting-row">
						<span>Offline embeddings</span>
						<label class="switch" title="Embed with the built-in model — no endpoint needed for RAG (downloads ~25 MB once)">
							<input type="checkbox" bind:checked={ai.builtinEmbeddings} onchange={onAiChange} />
							<span class="switch-slider"></span>
						</label>
					</div>
					<div class="setting-row">
						<span>Model</span>
						<input
							class="ai-input"
							bind:value={ai.model}
							onchange={onAiChange}
							aria-label="Model name"
							list="ai-models"
						/>
						<datalist id="ai-models">
							{#each installedModels as m (m)}
								<option value={m}></option>
							{/each}
						</datalist>
					</div>
					<div class="ai-status-row">
						<Button onclick={refreshModels}>Check connection</Button>
						{#if aiStatus}<span class="ai-status" role="status">{aiStatus}</span>{/if}
					</div>
					<div class="backup-hint">Point this at any OpenAI-compatible server — Ollama, llama.cpp, LM Studio, vLLM on this machine, or a frontier API (add its key above). With <b>offline embeddings</b> on, RAG retrieval needs no endpoint at all; chat still does.</div>
				{/if}
			</div>

			<div class="settings-section">
				<h3>Backup</h3>
				<div class="setting-row">
					<span>Export encrypted vault backup</span>
					<Button onclick={backupVault} disabled={backingUp}>{backingUp ? 'Backing up…' : 'Back up'}</Button>
				</div>
				{#if backupMsg}
					<div class="backup-msg" role="status">{backupMsg}</div>
				{/if}
				<div class="backup-hint">
					Backups are saved to the exports folder. To restore, close Enclave and replace
					<code>enclave.db</code> with the backup file.
				</div>
			</div>

			<div class="settings-section">
				<h3>Keyboard Shortcuts</h3>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>K</kbd> <span>Command palette</span></div>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>N</kbd> <span>New page</span></div>
				<div class="shortcut-row"><kbd>Ctrl</kbd>+<kbd>B</kbd> <span>Toggle sidebar</span></div>
				<div class="shortcut-row"><kbd>/</kbd> <span>Slash commands in editor</span></div>
				<div class="shortcut-row"><kbd>[[</kbd> <span>Link to page</span></div>
			</div>

			<div class="settings-section">
				<h3>About</h3>
				<div class="about-info">
					<div class="about-row"><span>Version</span><span class="about-value">{appVersion || '…'}</span></div>
					<div class="about-row"><span>Vault</span><span class="about-value">{vaultPath || '~/.local/share/com.enclave.app/'}</span></div>
				</div>
			</div>

			<div class="settings-footer">
				<Button onclick={() => (open = false)}>Close</Button>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		z-index: 300;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.settings-panel {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		width: 420px;
		max-width: 100%;
		max-height: min(90vh, 760px);
		overflow-y: auto;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.3);
	}

	.settings-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid var(--color-border);
	}
	.settings-header h2 { font-size: 16px; font-weight: 600; margin: 0; }
	.settings-close {
		background: none; border: none; color: var(--color-text-muted);
		cursor: pointer; font-size: 16px; padding: 4px;
	}
	.settings-section {
		padding: 12px 20px;
		border-bottom: 1px solid var(--color-border);
	}
	.settings-section h3 {
		font-size: 11px; font-weight: 600; text-transform: uppercase;
		letter-spacing: 0.05em; color: var(--color-text-muted); margin: 0 0 8px;
	}
	.setting-row {
		display: flex; align-items: center; justify-content: space-between; font-size: 14px;
	}
	.theme-toggle {
		background: var(--color-surface-hover); border: 1px solid var(--color-border);
		border-radius: var(--radius-md); color: var(--color-text);
		padding: 4px 12px; font-size: 13px; cursor: pointer; font-family: inherit;
	}
	.setting-row { margin: 10px 0; }
	.swatch-row { display: flex; gap: 6px; }
	.swatch {
		width: 20px; height: 20px; border-radius: 50%; border: 2px solid transparent;
		cursor: pointer; padding: 0;
	}
	.swatch.active { border-color: var(--color-text); }
	.swatch:hover { transform: scale(1.15); }
	.seg-row { display: flex; gap: 4px; }
	.seg {
		background: var(--color-surface-hover); border: 1px solid var(--color-border);
		border-radius: var(--radius-sm); color: var(--color-text-muted);
		padding: 3px 10px; font-size: 12px; cursor: pointer; font-family: inherit;
		text-transform: capitalize;
	}
	.seg.active { background: var(--color-accent-subtle); color: var(--color-accent); border-color: var(--color-accent); }
	.danger-btn {
		background: none; border: 1px solid var(--color-danger); color: var(--color-danger);
		border-radius: var(--radius-md); padding: 4px 12px; font-size: 13px;
		cursor: pointer; font-family: inherit;
	}
	.danger-btn:hover { background: var(--color-danger); color: white; }
	.switch { position: relative; display: inline-block; width: 36px; height: 20px; }
	.switch input { opacity: 0; width: 0; height: 0; }
	.switch-slider {
		position: absolute; inset: 0; border-radius: 999px;
		background: var(--color-surface-active); transition: background 0.15s; cursor: pointer;
	}
	.switch-slider::before {
		content: ''; position: absolute; width: 14px; height: 14px; border-radius: 50%;
		left: 3px; top: 3px; background: var(--color-surface); transition: transform 0.15s;
	}
	.switch input:checked + .switch-slider { background: var(--color-accent); }
	.switch input:checked + .switch-slider::before { transform: translateX(16px); background: #fff; }
	.ai-input {
		background: var(--color-surface-hover); border: 1px solid var(--color-border);
		border-radius: var(--radius-sm); color: var(--color-text);
		font-size: 13px; font-family: var(--font-mono); padding: 3px 8px; width: 190px;
	}
	.ai-status-row { display: flex; align-items: center; gap: 8px; margin-top: 8px; }
	.ai-status { font-size: 12px; color: var(--color-text-muted); line-height: 1.4; }
	.backup-msg { margin-top: 8px; font-size: 12px; color: var(--color-text-muted); word-break: break-all; }
	.backup-hint { margin-top: 8px; font-size: 12px; color: var(--color-text-faint); line-height: 1.5; }
	.backup-hint code {
		background: var(--color-surface-hover); border: 1px solid var(--color-border);
		border-radius: 3px; padding: 0 4px; font-size: 11px; font-family: var(--font-mono);
	}
	.sentinel-card {
		margin-top: 10px;
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		padding: 10px 12px;
	}
	.sentinel-title { font-size: 13px; color: var(--color-text); line-height: 1.5; }
	.sentinel-cmd {
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		padding: 8px 10px;
		margin: 8px 0;
		font-size: 11px;
		line-height: 1.6;
		font-family: var(--font-mono);
		color: var(--color-text-muted);
		overflow-x: auto;
		white-space: pre;
	}
	.shortcut-row {
		display: flex; align-items: center; gap: 6px;
		font-size: 13px; color: var(--color-text-muted); margin: 6px 0;
	}
	.shortcut-row kbd {
		background: var(--color-surface-hover); border: 1px solid var(--color-border);
		border-radius: 3px; padding: 1px 5px; font-size: 11px; font-family: var(--font-mono);
	}
	.about-info { display: flex; flex-direction: column; gap: 6px; }
	.about-row { display: flex; justify-content: space-between; font-size: 13px; color: var(--color-text-muted); }
	.about-value { color: var(--color-text); font-family: var(--font-mono); font-size: 12px; max-width: 220px; overflow: hidden; text-overflow: ellipsis; }
	.settings-footer {
		padding: 14px 20px; display: flex; justify-content: flex-end;
	}

	/* Full-screen sheet on phones — the panel is long and a centered modal
	 * overflows (and clips) on small screens. */
	@media (max-width: 768px) {
		.modal-backdrop { align-items: flex-end; padding: 0; }
		.settings-panel {
			width: 100%;
			max-width: 100%;
			max-height: 92vh;
			border-radius: 16px 16px 0 0;
			border-bottom: none;
		}
	}
</style>
