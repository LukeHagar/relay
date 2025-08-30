import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getRecentWebhooks } from '$lib/server/relay';

export const GET: RequestHandler = async (event) => {
	const session = await event.locals.auth();
	
	if (!session?.user?.id) {
		error(401, 'Unauthorized');
	}

	const limit = parseInt(event.url.searchParams.get('limit') || '50');
	const webhooks = await getRecentWebhooks(session.user.id, limit);

	return json({
		webhooks: webhooks.map(webhook => ({
			...webhook,
			body: webhook.body ? JSON.parse(webhook.body) : null,
			headers: webhook.headers ? JSON.parse(webhook.headers) : null
		})),
		total: webhooks.length,
		timestamp: new Date().toISOString()
	});
};