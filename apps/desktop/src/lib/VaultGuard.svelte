<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { generateMnemonic, validateMnemonic, deriveMasterKey, selfCheck, encryptWithPassword, decryptWithPassword, type EncryptedNote } from '@enclave/crypto';
	import { Button } from '@enclave/ui';

	let { onunlock }: { onunlock: () => void } = $props();

	type Step = 'loading' | 'checking' | 'welcome' | 'create-password' | 'create-seed' | 'unlock' | 'setup-password' | 'error';
	let step = $state<Step>('loading');
	let errorMsg = $state('');

	let password = $state('');
	let confirmPassword = $state('');
	let unlockInput = $state('');
	let mnemonic = $state('');
	let seedMnemonic = $state(''); // stored after seed phrase unlock, for optional password setup
	let unlocking = $state(false);
	let cryptoReady = $state(false);
	let hasPassword = $state(false);

	function serialize(enc: { salt: Uint8Array; iv: Uint8Array; ciphertext: ArrayBuffer }): Uint8Array {
		const salt = Array.from(enc.salt);
		const iv = Array.from(enc.iv);
		const ct = Array.from(new Uint8Array(enc.ciphertext));
		return new Uint8Array([salt.length, ...salt, iv.length, ...iv, ...ct]);
	}

	function deserialize(data: Uint8Array): EncryptedNote {
		const saltLen = data[0];
		const salt = Uint8Array.from(data.slice(1, 1 + saltLen)) as Uint8Array<ArrayBuffer>;
		const ivLen = data[1 + saltLen];
		const iv = Uint8Array.from(data.slice(2 + saltLen, 2 + saltLen + ivLen)) as Uint8Array<ArrayBuffer>;
		const ct = data.slice(2 + saltLen + ivLen);
		return { salt, iv, ciphertext: ct.buffer.slice(ct.byteOffset, ct.byteOffset + ct.byteLength) };
	}

	let passwordValid = $derived(password.length >= 4 && password === confirmPassword);
	let unlockReady = $derived(hasPassword ? unlockInput.length >= 4 : unlockInput.trim().split(/\s+/).length === 12);

	$effect(() => {
		(async () => {
			try {
				await selfCheck();
				cryptoReady = true;
				step = 'checking';

				const exists = await invoke<boolean>('is_vault_initialized');
				if (!exists) {
					step = 'welcome';
					return;
				}

				try {
					await invoke<number[]>('load_vault_key');
					hasPassword = true;
				} catch {
					hasPassword = false;
				}
				step = 'unlock';
			} catch (e: any) {
				errorMsg = `Startup failed: ${e?.message || e}`;
				step = 'error';
			}
		})();
	});

	async function handleUnlock() {
		try {
			unlocking = true;

			if (hasPassword) {
				const raw = await invoke<number[]>('load_vault_key');
				const encrypted = deserialize(new Uint8Array(raw));
				const savedMnemonic = await decryptWithPassword(encrypted, unlockInput);
				if (!validateMnemonic(savedMnemonic)) throw new Error('Invalid vault key — recovery phrase may be needed');
				const key = await deriveMasterKey(savedMnemonic);
				await invoke('unlock_vault', { key: Array.from(key) });
			} else {
				const words = unlockInput.trim().toLowerCase();
				if (!validateMnemonic(words)) {
					errorMsg = 'Invalid seed phrase. Check each word and try again.';
					step = 'unlock';
					unlocking = false;
					return;
				}
				const key = await deriveMasterKey(words);
				await invoke('unlock_vault', { key: Array.from(key) });
				// Offer to set up a password after seed phrase unlock
				seedMnemonic = words;
				password = '';
				confirmPassword = '';
				errorMsg = '';
				step = 'setup-password';
				unlocking = false;
				return;
			}
			onunlock();
		} catch (e: any) {
			errorMsg = e?.message?.includes('Invalid vault key')
				? 'Wrong password or recovery phrase.'
				: `Unlock failed: ${e?.message || e}`;
			step = 'unlock';
		} finally {
			unlocking = false;
		}
	}

	async function handleSetupPassword() {
		if (!passwordValid) return;
		try {
			unlocking = true;
			const encrypted = await encryptWithPassword(seedMnemonic, password);
			await invoke('store_vault_key', { keyData: Array.from(serialize(encrypted)) });
			onunlock();
		} catch (e: any) {
			errorMsg = `Failed to save password: ${e?.message || e}`;
		} finally {
			unlocking = false;
		}
	}

	function skipPasswordSetup() {
		onunlock();
	}

	function switchToSeedPhrase() {
		hasPassword = false;
		unlockInput = '';
		errorMsg = '';
	}

	async function handleCreateVault() {
		if (!passwordValid) return;
		try {
			unlocking = true;
			mnemonic = generateMnemonic();
			const key = await deriveMasterKey(mnemonic);
			await invoke('init_vault', { key: Array.from(key) });

			const encrypted = await encryptWithPassword(mnemonic, password);
			await invoke('store_vault_key', { keyData: Array.from(serialize(encrypted)) });

			step = 'create-seed';
		} catch (e: any) {
			errorMsg = `Failed to create vault: ${e?.message || e}`;
			step = 'error';
		} finally {
			unlocking = false;
		}
	}

	function handleFinishCreate() {
		onunlock();
	}
