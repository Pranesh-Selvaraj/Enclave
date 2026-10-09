#!/usr/bin/env node
// Sign release assets for the in-app updater (issue #83).
//
// The private key is an Ed25519 PKCS#8 PEM read from UPDATE_SIGNING_KEY (CI
// secret; never committed). For each file this writes `<file>.sig` containing
// the hex Ed25519 signature over the file's lowercase SHA-256 hex digest —
// the exact thing `download_update` verifies against the pinned public key in
// `src-tauri/src/updater.rs`.
//
//   node scripts/sign-release.mjs [--sums] <files...>
//
// `--sums` additionally writes SHA256SUMS (manual-install reference) and
// SHA256SUMS.sig (signature over the SHA256SUMS bytes). The updater does not
// read SHA256SUMS; it verifies the per-asset signature plus GitHub's own
// digest before installing.
import { createHash, createPrivateKey, sign } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { basename } from 'node:path';

const args = process.argv.slice(2);
const writeSums = args.includes('--sums');
const files = args.filter((a) => a !== '--sums');

const pem = process.env.UPDATE_SIGNING_KEY;
if (!pem || !pem.trim()) {
	console.error('UPDATE_SIGNING_KEY is not set — refusing to sign nothing.');
	process.exit(1);
}
if (files.length === 0) {
	console.error('usage: node scripts/sign-release.mjs [--sums] <files...>');
	process.exit(1);
}

const key = createPrivateKey(pem);
const sha256 = (data) => createHash('sha256').update(data).digest('hex');
const signDigest = (digest) => sign(null, Buffer.from(digest, 'utf8'), key).toString('hex');

const sums = [];
for (const file of files) {
	const data = readFileSync(file);
	const digest = sha256(data);
	const signature = signDigest(digest);
	writeFileSync(`${file}.sig`, `${signature}\n`);
	sums.push(`${digest}  ${basename(file)}`);
	console.log(`signed ${file}`);
}

if (writeSums) {
	const body = `${sums.join('\n')}\n`;
	writeFileSync('SHA256SUMS', body);
	// Manual verification is over the file bytes, so this one signs the file.
	writeFileSync('SHA256SUMS.sig', `${sign(null, Buffer.from(body, 'utf8'), key).toString('hex')}\n`);
	console.log('wrote SHA256SUMS and SHA256SUMS.sig');
}
