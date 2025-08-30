#!/usr/bin/env node

// Standalone WebSocket server for development
import { WebSocketServer } from 'ws';
import { createServer } from 'http';
import { parse } from 'url';

// Simple in-memory connection storage
const connections = new Map();

// Create HTTP server for WebSocket upgrade
const server = createServer();

const wss = new WebSocketServer({ 
	server,
	verifyClient: (info) => {
		// In development, allow all connections
		// In production, implement proper token verification
		const url = parse(info.req.url, true);
		const token = url.query.token;
		
		if (!token) {
			console.log('WebSocket connection rejected: No token');
			return false;
		}
		
		// Store token for connection (simplified for dev)
		info.req.token = token;
		return true;
	}
});

wss.on('connection', (ws, req) => {
	const token = req.token;
	const userId = `user-${token.slice(-8)}`; // Simplified user ID
	
	console.log(`WebSocket connected for user ${userId}`);
	
	// Store connection
	if (!connections.has(userId)) {
		connections.set(userId, new Set());
	}
	connections.get(userId).add(ws);
	
	// Send welcome message
	ws.send(JSON.stringify({
		id: Date.now().toString(),
		type: 'system',
		data: {
			message: 'Connected to webhook relay',
			timestamp: new Date().toISOString(),
			userId
		}
	}));

	// Handle messages
	ws.on('message', (data) => {
		try {
			const message = JSON.parse(data.toString());
			console.log(`Message from ${userId}:`, message.type);
			
			// Handle ping/pong
			if (message.type === 'ping') {
				ws.send(JSON.stringify({
					id: Date.now().toString(),
					type: 'pong',
					data: { timestamp: new Date().toISOString() }
				}));
			}
		} catch (error) {
			console.error('Failed to parse message:', error);
		}
	});

	// Handle close
	ws.on('close', () => {
		console.log(`WebSocket disconnected for user ${userId}`);
		const userConnections = connections.get(userId);
		if (userConnections) {
			userConnections.delete(ws);
			if (userConnections.size === 0) {
				connections.delete(userId);
			}
		}
	});

	// Handle errors
	ws.on('error', (error) => {
		console.error(`WebSocket error for user ${userId}:`, error);
	});
});

// Broadcast function for testing
global.broadcastToUser = (userId, event) => {
	const userConnections = connections.get(userId);
	if (userConnections) {
		userConnections.forEach(ws => {
			if (ws.readyState === 1) { // OPEN
				ws.send(JSON.stringify(event));
			}
		});
		return true;
	}
	return false;
};

const port = process.env.WS_PORT || 4001;
server.listen(port, () => {
	console.log(`WebSocket server listening on port ${port}`);
	console.log(`WebSocket URL: ws://localhost:${port}`);
});

// Graceful shutdown
process.on('SIGINT', () => {
	console.log('\nShutting down WebSocket server...');
	wss.close(() => {
		server.close(() => {
			process.exit(0);
		});
	});
});

process.on('SIGTERM', () => {
	wss.close(() => {
		server.close(() => {
			process.exit(0);
		});
	});
});