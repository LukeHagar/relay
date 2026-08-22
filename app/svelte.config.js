import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// SPA mode: the engine serves index.html for every non-API path and the
		// client router takes over. No SSR runtime in the binary — fastest load.
		adapter: adapter({ fallback: "index.html", pages: "build", assets: "build" })
	}
};

export default config;
