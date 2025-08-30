import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
	const session = await event.locals.auth();
	
	if (!session?.user) {
		return json({ session: null });
	}

	// Get session token from cookies for WebSocket authentication
	const sessionToken = event.cookies.get('authjs.session-token');
	
	return json({
		session: {
			user: session.user,
			sessionToken // Include for WebSocket auth
		}
	});
};