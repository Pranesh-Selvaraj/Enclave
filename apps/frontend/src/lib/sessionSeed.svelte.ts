// Session-only seed phrase holder.
//
// The vault's password path already decrypts `vault.key` to derive the master
// key, so the mnemonic is in memory at unlock time anyway. Account pairing
// (desktop → phone) reuses it to hand the account to a phone; keeping it here
// means the core never has to store the seed. Cleared on lock.

let seed = $state('');

export const sessionSeed = {
	get value() {
		return seed;
	},
	set(v: string) {
		seed = v;
	},
	clear() {
		seed = '';
	},
};
