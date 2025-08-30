import type { RequestHandler } from './$types';
import { error } from '@sveltejs/kit';
import { addSSEConnection, removeSSEConnection } from '$lib/server/relay';

export const GET: RequestHandler = async ({ url, locals }) => {
	const session = await locals.auth();
	
	if (!session?.user?.id) {
		throw error(401, 'Unauthorized');
	}

	const userId = session.user.id;

	// Create Server-Sent Events stream
	const stream = new ReadableStream({
		start(controller) {
			// Add this connection to our tracking
			addSSEConnection(userId, controller);

			// Set up periodic heartbeat to keep connection alive
			const heartbeat = setInterval(() => {
				try {
					controller.enqueue(new TextEncoder().encode(': heartbeat\n\n'));
				} catch (error) {
					clearInterval(heartbeat);
				}
			}, 30000); // 30 seconds

			// Clean up on stream close
			return () => {
				clearInterval(heartbeat);
				removeSSEConnection(userId, controller);
			};
		},
		cancel() {
			removeSSEConnection(userId, this);
		}
	});

	return new Response(stream, {
		headers: {
			'Content-Type': 'text/event-stream',
			'Cache-Control': 'no-cache',
			'Connection': 'keep-alive',
			'Access-Control-Allow-Origin': '*',
			'Access-Control-Allow-Headers': 'Cache-Control'
		}
	});
};