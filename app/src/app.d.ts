// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			auth: import('@auth/sveltekit').RequestEvent['locals']['auth'];
			getSession: import('@auth/sveltekit').RequestEvent['locals']['getSession'];
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};

