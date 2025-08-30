import { WebSocketServer } from 'ws';
import { createServer } from 'http';
import { parse } from 'url';
import { prisma } from '$db';

export interface WebhookEvent {
	id: string;
	type: 'webhook' | 'system' | 'ping' | 'pong';
	data: any;
}

export interface WSConnection {
	ws: any;
	userId: string;
	connected: boolean;
	lastPing: number;
}

// Connection storage
const connections = new Map<string, Set<WSConnection>>();
let wss: WebSocketServer | null = null;
let httpServer: any = null;

/**
 * Initialize WebSocket server on a separate port
 */
export function initWebSocketServer(port = 4001) {
	if (wss) return { wss, httpServer };

	// Create HTTP server for WebSocket upgrade
	httpServer = createServer();
	
	wss = new WebSocketServer({ 
		server: httpServer,
		verifyClient: async (info) => {
			try {
				// Extract token from query params or headers
				const url = parse(info.req.url!, true);
				const token = url.query.token as string;
				
				if (!token) {
					console.log('WebSocket rejected: No token');
					return false;
				}

				// Verify session token
				const session = await prisma.session.findUnique({
					where: { sessionToken: token },
					include: { user: true }
				});

				if (!session || session.expires < new Date()) {
					console.log('WebSocket rejected: Invalid token');
					return false;
				}

				// Store user info for this request
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
			ws.close(1008, 'Authentication required');
			return;
		}

		const connection = addConnection(userId, ws);

		// Handle messages
		ws.on('message', (data) => {
			try {
				const message = JSON.parse(data.toString());
				handleMessage(connection, message);
			} catch (error) {
				console.error('Failed to parse message:', error);
			}
		});

		// Handle connection close
		ws.on('close', () => {
			removeConnection(userId, connection);
		});

		// Handle errors
		ws.on('error', (error) => {
			console.error(`WebSocket error for user ${userId}:`, error);
			removeConnection(userId, connection);
		});
	});

	// Start HTTP server
	httpServer.listen(port, () => {
		console.log(`WebSocket server listening on port ${port}`);
	});

	// Cleanup stale connections every 5 minutes
	setInterval(cleanupStaleConnections, 5 * 60 * 1000);

	return { wss, httpServer };
}

/**
 * Add a WebSocket connection
 */
function addConnection(userId: string, ws: any): WSConnection {
	const connection: WSConnection = {
		ws,
		userId,
		connected: true,
		lastPing: Date.now()
	};

	if (!connections.has(userId)) {
		connections.set(userId, new Set());
	}
	
	connections.get(userId)!.add(connection);

	// Send welcome message
	sendMessage(connection, {
		id: crypto.randomUUID(),
		type: 'system',
		data: {
			message: 'Connected to webhook relay',
			timestamp: new Date().toISOString(),
			connectionCount: connections.get(userId)?.size || 1
		}
	});

	return connection;
}

/**
 * Remove a WebSocket connection
 */
function removeConnection(userId: string, connection: WSConnection) {
	const userConnections = connections.get(userId);
	if (userConnections) {
		userConnections.delete(connection);
		if (userConnections.size === 0) {
			connections.delete(userId);
		}
	}
	connection.connected = false;
}

/**
 * Handle incoming messages
 */
function handleMessage(connection: WSConnection, message: any) {
	connection.lastPing = Date.now();
	
	switch (message.type) {
		case 'ping':
			sendMessage(connection, {
				id: crypto.randomUUID(),
				type: 'pong',
				data: { timestamp: new Date().toISOString() }
			});
			break;
		case 'subscribe':
			// Handle subscription to specific event types
			break;
		default:
			console.log(`Unknown message type: ${message.type}`);
	}
}

/**
 * Send message to a specific connection
 */
function sendMessage(connection: WSConnection, event: WebhookEvent) {
	try {
		if (connection.connected && connection.ws.readyState === 1) { // OPEN
			connection.ws.send(JSON.stringify(event));
		}
	} catch (error) {
		console.error('Failed to send message:', error);
		connection.connected = false;
	}
}

/**
 * Broadcast to all user connections
 */
export async function broadcastToUser(userId: string, event: WebhookEvent): Promise<boolean> {
	const userConnections = connections.get(userId);
	
	if (!userConnections || userConnections.size === 0) {
		return false;
	}

	let successCount = 0;
	const failedConnections: WSConnection[] = [];

	userConnections.forEach(connection => {
		try {
			if (connection.connected && connection.ws.readyState === 1) {
				sendMessage(connection, event);
				successCount++;
			} else {
				failedConnections.push(connection);
			}
		} catch (error) {
			failedConnections.push(connection);
		}
	});

	// Remove failed connections
	failedConnections.forEach(connection => {
		removeConnection(userId, connection);
	});

	return successCount > 0;
}

/**
 * Get connection statistics
 */
export function getStats() {
	let totalConnections = 0;
	const userStats = new Map<string, number>();

	connections.forEach((userConnections, userId) => {
		const activeCount = Array.from(userConnections).filter(c => c.connected).length;
		totalConnections += activeCount;
		userStats.set(userId, activeCount);
	});

	return {
		totalUsers: connections.size,
		totalConnections,
		userStats
	};
}

/**
 * Cleanup stale connections
 */
function cleanupStaleConnections() {
	const now = Date.now();
	const staleThreshold = 5 * 60 * 1000; // 5 minutes

	connections.forEach((userConnections, userId) => {
		const staleConnections: WSConnection[] = [];
		
		userConnections.forEach(connection => {
			if (!connection.connected || 
			    (now - connection.lastPing > staleThreshold)) {
				staleConnections.push(connection);
			}
		});

		staleConnections.forEach(connection => {
			removeConnection(userId, connection);
		});
	});

	console.log('Cleaned up stale WebSocket connections');
}

/**
 * Shutdown WebSocket server
 */
export function shutdown() {
	if (wss) {
		wss.close();
		wss = null;
	}
	if (httpServer) {
		httpServer.close();
		httpServer = null;
	}
}