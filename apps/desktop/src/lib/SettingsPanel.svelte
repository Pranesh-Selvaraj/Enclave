<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '@enclave/ui';
	import { theme, ACCENTS, FONTS, DENSITIES } from '@enclave/ui';
	import { loadAISettings, saveAISettings, listModels, type AISettings } from './ollama.js';

	let {
		open = $bindable(false),
		onlock,
	}: {
		open?: boolean;
		onlock?: () => void;
	} = $props();

	let vaultPath = $state('~/.local/share/com.enclave.app/enclave.db');
	let appVersion = $state('0.4.0');

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

	// ── Local AI (Ollama) ──
	let ai = $state<AISettings>(loadAISettings());
	let installedModels = $state<string[]>([]);
	let aiStatus = $state('');

	async function refreshModels() {
		aiStatus = '';
		try {
			installedModels = await listModels(ai.url);
			if (installedModels.length === 0) aiStatus = 'Connected — no models available on this endpoint.';
			else if (!installedModels.includes(ai.model)) aiStatus = `Tip: model "${ai.model}" not available here.`;
		} catch (e: any) {
			installedModels = [];
			aiStatus = `LLM not reachable at ${ai.url} — start it and retry. (${e?.message || e})`;
		}
	}

	function onAiChange() {
		saveAISettings(ai);
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
				<h3>AI assistant (local)</h3>
				<div class="setting-row">
					<span>Enable local AI</span>
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
						<span>Vault-wide answers (RAG)</span>
						<label class="switch">
							<input type="checkbox" bind:checked={ai.rag} onchange={onAiChange} />
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
					<div class="backup-hint">Requires a local OpenAI-compatible endpoint (Ollama, llama.cpp, LM Studio, vllm…) on this machine. Content never leaves your device.</div>
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
					<div class="about-row"><span>Version</span><span class="about-value">{appVersion}</span></div>
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
</style>
