// Test-only vite config: bundles the editor test entrypoints (which import the
// editor's extensions, some of which are Svelte node views) into node-runnable
// files. All node_modules stay external; the tests run with
// `node --conditions=browser` so bare 'svelte' resolves to the client runtime —
// the same instance the components' 'svelte/internal/client' imports use (a
// split runtime makes svelte throw effect_orphan on mount).
import { fileURLToPath } from 'node:url';
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
		ssr: true,
		outDir: 'dist-test',
		ssrEmitAssets: false,
		rollupOptions: {
			input: {
				'test-insert': fileURLToPath(new URL('./test-insert.ts', import.meta.url)),
				'test-elements': fileURLToPath(new URL('./test-elements.ts', import.meta.url)),
			},
			external: ['svelte', 'svelte/internal/client'],
		},
	},
});
