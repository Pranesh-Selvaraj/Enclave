/* Runs synchronously in <head> so the first paint already has the right
   theme. Kept separate from main.js so the strict CSP can forbid inline
   scripts and style attributes. */
(function () {
  var root = document.documentElement;
  var stored = null;
  var theme;

  try {
    stored = localStorage.getItem('enclave-site-theme');
  } catch (error) {
    /* Storage can be disabled; fall back to the system preference. */
  }

  if (stored === 'light' || stored === 'dark') {
    theme = stored;
  } else {
    theme = window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  }

  root.setAttribute('data-theme', theme);

  var meta = document.querySelector('meta[name="theme-color"]');
  if (meta) {
    meta.setAttribute('content', theme === 'light' ? '#f1ede5' : '#16130f');
  }
})();
