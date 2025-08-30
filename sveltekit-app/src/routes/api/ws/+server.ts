import { prisma } from '$lib/db';
import { clients } from '../../webhook/[...path]/+server.js';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ request, locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return new Response('Unauthorized', { status: 401 });
	}

	// Get user data
	const user = await prisma.user.findUnique({
		where: { id: session.user.id },
	});

	if (!user) {
		return new Response('Unauthorized', { status: 401 });
	}

	// Upgrade to WebSocket
	const upgrade = request.headers.get('upgrade');
	if (upgrade !== 'websocket') {
		return new Response('Expected websocket', { status: 426 });
	}

	const { socket, response } = Deno.upgradeWebSocket(request);

	// Add client to the map
	if (!clients.has(user.subdomain)) {
		clients.set(user.subdomain, []);
	}
	clients.get(user.subdomain)!.push(socket);

	// Send welcome message
	socket.send(JSON.stringify({
		message: `Connected to WebSocket server as ${user.name} with subdomain ${user.subdomain}`,
		type: 'connection'
	}));

	// Handle WebSocket events
	socket.onopen = () => {
		console.log(`WebSocket connected for user: ${user.subdomain}`);
	};

	socket.onmessage = (event) => {
		try {
			const data = JSON.parse(event.data);
			console.log('Received message:', data);
			
			// Echo back for testing
			socket.send(JSON.stringify({
				message: 'Message received',
				type: 'echo',
				data: data
			}));
		} catch (error) {
			console.error('Error parsing WebSocket message:', error);
		}
	};

	socket.onclose = () => {
		console.log(`WebSocket disconnected for user: ${user.subdomain}`);
		
		// Remove client from the map
		const userClients = clients.get(user.subdomain);
		if (userClients) {
			const index = userClients.indexOf(socket);
			if (index > -1) {
				userClients.splice(index, 1);
			}
			
			// Remove empty arrays
			if (userClients.length === 0) {
				clients.delete(user.subdomain);
			}
		}
	};

	socket.onerror = (error) => {
		console.error(`WebSocket error for user ${user.subdomain}:`, error);
	};

	return response;
};