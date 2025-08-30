import adapter from '@sveltejs/adapter-auto';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// adapter-auto supports most environments, see https://svelte.dev/docs/kit/adapters for a list.
		adapter: adapter(),
		alias: {
			$db: './src/lib/server/db',
			$auth: './src/lib/server/auth',
			$stores: './src/lib/stores',
			$components: './src/lib/components'
		},
		// SvelteKit 2 enhanced configuration
		version: {
			pollInterval: 300
		}
	},
	
	// Svelte 5 compiler options
	compilerOptions: {
		runes: true
	}
};

export default config;