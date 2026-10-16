import * as assert from 'node:assert';
import {
	tuneAccent,
	onAccent,
	contrastRatio,
	mixHex,
	ACCENT_TINT,
	type ThemeName,
} from '../../../packages/ui/src/color';

// ── Known WCAG vectors ───────────────────────────────────────────────────────
assert.ok(Math.abs(contrastRatio('#000000', '#ffffff') - 21) < 0.01, 'black on white is 21:1');
assert.ok(Math.abs(contrastRatio('#ffffff', '#ffffff') - 1) < 0.01, 'same color is 1:1');
assert.ok(Math.abs(contrastRatio('#8b7cf6', '#201c17') - 5.09) < 0.02, 'violet on dark surface ≈5.09');
assert.strictEqual(mixHex('#000000', '#ffffff', 1), '#000000', 'mix weight 1 = first color');
assert.strictEqual(mixHex('#000000', '#ffffff', 0), '#ffffff', 'mix weight 0 = second color');

// ── Every preset accent must clear AA in both themes (issues #142/#143) ──────
const PRESETS = ['#8b7cf6', '#4f8ef7', '#2fbf71', '#2bb6b6', '#e8873a', '#e5534b'];
const SURFACE: Record<ThemeName, string> = { dark: '#201c17', light: '#faf7f1' };
const THEMES: ThemeName[] = ['dark', 'light'];

for (const theme of THEMES) {
	for (const preset of PRESETS) {
		const tuned = tuneAccent(preset, theme);
		assert.ok(
			contrastRatio(tuned, SURFACE[theme]) >= 4.5,
			`${theme} ${preset}: accent text on surface must clear 4.5 (got ${contrastRatio(tuned, SURFACE[theme]).toFixed(2)})`,
		);
		const subtle = mixHex(tuned, SURFACE[theme], ACCENT_TINT[theme]);
		assert.ok(
			contrastRatio(tuned, subtle) >= 4.5,
			`${theme} ${preset}: accent text on accent-subtle must clear 4.5 (got ${contrastRatio(tuned, subtle).toFixed(2)})`,
		);
		const ink = onAccent(tuned);
		assert.ok(
			contrastRatio(ink, tuned) >= 4.5,
			`${theme} ${preset}: on-accent ink must clear 4.5 (got ${contrastRatio(ink, tuned).toFixed(2)})`,
		);
	}
}

// ── Custom colors, including extremes, must be tuned into range ──────────────
const CUSTOMS = ['#ffff00', '#00ff00', '#123456', '#d6006e', '#ffffff', '#000000', '#777777'];
for (const theme of THEMES) {
	for (const custom of CUSTOMS) {
		const tuned = tuneAccent(custom, theme);
		assert.ok(
			contrastRatio(tuned, SURFACE[theme]) >= 4.5,
			`${theme} ${custom}: accent text on surface must clear 4.5`,
		);
		const subtle = mixHex(tuned, SURFACE[theme], ACCENT_TINT[theme]);
		assert.ok(contrastRatio(tuned, subtle) >= 4.5, `${theme} ${custom}: accent text on subtle must clear 4.5`);
		assert.ok(contrastRatio(onAccent(tuned), tuned) >= 4.5, `${theme} ${custom}: on-accent ink must clear 4.5`);
	}
}

// Already-passing colors are returned untouched.
assert.strictEqual(tuneAccent('#9386f6', 'dark'), '#9386f6', 'a passing dark accent is not altered');
assert.strictEqual(tuneAccent('#645aad', 'light'), '#645aad', 'a passing light accent is not altered');

console.log('theme colors: PASS');
