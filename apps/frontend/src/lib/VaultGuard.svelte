<script lang="ts">
	import { invoke } from '$lib/backend.js';
	import { generateMnemonic, validateMnemonic, deriveMasterKey, selfCheck, encryptWithPassword, decryptWithPassword, type EncryptedNote } from '@enclave/crypto';
	import { Button, Logo, Icon } from '@enclave/ui';

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
		if (data.length < 3) throw new Error('Corrupt vault key file');
		const saltLen = data[0];
		const ivLen = data[1 + saltLen];
		if (2 + saltLen + ivLen > data.length) throw new Error('Corrupt vault key file');
		const salt = Uint8Array.from(data.slice(1, 1 + saltLen)) as Uint8Array<ArrayBuffer>;
		const iv = Uint8Array.from(data.slice(2 + saltLen, 2 + saltLen + ivLen)) as Uint8Array<ArrayBuffer>;
		const ct = data.slice(2 + saltLen + ivLen);
		return { salt, iv, ciphertext: ct.buffer.slice(ct.byteOffset, ct.byteOffset + ct.byteLength) };
	}

	let passwordValid = $derived(password.length >= 4 && password === confirmPassword);
	// BIP39 supports 12-24 words; don't lock out users with longer phrases
	let unlockReady = $derived(hasPassword ? unlockInput.length >= 4 : unlockInput.trim().split(/\s+/).length >= 12);

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
			// A half-created vault (init ok, key store failed) would leave the
			// user permanently locked out — no password and no seed shown.
			try { await invoke('reset_vault'); } catch { /* already clean */ }
			errorMsg = `Failed to create vault: ${e?.message || e}`;
			step = 'error';
		} finally {
			unlocking = false;
		}
	}

	function handleFinishCreate() {
		onunlock();
	}

	let phraseCopied = $state(false);
	async function copyPhrase() {
		try {
			await navigator.clipboard.writeText(mnemonic);
			phraseCopied = true;
			setTimeout(() => (phraseCopied = false), 1500);
		} catch { /* clipboard unavailable */ }
	}
</script>

