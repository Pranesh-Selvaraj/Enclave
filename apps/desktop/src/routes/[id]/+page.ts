// Dynamic route: doc ids are runtime data, so this page can't be prerendered.
// Without this, the static adapter bakes a 404 "Not Found" error into the
// index.html fallback and every full page load on a document 404s.
export const prerender = false;
export const ssr = false;
