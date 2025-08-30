// WebSocket connection manager for SvelteKit
// This provides a simple in-memory WebSocket management system

export interface WebhookEvent {
	id: string;
	type: 'webhook' | 'system' | 'ping' | 'pong';
	data: any;
}

export interface WSConnection {
	ws: WebSocket;
	userId: string;
	connected: boolean;
	lastPing?: number;
}

// Store WebSocket connections by user ID
const connections = new Map<string, Set<WSConnection>>();

/**
 * Add a WebSocket connection for a user
 */
export function addConnection(userId: string, ws: WebSocket): WSConnection {
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
	
	// Set up connection event handlers
	ws.addEventListener('close', () => {
		connection.connected = false;
		removeConnection(userId, connection);
	});

	ws.addEventListener('error', (error) => {
		console.error(`WebSocket error for user ${userId}:`, error);
		connection.connected = false;
		removeConnection(userId, connection);
	});

	ws.addEventListener('message', (event) => {
		try {
			const message = JSON.parse(event.data);
			handleMessage(connection, message);
		} catch (error) {
			console.error('Failed to parse WebSocket message:', error);
		}
	});

	// Send welcome message
	sendMessage(connection, {
		id: crypto.randomUUID(),
		type: 'system',
		data: {
			message: 'Connected to webhook relay',
			timestamp: new Date().toISOString(),
			userId
		}
	});

	console.log(`WebSocket connected for user ${userId}. Total connections: ${connections.get(userId)?.size}`);
	return connection;
}

/**
 * Remove a WebSocket connection
 */
export function removeConnection(userId: string, connection: WSConnection) {
	const userConnections = connections.get(userId);
	if (userConnections) {
		userConnections.delete(connection);
		if (userConnections.size === 0) {
			connections.delete(userId);
		}
		console.log(`WebSocket disconnected for user ${userId}. Remaining: ${userConnections.size}`);
	}
}

/**
 * Handle incoming WebSocket messages
 */
function handleMessage(connection: WSConnection, message: any) {
	switch (message.type) {
		case 'ping':
			connection.lastPing = Date.now();
			sendMessage(connection, {
				id: crypto.randomUUID(),
				type: 'pong',
				data: { timestamp: new Date().toISOString() }
			});
			break;
		default:
			console.log(`Received message from user ${connection.userId}:`, message);
	}
}

/**
 * Send message to a specific connection
 */
function sendMessage(connection: WSConnection, event: WebhookEvent) {
	try {
		if (connection.connected && connection.ws.readyState === WebSocket.OPEN) {
			connection.ws.send(JSON.stringify(event));
		}
	} catch (error) {
		console.error('Failed to send WebSocket message:', error);
		connection.connected = false;
	}
}

/**
 * Broadcast event to all connections for a specific user
 */
export async function broadcastToUser(userId: string, event: WebhookEvent): Promise<boolean> {
	const userConnections = connections.get(userId);
	
	if (!userConnections || userConnections.size === 0) {
		console.log(`No WebSocket connections found for user ${userId}`);
		return false;
	}

	let successCount = 0;
	const failedConnections: WSConnection[] = [];

	userConnections.forEach(connection => {
		try {
			if (connection.connected && connection.ws.readyState === WebSocket.OPEN) {
				sendMessage(connection, event);
				successCount++;
			} else {
				failedConnections.push(connection);
			}
		} catch (error) {
			console.error('Failed to broadcast to connection:', error);
			failedConnections.push(connection);
		}
	});

	// Clean up failed connections
	failedConnections.forEach(connection => {
		removeConnection(userId, connection);
	});

	console.log(`Broadcast to ${successCount}/${userConnections.size} connections for user ${userId}`);
	return successCount > 0;
}

/**
 * Get connection statistics
 */
export function getConnectionStats() {
	const stats = {
		totalUsers: connections.size,
		totalConnections: 0,
		userConnections: new Map<string, number>()
	};

	connections.forEach((userConnections, userId) => {
		const activeConnections = Array.from(userConnections).filter(c => c.connected).length;
		stats.totalConnections += activeConnections;
		stats.userConnections.set(userId, activeConnections);
	});

	return stats;
}

/**
 * Cleanup stale connections (call periodically)
 */
export function cleanupStaleConnections() {
	const now = Date.now();
	const staleThreshold = 5 * 60 * 1000; // 5 minutes

	connections.forEach((userConnections, userId) => {
		const staleConnections: WSConnection[] = [];
		
		userConnections.forEach(connection => {
			if (!connection.connected || 
			    (connection.lastPing && now - connection.lastPing > staleThreshold)) {
				staleConnections.push(connection);
			}
		});

		staleConnections.forEach(connection => {
			removeConnection(userId, connection);
		});
	});
}