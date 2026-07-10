import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    files: {
      appTemplate: '../../src/app.html',
      routes: '../../src/routes',
      lib: '../../src/lib',
    },
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      strict: true
    }),
    prerender: {
      handleHttpError: 'warn',
      handleMissingId: 'warn',
      handleUnseenRoutes: 'ignore',
      entries: ['/']
    }
  }
};

export default config;
