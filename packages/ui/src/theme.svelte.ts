// Reactive theme + appearance settings. Persisted to localStorage.

let current = $state<'dark' | 'light'>('dark');

const KEY = 'enclave-theme';
const SKEY = 'enclave-settings';

export const ACCENTS = [
	{ id: 'violet', color: '#7c6cf0' },
	{ id: 'blue', color: '#4f8ef7' },
	{ id: 'green', color: '#2fbf71' },
	{ id: 'teal', color: '#2bb6b6' },
	{ id: 'orange', color: '#e8873a' },
	{ id: 'red', color: '#e5534b' },
] as const;

export const FONTS = ['inter', 'system', 'serif', 'mono'] as const;
export const DENSITIES = ['narrow', 'normal', 'wide'] as const;

let accent = $state<string>(ACCENTS[0].id);
let font = $state<string>(FONTS[0]);
let density = $state<string>(DENSITIES[1]);

function apply() {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;
	root.setAttribute('data-theme', current);
	root.setAttribute('data-accent', accent);
	root.setAttribute('data-font', font);
	root.setAttribute('data-density', density);
	try {
		localStorage.setItem(KEY, current);
		localStorage.setItem(SKEY, JSON.stringify({ accent, font, density }));
	} catch { /* private browsing */ }
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
	get accent() { return accent; },
	set accent(v: string) {
		accent = v;
		apply();
	},
	get font() { return font; },
	set font(v: string) {
		font = v;
		apply();
	},
	get density() { return density; },
	set density(v: string) {
		density = v;
		apply();
	},
	init() {
		try {
			const saved = localStorage.getItem(KEY);
			if (saved === 'light' || saved === 'dark') current = saved;
			const s = JSON.parse(localStorage.getItem(SKEY) || '{}');
			if (ACCENTS.some((a) => a.id === s.accent)) accent = s.accent;
			if (FONTS.includes(s.font)) font = s.font;
			if (DENSITIES.includes(s.density)) density = s.density;
		} catch { /* ignore */ }
		apply();
	},
};
