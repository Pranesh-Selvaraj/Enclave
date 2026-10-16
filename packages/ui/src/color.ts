// Accessible accent-color helpers (issues #142/#143/#144). Pure, DOM-free and
// unit-tested: the theme store derives a per-theme accent from these so preset
// *and* custom colors keep accent text and on-accent ink readable.

export type ThemeName = 'dark' | 'light';

/** Surface behind accent text (must match --color-surface in app.css). */
const SURFACE: Record<ThemeName, string> = {
	dark: '#201c17',
	light: '#faf7f1',
};

/** accent-subtle tint strength per theme (must match --accent-tint in app.css). */
export const ACCENT_TINT: Record<ThemeName, number> = { dark: 0.14, light: 0.12 };

/** Ink used on bright fills (preferred over pure black, matches the palette). */
export const DARK_INK = '#17130e';
const WHITE = '#ffffff';
/** WCAG AA for normal text; also used for the non-text accent tint. */
const TARGET = 4.5;

export function hexToRgb(hex: string): [number, number, number] {
	const h = hex.replace('#', '');
	return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

function rgbToHex(rgb: number[]): string {
	return `#${rgb.map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('')}`;
}

function channel(v: number): number {
	const c = v / 255;
	return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

export function relativeLuminance(hex: string): number {
	const [r, g, b] = hexToRgb(hex);
	return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** WCAG contrast ratio between two `#rrggbb` colors. */
export function contrastRatio(a: string, b: string): number {
	const la = relativeLuminance(a);
	const lb = relativeLuminance(b);
	const hi = Math.max(la, lb);
	const lo = Math.min(la, lb);
	return (hi + 0.05) / (lo + 0.05);
}

/** Mix two colors; `t` is the weight of `a` (0..1). */
export function mixHex(a: string, b: string, t: number): string {
	const A = hexToRgb(a);
	const B = hexToRgb(b);
	return rgbToHex([A[0] * t + B[0] * (1 - t), A[1] * t + B[1] * (1 - t), A[2] * t + B[2] * (1 - t)]);
}

/** The ink (white or dark) with the better contrast on `accent`. */
export function onAccent(accent: string): string {
	return contrastRatio(WHITE, accent) >= contrastRatio(DARK_INK, accent) ? WHITE : DARK_INK;
}

/**
 * Nudge `accent` (toward white on dark, toward black on light) until it clears
 * 4.5:1 both on the surface and on its own accent-subtle tint. Returns the
 * adjusted color; the input is returned unchanged when it already passes.
 */
export function tuneAccent(accent: string, theme: ThemeName): string {
	let c = accent;
	for (let i = 0; i < 100; i++) {
		const subtle = mixHex(c, SURFACE[theme], ACCENT_TINT[theme]);
		if (contrastRatio(c, SURFACE[theme]) >= TARGET && contrastRatio(c, subtle) >= TARGET) return c;
		c = mixHex(theme === 'dark' ? WHITE : '#000000', c, 0.02);
	}
	return c;
}