</script>

{#if step === 'loading' || step === 'checking'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">🔒</div>
			<div class="vault-loader"></div>
			<p class="vault-message">{step === 'loading' ? 'Initializing cryptography…' : 'Checking vault…'}</p>
		</div>
	</div>

{:else if step === 'welcome'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">🔒</div>
			<h1 class="vault-heading">Welcome to Enclave</h1>
			<p class="vault-desc">Your pages are encrypted and live only on this device.</p>
			<div class="vault-form">
				<label class="field-label" for="password">Create vault password</label>
				<input type="password" id="password" class="seed-input" bind:value={password} placeholder="Choose a strong password…" />
				<label class="field-label" for="confirm">Confirm password</label>
				<input type="password" id="confirm" class="seed-input" bind:value={confirmPassword} placeholder="Re-enter password…" />
			</div>
			<div class="vault-actions">
				<Button onclick={handleCreateVault} disabled={!passwordValid}>
					{unlocking ? 'Creating vault…' : 'Create vault'}
				</Button>
				<button class="vault-link-btn" onclick={() => { step = 'unlock'; hasPassword = false; }}>
					I already have a seed phrase
				</button>
			</div>
		</div>
	</div>

{:else if step === 'create-seed'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">✅</div>
			<h1 class="vault-heading">Vault created</h1>
			<p class="vault-desc-warn">
				Save these 12 words somewhere safe. This is your recovery phrase if you forget your password.
			</p>
			<div class="seed-box">
				{#each mnemonic.split(' ') as word, i}
					<span class="seed-word">{i + 1}. {word}</span>
				{/each}
			</div>
			<div class="vault-actions">
				<Button onclick={handleFinishCreate}>I've saved my recovery phrase</Button>
			</div>
		</div>
	</div>

{:else if step === 'unlock'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">🔒</div>
			<h1 class="vault-heading">{hasPassword ? 'Unlock your vault' : 'Enter recovery phrase'}</h1>
			<p class="vault-desc">
				{#if hasPassword}
					Enter your password to unlock.
				{:else}
					Enter your 12-word recovery phrase.
				{/if}
			</p>
			{#if hasPassword}
				<input type="password" class="seed-input" bind:value={unlockInput} placeholder="Enter password…"
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleUnlock(); }} />
			{:else}
				<textarea class="seed-input" bind:value={unlockInput} placeholder="Enter all 12 words, separated by spaces…" rows={3}
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleUnlock(); } }}
				></textarea>
			{/if}
			{#if errorMsg}
				<p class="vault-error">{errorMsg}</p>
			{/if}
			<div class="vault-actions">
				<Button onclick={handleUnlock} disabled={!unlockReady}>
					{unlocking ? 'Unlocking…' : 'Unlock'}
				</Button>
				{#if hasPassword}
					<button class="vault-link-btn" onclick={switchToSeedPhrase}>
						Forgot password? Use recovery phrase
					</button>
				{:else}
					<button class="vault-link-btn" onclick={() => { hasPassword = true; unlockInput = ''; errorMsg = ''; }}>
						Back to password
					</button>
				{/if}
			</div>
		</div>
	</div>

{:else if step === 'setup-password'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">🔑</div>
			<h1 class="vault-heading">Set up a password?</h1>
			<p class="vault-desc">
				You unlocked with your recovery phrase. Set up a password for faster unlocking next time.
			</p>
			<div class="vault-form">
				<label class="field-label" for="setup-pw">Password</label>
				<input type="password" id="setup-pw" class="seed-input" bind:value={password} placeholder="Choose a password…" />
				<label class="field-label" for="setup-confirm">Confirm password</label>
				<input type="password" id="setup-confirm" class="seed-input" bind:value={confirmPassword} placeholder="Re-enter password…" />
			</div>
			{#if errorMsg}
				<p class="vault-error">{errorMsg}</p>
			{/if}
			<div class="vault-actions">
				<Button onclick={handleSetupPassword} disabled={!passwordValid}>
					{unlocking ? 'Saving…' : 'Set password'}
				</Button>
				<button class="vault-link-btn" onclick={skipPasswordSetup}>
					Skip for now
				</button>
			</div>
		</div>
	</div>

{:else if step === 'error'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">⚠️</div>
			<h1 class="vault-heading">Something went wrong</h1>
			<p class="vault-error">{errorMsg}</p>
			<div class="vault-actions">
				<Button onclick={() => { errorMsg = ''; step = 'welcome'; }}>Try again</Button>
			</div>
		</div>
	</div>
{/if}

<style>
	.vault-wall {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		width: 100vw;
		background-color: var(--color-bg);
	}

	.vault-card {
		width: 420px;
		max-width: 90vw;
		padding: 40px 36px;
		border-radius: 16px;
		border: 1px solid var(--color-border);
		background-color: var(--color-surface);
		text-align: center;
	}

	.vault-icon { font-size: 40px; margin-bottom: 12px; }
	.vault-heading { font-size: 22px; font-weight: 700; margin: 0 0 8px; }
	.vault-desc { font-size: 14px; color: var(--color-text-muted); line-height: 1.6; margin: 0 0 24px; }
	.vault-desc-warn { font-size: 13px; color: var(--color-warning); line-height: 1.6; margin: 0 0 16px; }
	.vault-message { font-size: 14px; color: var(--color-text-muted); margin: 16px 0 0; }
	.vault-error { font-size: 13px; color: var(--color-danger); margin: 8px 0 0; }
	.vault-form { text-align: left; margin-bottom: 20px; }
	.field-label { display: block; font-size: 13px; font-weight: 600; color: var(--color-text-muted); margin: 12px 0 4px; }
	.field-label:first-child { margin-top: 0; }

	.vault-actions {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
	}

	.vault-link-btn {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 13px;
		font-family: inherit;
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.vault-link-btn:hover { color: var(--color-text); }

	.seed-box {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		gap: 6px;
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		padding: 14px;
		margin-bottom: 20px;
		text-align: left;
	}

	.seed-word { font-size: 13px; font-family: var(--font-mono); color: var(--color-text); padding: 2px 6px; }

	.seed-input {
		width: 100%;
		box-sizing: border-box;
		padding: 12px 14px;
		border: 1px solid var(--color-border);
		border-radius: 10px;
		background: var(--color-bg);
		color: var(--color-text);
		font-size: 15px;
		font-family: var(--font-mono);
		line-height: 1.6;
		resize: none;
		outline: none;
		transition: border-color 0.15s;
	}
	.seed-input:focus { border-color: var(--color-accent); }
	.seed-input::placeholder { color: var(--color-text-muted); opacity: 0.6; }

	.vault-loader {
		width: 24px;
		height: 24px;
		margin: 12px auto 0;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }
</style>
