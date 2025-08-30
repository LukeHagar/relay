import { WebSocketServer } from 'ws';
import { parse } from 'url';
import { verify } from 'jsonwebtoken';
import { prisma } from '$db';

// Store for WebSocket connections
const wsConnections = new Map<string, Set<any>>();

export interface WebhookEvent {
	id: string;
	type: 'webhook' | 'system';
	data: any;
}

let wss: WebSocketServer | null = null;

/**
 * Initialize WebSocket server
 */
export function initWebSocketServer(server: any) {
	if (wss) return wss;

	wss = new WebSocketServer({ 
		server,
		path: '/api/relay/ws',
		verifyClient: async (info) => {
			try {
				// Extract session token from URL or headers
				const url = parse(info.req.url!, true);
				const token = url.query.token as string || 
							 info.req.headers.authorization?.replace('Bearer ', '');
				
				if (!token) {
					console.log('WebSocket connection rejected: No token provided');
					return false;
				}

				// Verify session token (simplified - in production use proper JWT verification)
				const session = await prisma.session.findUnique({
					where: { sessionToken: token },
					include: { user: true }
				});

				if (!session || session.expires < new Date()) {
					console.log('WebSocket connection rejected: Invalid or expired token');
					return false;
				}

				// Store user info for connection
				(info.req as any).userId = session.userId;
				return true;
			} catch (error) {
				console.error('WebSocket verification error:', error);
				return false;
			}
		}
	});

	wss.on('connection', (ws, req) => {
		const userId = (req as any).userId;
		
		if (!userId) {
			ws.close(1008, 'Invalid authentication');
			return;
		}

		// Add connection
		addWSConnection(userId, ws);

		// Handle messages
		ws.on('message', (data) => {
			try {
				const message = JSON.parse(data.toString());
				console.log(`WebSocket message from user ${userId}:`, message);
				
				// Handle ping/pong for connection health
				if (message.type === 'ping') {
					ws.send(JSON.stringify({ 
						type: 'pong', 
						timestamp: new Date().toISOString() 
					}));
				}
			} catch (error) {
				console.error('Failed to parse WebSocket message:', error);
			}
		});

		// Handle connection close
		ws.on('close', () => {
			removeWSConnection(userId, ws);
		});

		// Handle errors
		ws.on('error', (error) => {
			console.error(`WebSocket error for user ${userId}:`, error);
			removeWSConnection(userId, ws);
		});
	});

	console.log('WebSocket server initialized on path /api/relay/ws');
	return wss;
}

/**
 * Add a WebSocket connection for a user
 */
export function addWSConnection(userId: string, ws: any) {
	if (!wsConnections.has(userId)) {
		wsConnections.set(userId, new Set());
	}
	wsConnections.get(userId)!.add(ws);
	
	// Send initial connection message
	sendWSMessage(ws, {
		id: crypto.randomUUID(),
		type: 'system',
		data: { 
			message: 'Connected to webhook relay', 
			timestamp: new Date().toISOString(),
			userId 
		}
	});
	
	console.log(`WebSocket connected for user ${userId}. Total connections: ${wsConnections.get(userId)?.size}`);
}

/**
 * Remove a WebSocket connection for a user
 */
export function removeWSConnection(userId: string, ws: any) {
	const userConnections = wsConnections.get(userId);
	if (userConnections) {
		userConnections.delete(ws);
		if (userConnections.size === 0) {
			wsConnections.delete(userId);
		}
		console.log(`WebSocket disconnected for user ${userId}. Remaining connections: ${userConnections.size}`);
	}
}

/**
 * Send message to a specific WebSocket connection
 */
function sendWSMessage(ws: any, event: WebhookEvent) {
	try {
		if (ws.readyState === 1) { // WebSocket.OPEN
			ws.send(JSON.stringify(event));
		}
	} catch (error) {
		console.error('Failed to send WebSocket message:', error);
	}
}

/**
 * Broadcast event to all connections for a specific user
 */
export async function broadcastToUser(userId: string, event: WebhookEvent): Promise<boolean> {
	const userConnections = wsConnections.get(userId);
	
	if (!userConnections || userConnections.size === 0) {
		console.log(`No WebSocket connections found for user ${userId}`);
		return false;
	}

	let successCount = 0;
	const totalConnections = userConnections.size;
	const failedConnections: any[] = [];

	userConnections.forEach(ws => {
		try {
			if (ws.readyState === 1) { // WebSocket.OPEN
				sendWSMessage(ws, event);
				successCount++;
			} else {
				// Mark for removal if connection is closed
				failedConnections.push(ws);
			}
		} catch (error) {
			console.error('Failed to broadcast to WebSocket connection:', error);
			failedConnections.push(ws);
		}
	});

	// Clean up failed connections
	failedConnections.forEach(ws => {
		userConnections.delete(ws);
	});

	console.log(`Broadcast to ${successCount}/${totalConnections} connections for user ${userId}`);
	return successCount > 0;
}

/**
 * Get connection count for a user
 */
export function getConnectionCount(userId: string): number {
	return wsConnections.get(userId)?.size || 0;
}

/**
 * Get total connection count
 */
export function getTotalConnections(): number {
	let total = 0;
	wsConnections.forEach(connections => {
		total += connections.size;
	});
	return total;
}