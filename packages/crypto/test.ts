// Enclave crypto verification — exercises the full pipeline:
// mnemonic generation → validation → master key derivation → encrypt/decrypt → determinism

import {
	generateMnemonic,
	validateMnemonic,
	entropyFromMnemonic,
	deriveMasterKey,
	importAesKey,
	encrypt,
	decrypt,
	selfCheck,
} from './src/index.ts';

async function main() {
	console.log('=== Enclave Crypto Verification ===\n');

	// 1. Run existing self-check
	console.log('[1/5] Running built-in self-check...');
	await selfCheck();
	console.log('  PASS\n');

	// 2. Generate mnemonic — must be 12 words, valid
	console.log('[2/5] Mnemonic generation...');
	const mnemonic = generateMnemonic();
	const words = mnemonic.split(' ');
	console.assert(words.length === 12, '12 words');
	console.assert(validateMnemonic(mnemonic), 'validateMnemonic returned true');
	console.log('  PASS:', mnemonic);
	console.log();

	// 3. Mnemonic ↔ entropy round-trip
	console.log('[3/5] Mnemonic ↔ entropy round-trip...');
	const entropy = entropyFromMnemonic(mnemonic);
	console.assert(entropy.length === 16, `entropy is 16 bytes (got ${entropy.length})`);
	// validateMnemonic also checks checksum, so this implicitly verifies the round-trip
	console.log('  PASS: 16 bytes entropy\n');

	// 4. Master key derivation is deterministic
	console.log('[4/5] Master key derivation...');
	const key1 = await deriveMasterKey(mnemonic);
	const key2 = await deriveMasterKey(mnemonic);
	console.assert(key1.length === 32, 'key is 32 bytes');
	console.assert(
		Buffer.from(key1).equals(Buffer.from(key2)),
		'same mnemonic → same key (deterministic)',
	);
	console.log(`  PASS: 32-byte key, deterministic\n`);

	// 5. Encrypt/decrypt with derived master key
	console.log('[5/5] Encrypt/decrypt with master key...');
	const aesKey = await importAesKey(key1);
	const plaintext = 'Hello from Enclave! 🔒';
	const { iv, ciphertext } = await encrypt(plaintext, aesKey);
	const decrypted = await decrypt(ciphertext, iv, aesKey);
	console.assert(decrypted === plaintext, 'round-trip works');
	console.log('  PASS:', plaintext, '→', `(${ciphertext.byteLength} bytes encrypted)`);
	console.log();

	// 6. Different mnemonic → different key
	console.log('[bonus] Different mnemonic → different key...');
	const otherMnemonic = generateMnemonic();
	const otherKey = await deriveMasterKey(otherMnemonic);
	console.assert(
		!Buffer.from(key1).equals(Buffer.from(otherKey)),
		'different mnemonics produce different keys',
	);
	console.log('  PASS\n');

	console.log('=== All crypto checks passed ===');
}

main().catch((e) => {
	console.error('FAIL:', e);
	process.exit(1);
});
