<script lang="ts">
	import { getStorage } from '$lib/storage';
	import { generateMnemonic, validateMnemonic, deriveMasterKey, selfCheck } from '@enclave/crypto';
	import { Button } from '@enclave/ui';

	let { onunlock }: { onunlock: () => void } = $props();

	type Step = 'loading' | 'checking' | 'welcome' | 'create-seed' | 'confirm-seed' | 'unlock' | 'error';
	let step = $state<Step>('loading');
	let errorMsg = $state('');

	let mnemonic = $state('');
	let confirmInput = $state('');
	let unlockInput = $state('');
	let unlocking = $state(false);
	let cryptoReady = $state(false);

	let confirmMatch = $derived(confirmInput.trim().toLowerCase() === mnemonic.trim());
	let unlockReady = $derived(unlockInput.trim().split(/\s+/).length === 12);

	// Boot sequence: verify crypto, then check vault state
	$effect(() => {
		(async () => {
			try {
				await selfCheck();
				cryptoReady = true;
				step = 'checking';

				const s = await getStorage();
				const exists = await s.isVaultInitialized();
				step = exists ? 'unlock' : 'welcome';
			} catch (e: any) {
				errorMsg = `Startup failed: ${e?.message || e}`;
				step = 'error';
			}
		})();
	});

	async function handleCreateSeed() {
		try {
			mnemonic = generateMnemonic();
			step = 'create-seed';
		} catch (e: any) {
			errorMsg = `Seed generation failed: ${e?.message || e}`;
			step = 'error';
		}
	}

	function handleConfirmSeed() {
		confirmInput = '';
		step = 'confirm-seed';
	}

	async function handleFinalizeVault() {
		if (!confirmMatch) return;
		try {
			unlocking = true;
			const key = await deriveMasterKey(mnemonic);
			const s = await getStorage();
			await s.initVault(key);
			onunlock();
		} catch (e: any) {
			errorMsg = `Failed to create vault: ${e?.message || e}`;
			step = 'error';
		} finally {
			unlocking = false;
		}
	}

	async function handleUnlock() {
		const words = unlockInput.trim().toLowerCase();
		if (!validateMnemonic(words)) {
			errorMsg = 'Invalid seed phrase. Check each word and try again.';
			step = 'unlock';
			return;
		}
		try {
			unlocking = true;
			const key = await deriveMasterKey(words);
			const s = await getStorage();
			await s.unlockVault(key);
			onunlock();
		} catch (e: any) {
			errorMsg = e?.message?.includes('Invalid vault key')
				? 'Wrong seed phrase. This is not the phrase used to create this vault.'
				: `Unlock failed: ${e?.message || e}`;
			step = 'unlock';
		} finally {
			unlocking = false;
		}
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
			<p class="vault-desc">
				Your notes are encrypted with a 12-word seed phrase.
				Only you hold the keys — there are no accounts, no cloud, no recovery.
			</p>
			<div class="vault-actions">
				<Button onclick={handleCreateSeed}>Create new vault</Button>
				<button class="vault-link-btn" onclick={() => (step = 'unlock')}>
					I already have a seed phrase
				</button>
			</div>
		</div>
	</div>

{:else if step === 'create-seed'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">🔒</div>
			<h1 class="vault-heading">Your recovery phrase</h1>
			<p class="vault-desc-warn">
				Write these 12 words down on paper and store them safely.
				You will need them to unlock your vault. They cannot be recovered.
			</p>
			<div class="seed-box">
				{#each mnemonic.split(' ') as word, i}
					<span class="seed-word">{i + 1}. {word}</span>
				{/each}
			</div>
			<div class="vault-actions">
				<Button onclick={handleConfirmSeed}>I've saved my phrase</Button>
				<button class="vault-link-btn" onclick={() => (step = 'welcome')}>Back</button>
			</div>
		</div>
	</div>

{:else if step === 'confirm-seed'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">🔒</div>
			<h1 class="vault-heading">Confirm your phrase</h1>
			<p class="vault-desc">
				Re-enter your 12-word seed phrase to confirm you've saved it correctly.
			</p>
			<textarea
				class="seed-input"
				bind:value={confirmInput}
				placeholder="Enter all 12 words, separated by spaces…"
				rows={3}
			></textarea>
			{#if !confirmMatch && confirmInput.trim()}
				<p class="vault-error">Phrases don't match</p>
			{/if}
			<div class="vault-actions">
				<Button
					onclick={handleFinalizeVault}
					disabled={!confirmMatch}
				>{unlocking ? 'Creating vault…' : 'Create vault'}</Button
				>
				<button class="vault-link-btn" onclick={() => (step = 'create-seed')}>Back</button>
			</div>
		</div>
	</div>

{:else if step === 'unlock'}
	<div class="vault-wall">
		<div class="vault-card">
			<div class="vault-icon">🔒</div>
			<h1 class="vault-heading">Unlock your vault</h1>
			<p class="vault-desc">Enter your 12-word seed phrase to decrypt your notes.</p>
			<textarea
				class="seed-input"
				bind:value={unlockInput}
				placeholder="Enter all 12 words, separated by spaces…"
				rows={3}
				onkeydown={(e: KeyboardEvent) => {
					if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleUnlock(); }
				}}
			></textarea>
			{#if errorMsg && step === 'unlock'}
				<p class="vault-error">{errorMsg}</p>
			{/if}
			<div class="vault-actions">
				<Button
					onclick={handleUnlock}
					disabled={!unlockReady}
				>{unlocking ? 'Unlocking…' : 'Unlock'}</Button
				>
				<button class="vault-link-btn" onclick={() => { step = 'welcome'; errorMsg = ''; }}>
					Create a new vault instead
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

	.vault-icon {
		font-size: 40px;
		margin-bottom: 12px;
	}

	.vault-heading {
		font-size: 22px;
		font-weight: 700;
		margin: 0 0 8px;
	}

	.vault-desc {
		font-size: 14px;
		color: var(--color-text-muted);
		line-height: 1.6;
		margin: 0 0 24px;
	}

	.vault-desc-warn {
		font-size: 13px;
		color: var(--color-warning);
		line-height: 1.6;
		margin: 0 0 16px;
	}

	.vault-message {
		font-size: 14px;
		color: var(--color-text-muted);
		margin: 16px 0 0;
	}

	.vault-error {
		font-size: 13px;
		color: var(--color-danger);
		margin: 8px 0 0;
	}

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

	.vault-link-btn:hover {
		color: var(--color-text);
	}

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

	.seed-word {
		font-size: 13px;
		font-family: var(--font-mono);
		color: var(--color-text);
		padding: 2px 6px;
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
		transition: border-color 0.15s;
	}

	.seed-input:focus {
		border-color: var(--color-accent);
	}

	.seed-input::placeholder {
		color: var(--color-text-muted);
		opacity: 0.6;
	}

	/* simple spinning loader */
	.vault-loader {
		width: 24px;
		height: 24px;
		margin: 12px auto 0;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
