import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals, cookies }) => {
	const session = await locals.auth();
	
	if (!session?.user) {
		return json({ session: null });
	}

	// Get session token from cookies for WebSocket authentication
	const sessionToken = cookies.get('authjs.session-token');
	
	return json({
		session: {
			user: session.user,
			sessionToken // Include for WebSocket auth
		}
	});
};