import { initWebSocketServer } from './websocket-server';

// Initialize WebSocket server when the module loads
let wsServerInitialized = false;

export function ensureWebSocketServer() {
	if (!wsServerInitialized) {
		const port = parseInt(process.env.WS_PORT || '4001');
		initWebSocketServer(port);
		wsServerInitialized = true;
		console.log(`WebSocket server initialized on port ${port}`);
	}
}

// Auto-initialize in server environment
if (typeof window === 'undefined') {
	ensureWebSocketServer();
}