<div class="vault-wall">
	<div class="vault-glow vault-glow-a"></div>
	<div class="vault-glow vault-glow-b"></div>

	{#if step === 'loading' || step === 'checking'}
		<div class="vault-card vault-card-center">
			<Logo size={56} />
			<div class="vault-loader"></div>
			<p class="vault-message">{step === 'loading' ? 'Initializing cryptography…' : 'Checking vault…'}</p>
		</div>

	{:else if step === 'welcome'}
		<div class="vault-card">
			<Logo size={56} />
			<h1 class="vault-heading">Welcome to Enclave</h1>
			<p class="vault-desc">Your pages are encrypted and live only on this device.</p>
			<div class="vault-features">
				<div class="vf-item"><Icon name="lock" size={15} /><span>Encrypted vault on your device</span></div>
				<div class="vf-item"><Icon name="network" size={15} /><span>Peer-to-peer sync over Wi-Fi</span></div>
				<div class="vf-item"><Icon name="zap" size={15} /><span>Offline-first — no cloud, ever</span></div>
			</div>
			<div class="vault-form">
				<label class="field-label" for="password">Create vault password</label>
				<!-- svelte-ignore a11y_autofocus -->
				<input type="password" id="password" class="seed-input" bind:value={password} placeholder="Choose a strong password…" autofocus
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' && passwordValid) handleCreateVault(); }} />
				<label class="field-label" for="confirm">Confirm password</label>
				<input type="password" id="confirm" class="seed-input" bind:value={confirmPassword} placeholder="Re-enter password…"
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' && passwordValid) handleCreateVault(); }} />
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

	{:else if step === 'create-seed'}
		<div class="vault-card">
			<div class="brand-mark brand-mark-ok">✓</div>			<h1 class="vault-heading">Vault created</h1>
			<p class="vault-desc-warn">
				Save these 12 words somewhere safe. This is your recovery phrase if you forget your password.
			</p>
			<div class="seed-box">
				{#each mnemonic.split(' ') as word, i}
					<div class="seed-word">
						<span class="seed-num">{i + 1}</span>
						<span>{word}</span>
					</div>
				{/each}
			</div>
			<button class="vault-link-btn" onclick={copyPhrase}>
				{phraseCopied ? '✓ Copied to clipboard' : 'Copy phrase to clipboard'}
			</button>
			<div class="vault-actions">
				<Button onclick={handleFinishCreate}>I've saved my recovery phrase</Button>
			</div>
		</div>

	{:else if step === 'unlock'}
		<div class="vault-card">
			<Logo size={56} />
			<h1 class="vault-heading">{hasPassword ? 'Unlock your vault' : 'Enter recovery phrase'}</h1>
			<p class="vault-desc">
				{#if hasPassword}
					Enter your password to unlock.
				{:else}
					Enter your 12-word recovery phrase.
				{/if}
			</p>
			{#if hasPassword}
				<!-- svelte-ignore a11y_autofocus -->
				<input type="password" class="seed-input" bind:value={unlockInput} placeholder="Enter password…" autofocus
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleUnlock(); }} />
			{:else}
				<textarea class="seed-input" bind:value={unlockInput} placeholder="Enter all 12 words, separated by spaces…" rows={3}
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleUnlock(); } }}
				></textarea>
			{/if}
			{#if errorMsg}
				<p class="vault-error" role="alert">{errorMsg}</p>
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

	{:else if step === 'setup-password'}
		<div class="vault-card">
			<Logo size={56} />
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
				<p class="vault-error" role="alert">{errorMsg}</p>
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

	{:else if step === 'error'}
		<div class="vault-card">
			<div class="brand-mark brand-mark-err">!</div>
			<h1 class="vault-heading">Something went wrong</h1>
			<p class="vault-error" role="alert">{errorMsg}</p>
			<div class="vault-actions">
				<Button onclick={() => { errorMsg = ''; step = 'welcome'; }}>Try again</Button>
			</div>
		</div>
	{/if}
</div>

<style>
	.vault-wall {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		width: 100vw;
		background-color: var(--color-bg);
		overflow: hidden;
	}

	.vault-glow {
		position: absolute;
		border-radius: 50%;
		filter: blur(90px);
		pointer-events: none;
	}
	.vault-glow-a {
		width: 480px;
		height: 480px;
		top: -160px;
		left: -120px;
		background: rgba(124, 111, 240, 0.16);
	}
	.vault-glow-b {
		width: 420px;
		height: 420px;
		bottom: -140px;
		right: -100px;
		background: rgba(74, 144, 226, 0.12);
	}

	.vault-card {
		position: relative;
		width: 420px;
		max-width: 90vw;
		padding: 44px 40px;
		border-radius: 20px;
		border: 1px solid var(--color-border);
		background-color: var(--color-surface);
		text-align: center;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.35);
	}
	.vault-card-center {
		display: flex;
		flex-direction: column;
		align-items: center;
	}
	/* The brand logo is its own tile — center it like the old text mark. */
	.vault-card > svg {
		display: block;
		margin: 0 auto 18px;
	}

	.brand-mark {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 56px;
		height: 56px;
		margin: 0 auto 18px;
		border-radius: 16px;
		background: linear-gradient(135deg, #7c6cf0, #4f46e5);
		color: #fff;
		font-size: 26px;
		font-weight: 800;
		letter-spacing: -0.02em;
		box-shadow: 0 8px 24px rgba(124, 111, 240, 0.35);
	}
	.brand-mark-ok {
		background: linear-gradient(135deg, #46a758, #2f9e44);
		box-shadow: 0 8px 24px rgba(70, 167, 88, 0.3);
	}
	.brand-mark-err {
		background: linear-gradient(135deg, #e5484d, #b13b40);
		box-shadow: 0 8px 24px rgba(229, 72, 77, 0.3);
	}

	.vault-heading { font-size: 22px; font-weight: 700; margin: 0 0 8px; letter-spacing: -0.01em; }
	.vault-desc { font-size: 14px; color: var(--color-text-muted); line-height: 1.6; margin: 0 0 26px; }
	.vault-desc-warn { font-size: 13px; color: var(--color-warning); line-height: 1.6; margin: 0 0 18px; }
	.vault-message { font-size: 14px; color: var(--color-text-muted); margin: 16px 0 0; }
	.vault-error {
		font-size: 13px;
		color: var(--color-danger);
		background: color-mix(in srgb, var(--color-danger) 8%, transparent);
		border: 1px solid color-mix(in srgb, var(--color-danger) 25%, transparent);
		border-radius: 8px;
		padding: 8px 12px;
		margin: 12px 0 0;
	}
	.vault-features {
		display: flex;
		flex-direction: column;
		gap: 8px;
		text-align: left;
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		padding: 12px 14px;
		margin-bottom: 20px;
	}
	.vf-item {
		display: flex;
		align-items: center;
		gap: 10px;
		font-size: 13px;
		color: var(--color-text);
	}
	.vf-item :global(svg) {
		color: var(--color-accent);
		flex-shrink: 0;
	}

	.vault-form { text-align: left; margin-bottom: 22px; }
	.field-label { display: block; font-size: 13px; font-weight: 600; color: var(--color-text-muted); margin: 14px 0 5px; }
	.field-label:first-child { margin-top: 0; }

	.vault-actions {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		margin-top: 22px;
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
		border-radius: 12px;
		padding: 14px;
		margin-bottom: 4px;
		text-align: left;
	}

	.seed-word {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		font-family: var(--font-mono);
		color: var(--color-text);
		padding: 3px 6px;
		border-radius: 6px;
	}
	.seed-word:hover { background: var(--color-hover); }
	.seed-num {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 18px;
		height: 18px;
		padding: 0 4px;
		border-radius: 5px;
		background: rgba(124, 111, 240, 0.15);
		color: var(--color-accent);
		font-size: 10px;
		font-weight: 700;
	}

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
		transition: border-color 0.15s, box-shadow 0.15s;
	}
	.seed-input:focus {
		border-color: var(--color-accent);
		box-shadow: 0 0 0 3px rgba(124, 111, 240, 0.15);
	}
	.seed-input::placeholder { color: var(--color-text-muted); opacity: 0.6; }

	.vault-loader {
		width: 24px;
		height: 24px;
		margin: 20px auto 0;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }
</style>
