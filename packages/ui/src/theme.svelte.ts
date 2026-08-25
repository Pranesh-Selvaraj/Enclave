// Reactive theme + appearance settings. Persisted to localStorage.

let mode = $state<'auto' | 'dark' | 'light'>('auto');
/** Effective theme — `auto` resolves to the OS preference. */
let current = $state<'dark' | 'light'>('dark');
let systemDark = $state(false);

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
export const FONT_SIZES = ['s', 'm', 'l', 'xl'] as const;
export const PAGE_WIDTHS = ['compact', 'wide', 'full'] as const;
export const HOME_SORTS = ['recent', 'created', 'title'] as const;
export const LOCK_AFTERS = [0, 1, 5, 15, 60] as const;

let accent = $state<string>(ACCENTS[0].id);
let font = $state<string>(FONTS[0]);
let density = $state<string>(DENSITIES[1]);
let fontSize = $state<string>(FONT_SIZES[1]);
let pageWidth = $state<string>(PAGE_WIDTHS[1]);
let trueBlack = $state(false);
let reduceMotion = $state(false);
let haptics = $state(true);
let homeSort = $state<string>(HOME_SORTS[0]);
let lockAfter = $state(0); // minutes; 0 = never auto-lock

function apply() {
	if (typeof document === 'undefined') return;
	current = mode === 'auto' ? (systemDark ? 'dark' : 'light') : mode;
	const root = document.documentElement;
	root.setAttribute('data-theme', current);
	root.setAttribute('data-accent', accent);
	root.setAttribute('data-font', font);
	root.setAttribute('data-density', density);
	root.setAttribute('data-font-size', fontSize);
	root.setAttribute('data-page-width', pageWidth);
	root.toggleAttribute('data-true-black', trueBlack);
	root.toggleAttribute('data-reduce-motion', reduceMotion);
	try {
		localStorage.setItem(KEY, mode);
		localStorage.setItem(
			SKEY,
			JSON.stringify({ accent, font, density, fontSize, pageWidth, trueBlack, reduceMotion, haptics, homeSort, lockAfter }),
		);
	} catch { /* private browsing */ }
}

export const theme = {
	/** Effective theme ('dark' | 'light') — auto is already resolved. */
	get value() { return current; },
	set value(v: 'dark' | 'light') { mode = v; apply(); },
	get mode() { return mode; },
	set mode(v: 'auto' | 'dark' | 'light') { mode = v; apply(); },
	toggle() {
		mode = mode === 'auto' ? 'light' : mode === 'light' ? 'dark' : 'auto';
		apply();
	},
	get accent() { return accent; },
	set accent(v: string) { accent = v; apply(); },
	get font() { return font; },
	set font(v: string) { font = v; apply(); },
	get density() { return density; },
	set density(v: string) { density = v; apply(); },
	get fontSize() { return fontSize; },
	set fontSize(v: string) { fontSize = v; apply(); },
	get pageWidth() { return pageWidth; },
	set pageWidth(v: string) { pageWidth = v; apply(); },
	get trueBlack() { return trueBlack; },
	set trueBlack(v: boolean) { trueBlack = v; apply(); },
	get reduceMotion() { return reduceMotion; },
	set reduceMotion(v: boolean) { reduceMotion = v; apply(); },
	get haptics() { return haptics; },
	set haptics(v: boolean) { haptics = v; apply(); },
	get homeSort() { return homeSort; },
	set homeSort(v: string) { homeSort = v; apply(); },
	get lockAfter() { return lockAfter; },
	set lockAfter(v: number) { lockAfter = v; apply(); },
	init() {
		try {
			const saved = localStorage.getItem(KEY);
			if (saved === 'light' || saved === 'dark' || saved === 'auto') mode = saved;
			const s = JSON.parse(localStorage.getItem(SKEY) || '{}');
			if (ACCENTS.some((a) => a.id === s.accent)) accent = s.accent;
			if (FONTS.includes(s.font)) font = s.font;
			if (DENSITIES.includes(s.density)) density = s.density;
			if (FONT_SIZES.includes(s.fontSize)) fontSize = s.fontSize;
			if (PAGE_WIDTHS.includes(s.pageWidth)) pageWidth = s.pageWidth;
			if (HOME_SORTS.includes(s.homeSort)) homeSort = s.homeSort;
			if (typeof s.trueBlack === 'boolean') trueBlack = s.trueBlack;
			if (typeof s.reduceMotion === 'boolean') reduceMotion = s.reduceMotion;
			if (typeof s.haptics === 'boolean') haptics = s.haptics;
			if (LOCK_AFTERS.includes(s.lockAfter)) lockAfter = s.lockAfter;
		} catch { /* ignore */ }
		if (typeof window !== 'undefined' && 'matchMedia' in window) {
			const mq = window.matchMedia('(prefers-color-scheme: dark)');
			systemDark = mq.matches;
			mq.addEventListener('change', (e) => { systemDark = e.matches; apply(); });
		}
		apply();
	},
};
