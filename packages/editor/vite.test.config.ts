// Test-only vite config: bundles test-insert.ts (which imports the editor's
// extensions, some of which are Svelte node views) into a node-runnable file.
// The components compile in CLIENT mode (so mount() works against jsdom) —
// dynamicCompileOptions overrides the plugin's server-mode default for SSR
// builds — and the test runs with `node --conditions=browser` so the bare
// 'svelte' import resolves to the client runtime.
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
	plugins: [svelte({ emitCss: false, compilerOptions: { generate: 'client' } })],
	resolve: {
		// Node resolves the bare 'svelte' export to the server runtime; the
		// client-compiled components need the client runtime (mount works
		// against jsdom here).
		alias: { svelte: '/home/deblac/projects/Enclave/node_modules/svelte/src/index-client.js' },
	},
	build: {
		lib: { entry: 'test-insert.ts', formats: ['es'], fileName: () => 'test-insert.js' },
		outDir: 'dist-test',
		rollupOptions: {
			// Keep node_modules external (jsdom, tiptap) — loaded at runtime —
			// but bundle the @enclave workspace packages (they contain .svelte
			// files node can't import) and svelte's #client/* internal imports.
			external: (id) =>
				id !== 'svelte' &&
				!id.startsWith('.') && !id.startsWith('/') && !id.startsWith('#') && !id.startsWith('@enclave/'),
		},
	},
});
