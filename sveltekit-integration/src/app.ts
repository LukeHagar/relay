// Initialize WebSocket server when the app starts
import { initWebSocketServer } from '$lib/server/websocket-server';

// Initialize WebSocket server in server environment
if (typeof window === 'undefined') {
	const wsPort = parseInt(process.env.WS_PORT || '4001');
	
	// Delay initialization to ensure everything is ready
	setTimeout(() => {
		try {
			initWebSocketServer(wsPort);
			console.log(`WebSocket server started on port ${wsPort}`);
		} catch (error) {
			console.error('Failed to start WebSocket server:', error);
		}
	}, 1000);
}