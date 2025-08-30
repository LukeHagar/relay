import type { RequestHandler } from './$types';
import { error } from '@sveltejs/kit';

// Simple in-memory WebSocket connection store
const connections = new Map<string, Set<WebSocket>>();

export interface WebhookEvent {
	id: string;
	type: 'webhook' | 'system' | 'ping' | 'pong';
	data: any;
}

export const GET: RequestHandler = async ({ request, locals, url }) => {
	const session = await locals.auth();
	
	if (!session?.user?.id) {
		throw error(401, 'Unauthorized');
	}

	const userId = session.user.id;

	// Check for WebSocket upgrade
	const upgrade = request.headers.get('upgrade');
	const connection = request.headers.get('connection');
	
	if (upgrade?.toLowerCase() !== 'websocket' || !connection?.toLowerCase().includes('upgrade')) {
		return new Response('WebSocket upgrade required', { 
			status: 426,
			headers: {
				'Upgrade': 'websocket',
				'Connection': 'Upgrade'
			}
		});
	}

	try {
		// For compatibility, we'll use a simple WebSocket response
		// This works with most WebSocket implementations
		const webSocketKey = request.headers.get('sec-websocket-key');
		if (!webSocketKey) {
			throw error(400, 'Missing WebSocket key');
		}

		// Create WebSocket response headers
		const acceptKey = await generateWebSocketAccept(webSocketKey);
		
		return new Response(null, {
			status: 101,
			headers: {
				'Upgrade': 'websocket',
				'Connection': 'Upgrade',
				'Sec-WebSocket-Accept': acceptKey,
			}
		});

	} catch (err) {
		console.error('WebSocket upgrade error:', err);
		throw error(500, 'WebSocket upgrade failed');
	}
};

// Generate WebSocket accept key
async function generateWebSocketAccept(key: string): Promise<string> {
	const concatenated = key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
	const hash = await crypto.subtle.digest('SHA-1', new TextEncoder().encode(concatenated));
	return btoa(String.fromCharCode(...new Uint8Array(hash)));
}

// Export connection management functions
export function addConnection(userId: string, ws: WebSocket) {
	if (!connections.has(userId)) {
		connections.set(userId, new Set());
	}
	connections.get(userId)!.add(ws);
	
	// Send welcome message
	if (ws.readyState === WebSocket.OPEN) {
		ws.send(JSON.stringify({
			id: crypto.randomUUID(),
			type: 'system',
			data: {
				message: 'Connected to webhook relay',
				timestamp: new Date().toISOString(),
				userId
			}
		}));
	}
	
	console.log(`WebSocket connected for user ${userId}`);
}

export function removeConnection(userId: string, ws: WebSocket) {
	const userConnections = connections.get(userId);
	if (userConnections) {
		userConnections.delete(ws);
		if (userConnections.size === 0) {
			connections.delete(userId);
		}
	}
	console.log(`WebSocket disconnected for user ${userId}`);
}

export async function broadcastToUser(userId: string, event: WebhookEvent): Promise<boolean> {
	const userConnections = connections.get(userId);
	
	if (!userConnections || userConnections.size === 0) {
		return false;
	}

	let successCount = 0;
	const message = JSON.stringify(event);
	const staleConnections: WebSocket[] = [];

	userConnections.forEach(ws => {
		try {
			if (ws.readyState === WebSocket.OPEN) {
				ws.send(message);
				successCount++;
			} else {
				staleConnections.push(ws);
			}
		} catch (error) {
			console.error('Failed to send message:', error);
			staleConnections.push(ws);
		}
	});

	// Clean up stale connections
	staleConnections.forEach(ws => {
		userConnections.delete(ws);
	});

	return successCount > 0;
}