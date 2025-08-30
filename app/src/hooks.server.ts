import { SvelteKitAuth } from '@auth/sveltekit';
import GitHub from '@auth/core/providers/github';
import { PrismaAdapter } from '@auth/prisma-adapter';
import { getPrisma } from '$lib/server/prisma';
import { ensureWebSocketServer } from '$lib/server/ws';
import { building } from '$app/environment';

if (!building) {
	void ensureWebSocketServer();
}

export const handle = SvelteKitAuth(async () => {
	const prisma = await getPrisma();
	return {
		basePath: '/api/auth',
		secret: process.env.AUTH_SECRET,
		trustHost: true,
		adapter: PrismaAdapter(prisma),
		providers: [
			GitHub({
				clientId: process.env.GITHUB_CLIENT_ID!,
				clientSecret: process.env.GITHUB_CLIENT_SECRET!,
				profile(profile) {
					return {
						id: profile.id.toString(),
						name: profile.name ?? profile.login,
						username: profile.login,
						email: profile.email,
						image: profile.avatar_url,
						subdomain: crypto.randomUUID()
					};
				}
			})
		],
		callbacks: {
			session: async ({ session, user }) => {
				// @ts-expect-error - add custom fields
				session.user.id = user.id;
				// @ts-expect-error - add custom fields
				session.user.subdomain = (user as any).subdomain;
				return session;
			}
		}
	};
});

