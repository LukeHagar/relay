import { SvelteKitAuth } from '@auth/sveltekit';
import GitHub from '@auth/core/providers/github';
import { PrismaAdapter } from '@auth/prisma-adapter';
import { prisma } from './db';
import { 
	GITHUB_CLIENT_ID, 
	GITHUB_CLIENT_SECRET, 
	AUTH_SECRET,
	REDIRECT_URL 
} from '$env/static/private';

export const { handle, signIn, signOut } = SvelteKitAuth({
	adapter: PrismaAdapter(prisma),
	providers: [
		GitHub({
			clientId: GITHUB_CLIENT_ID,
			clientSecret: GITHUB_CLIENT_SECRET,
			profile(profile) {
				return {
					id: profile.id.toString(),
					name: profile.name ?? profile.login,
					username: profile.login,
					email: profile.email,
					image: profile.avatar_url,
					subdomain: crypto.randomUUID().slice(0, 8), // Shorter subdomain
				};
			},
		})
	],
	secret: AUTH_SECRET,
	trustHost: true,
	callbacks: {
		session: async ({ session, user }) => {
			// Add custom user data to session
			const dbUser = await prisma.user.findUnique({
				where: { id: user.id },
				select: { subdomain: true, username: true }
			});
			
			if (dbUser) {
				session.user.subdomain = dbUser.subdomain;
				session.user.username = dbUser.username;
			}
			
			return session;
		},
		redirect: async ({ url, baseUrl }) => {
			// Redirect to dashboard after sign in
			if (url.startsWith(baseUrl)) return url;
			return REDIRECT_URL || `${baseUrl}/dashboard`;
		}
	}
});