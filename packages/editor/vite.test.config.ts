// Test-only vite config: bundles test-insert.ts (which imports the editor's
// extensions, some of which are Svelte node views) into a node-runnable file.
// All node_modules stay external; the test runs with `node --conditions=browser`
// so bare 'svelte' resolves to the client runtime — the same instance the
// components' 'svelte/internal/client' imports use (a split runtime makes
// svelte throw effect_orphan on mount).
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
	plugins: [svelte({ emitCss: false })],
	ssr: {
		// Keep ALL deps external (vite 6 defaults to bundling them) so node
		// resolves svelte with --conditions=browser to the client runtime.
		noExternal: [],
	},
	build: {
		ssr: 'test-insert.ts',
		outDir: 'dist-test',
		ssrEmitAssets: false,
		rollupOptions: {
			external: ['svelte', 'svelte/internal/client'],
		},
	},
});
