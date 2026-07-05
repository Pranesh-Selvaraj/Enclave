// Reactive theme store. Persisted to localStorage under "enclave-theme".

let current = $state<'dark' | 'light'>('dark');

const KEY = 'enclave-theme';

function apply() {
	document.documentElement.setAttribute('data-theme', current);
	try { localStorage.setItem(KEY, current); } catch { /* private browsing */ }
}

export const theme = {
	get value() { return current; },
	set value(v: 'dark' | 'light') {
		current = v;
		apply();
	},
	toggle() {
		current = current === 'dark' ? 'light' : 'dark';
		apply();
	},
	/** Call once at app startup to restore saved preference. */
	init() {
		try {
			const saved = localStorage.getItem(KEY);
			if (saved === 'light' || saved === 'dark') current = saved;
		} catch { /* ignore */ }
		apply();
	},
};
