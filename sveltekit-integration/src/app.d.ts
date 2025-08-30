import type { DefaultSession } from '@auth/core/types';

declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			auth: () => Promise<{
				user?: {
					id: string;
					name?: string | null;
					email?: string | null;
					image?: string | null;
					subdomain?: string;
					username?: string;
				};
			} | null>;
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

declare module '@auth/core/types' {
	interface User {
		subdomain?: string;
		username?: string;
	}
	
	interface Session extends DefaultSession {
		user?: User;
	}
}

export {};