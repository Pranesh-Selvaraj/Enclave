// Pre-paint appearance bootstrap.
//
// The Tauri CSP forbids inline scripts, so this same-origin file runs before
// the app bundle and applies the saved theme + appearance presets to <html>.
// Without it, the first paint (login/unlock page, new windows) uses the
// default dark theme until the Svelte bundle boots — a flash that made the
// vault screen look disconnected from the chosen theme.
//
// Keep in sync with packages/ui/src/theme.svelte.ts (init/apply).
(function () {
	var ACCENTS = {
		violet: '#8b7cf6',
		blue: '#4f8ef7',
		green: '#2fbf71',
		teal: '#2bb6b6',
		orange: '#e8873a',
		red: '#e5534b',
	};
	try {
		var root = document.documentElement;
		var mode = localStorage.getItem('enclave-theme') || 'auto';
		var dark =
			mode === 'dark' ||
			(mode === 'auto' && window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
		var s = JSON.parse(localStorage.getItem('enclave-settings') || '{}');
		root.setAttribute('data-theme', dark ? 'dark' : 'light');
		// Accent: preset ids keep the CSS preset path; raw hex values are
		// forced inline so custom accents work everywhere too.
		var accent = typeof s.accent === 'string' ? s.accent : 'violet';
		var isPreset = Object.prototype.hasOwnProperty.call(ACCENTS, accent);
		root.setAttribute('data-accent', isPreset ? accent : 'custom');
		root.style.setProperty(
			'--color-accent',
			ACCENTS[accent] || (/^#[0-9a-f]{6}$/i.test(accent) ? accent : ACCENTS.violet),
		);
		if (s.font) root.setAttribute('data-font', s.font);
		if (s.density) root.setAttribute('data-density', s.density);
		if (s.fontSize) root.setAttribute('data-font-size', s.fontSize);
		if (s.pageWidth) root.setAttribute('data-page-width', s.pageWidth);
		if (s.corners) root.setAttribute('data-corners', s.corners);
		if (s.uiScale) root.setAttribute('data-ui-scale', s.uiScale);
		if (s.background) root.setAttribute('data-bg', s.background);
		if (s.trueBlack) root.setAttribute('data-true-black', '');
		if (s.reduceMotion) root.setAttribute('data-reduce-motion', '');
	} catch (e) {
		/* first run / private mode — defaults are fine */
	}
})();
