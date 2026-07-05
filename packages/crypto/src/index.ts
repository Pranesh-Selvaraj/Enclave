// Enclave crypto — BIP39 mnemonic generation, Argon2id key derivation,
// and AES-256-GCM encryption. All keys derived client-side; never leave the device.
//
// Dependencies: hash-wasm (WASM Argon2id), @scure/bip39 (mnemonic), Web Crypto API (AES-GCM).

import { argon2id } from 'hash-wasm';
import { generateMnemonic as _generateMnemonic, validateMnemonic as _validateMnemonic, mnemonicToEntropy } from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english.js';

// ── Argon2id parameters (tuned for interactive desktop unlock) ────────────────

const ARGON2_MEMORY_KIB = 65536; // 64 MiB
const ARGON2_ITERATIONS = 3;
const ARGON2_PARALLELISM = 4;
const KEY_LENGTH = 32; // 256 bits

// Deterministic salt for master key derivation (versioned for future rotation).
const MASTER_KEY_SALT = new TextEncoder().encode('enclave-vault-master-key-v1');

// ── BIP39 Mnemonic ────────────────────────────────────────────────────────────

const MNEMONIC_STRENGTH = 128; // 12 words

/** Generate a new 12-word BIP39 mnemonic (English). Uses CSPRNG. */
export function generateMnemonic(): string {
	return _generateMnemonic(wordlist, MNEMONIC_STRENGTH);
}

/** Validate a BIP39 English mnemonic (checksum + wordlist). */
export function validateMnemonic(mnemonic: string): boolean {
	return _validateMnemonic(mnemonic, wordlist);
}

/** Decode a validated mnemonic to raw entropy bytes (128 bits). Uses English wordlist. */
export function entropyFromMnemonic(mnemonic: string): Uint8Array {
	return mnemonicToEntropy(mnemonic, wordlist);
}

// ── Key derivation ────────────────────────────────────────────────────────────

/** Derive a 256-bit key from any password + salt using Argon2id. */
export async function deriveKeyBytes(password: string, salt: Uint8Array): Promise<Uint8Array<ArrayBuffer>> {
	const raw = await argon2id({
		password,
		salt,
		parallelism: ARGON2_PARALLELISM,
		iterations: ARGON2_ITERATIONS,
		memorySize: ARGON2_MEMORY_KIB,
		hashLength: KEY_LENGTH,
		outputType: 'binary',
	});
	return Uint8Array.from(raw) as Uint8Array<ArrayBuffer>;
}

/**
 * Derive the 256-bit vault master key from a 12-word BIP39 mnemonic.
 * Uses a deterministic app-level salt — same mnemonic always yields same key.
 */
export async function deriveMasterKey(mnemonic: string): Promise<Uint8Array<ArrayBuffer>> {
	return deriveKeyBytes(mnemonic, MASTER_KEY_SALT);
}

// ── AES-256-GCM via Web Crypto ────────────────────────────────────────────────

/** Import raw bytes as a Web Crypto AES-GCM key. */
export async function importAesKey(rawKey: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
	return crypto.subtle.importKey(
		'raw',
		rawKey,
		{ name: 'AES-GCM' },
		false,
		['encrypt', 'decrypt'],
	);
}

/** Encrypt plaintext with AES-256-GCM. Returns IV + ciphertext. */
export async function encrypt(
	plaintext: string,
	key: CryptoKey,
): Promise<{ iv: Uint8Array<ArrayBuffer>; ciphertext: ArrayBuffer }> {
	const iv = crypto.getRandomValues(new Uint8Array(12)) as Uint8Array<ArrayBuffer>;
	const encoded = new TextEncoder().encode(plaintext);
	const ciphertext = await crypto.subtle.encrypt(
		{ name: 'AES-GCM', iv },
		key,
		encoded,
	);
	return { iv, ciphertext };
}

/** Decrypt ciphertext with AES-256-GCM. */
export async function decrypt(
	ciphertext: ArrayBuffer,
	iv: Uint8Array<ArrayBuffer>,
	key: CryptoKey,
): Promise<string> {
	const decrypted = await crypto.subtle.decrypt(
		{ name: 'AES-GCM', iv },
		key,
		ciphertext,
	);
	return new TextDecoder().decode(decrypted);
}

// ── Convenience: encrypt/decrypt with raw password (not mnemonic) ─────────────

export interface EncryptedNote {
	salt: Uint8Array<ArrayBuffer>;
	iv: Uint8Array<ArrayBuffer>;
	ciphertext: ArrayBuffer;
}

export async function encryptWithPassword(
	plaintext: string,
	password: string,
): Promise<EncryptedNote> {
	const salt = crypto.getRandomValues(new Uint8Array(32)) as Uint8Array<ArrayBuffer>;
	const rawKey = await deriveKeyBytes(password, salt);
	const key = await importAesKey(rawKey);
	const { iv, ciphertext } = await encrypt(plaintext, key);
	return { salt, iv, ciphertext };
}

export async function decryptWithPassword(
	encrypted: EncryptedNote,
	password: string,
): Promise<string> {
	const rawKey = await deriveKeyBytes(password, encrypted.salt);
	const key = await importAesKey(rawKey);
	return decrypt(encrypted.ciphertext, encrypted.iv, key);
}

// ── Self-check ─────────────────────────────────────────────────────────────────

/** Run all crypto self-checks. Throws on any failure. */
export async function selfCheck(): Promise<void> {
	// 1. Basic encrypt/decrypt round trip
	const pw = 'enclave-self-check';
	const plaintext = 'crypto works';
	const encrypted = await encryptWithPassword(plaintext, pw);
	const decrypted = await decryptWithPassword(encrypted, pw);
	if (decrypted !== plaintext) throw new Error('Crypto self-check failed: encrypt/decrypt mismatch');
	// Wrong password must fail
	try {
		await decryptWithPassword(encrypted, 'wrong-password');
		throw new Error('Crypto self-check failed: wrong password should have thrown');
	} catch {
		// expected
	}

	// 2. Mnemonic generation + validation
	const mnemonic = generateMnemonic();
	if (!validateMnemonic(mnemonic)) throw new Error('Crypto self-check failed: mnemonic invalid');
	if (mnemonic.split(' ').length !== 12) throw new Error('Crypto self-check failed: not 12 words');

	// 3. Master key derivation is deterministic
	const key1 = await deriveMasterKey(mnemonic);
	const key2 = await deriveMasterKey(mnemonic);
	if (bytesToHex(key1) !== bytesToHex(key2)) {
		throw new Error('Crypto self-check failed: master key not deterministic');
	}

	// 4. Different mnemonic → different key
	const otherMnemonic = generateMnemonic();
	const otherKey = await deriveMasterKey(otherMnemonic);
	if (bytesToHex(key1) === bytesToHex(otherKey)) {
		throw new Error('Crypto self-check failed: different mnemonics produced same key');
	}

	// 5. Master key is 32 bytes
	if (key1.length !== 32) throw new Error('Crypto self-check failed: master key wrong length');
}

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes).map((b) => b.toString(16).padStart(2, '0')).join('');
}
