import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

export interface WebhookEvent {
	id: string;
	method: string;
	path: string;
	query: string;
	body: any;
	headers: any;
	createdAt: string;
	timestamp?: string;
}

export interface RelayTarget {
	id: string;
	target: string;
	nickname?: string;
	active: boolean;
	createdAt: string;
}

// Stores
export const webhookEvents = writable<WebhookEvent[]>([]);
export const relayTargets = writable<RelayTarget[]>([]);
export const connectionStatus = writable<'connected' | 'disconnected' | 'connecting'>('disconnected');
export const isLoading = writable(false);

// Derived stores
export const activeTargets = derived(relayTargets, $targets => 
	$targets.filter(target => target.active)
);

export const recentEvents = derived(webhookEvents, $events => 
	$events.slice(0, 10)
);

// WebSocket Connection management
let websocket: WebSocket | null = null;
let reconnectTimeout: number | null = null;
let pingInterval: number | null = null;

export const webhookStore = {
	// Initialize WebSocket connection
	connect: async () => {
		if (!browser) return;
		
		connectionStatus.set('connecting');
		
		// Create WebSocket connection to separate WebSocket server
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		const wsPort = 4001; // WebSocket server port
		
		// Get session token from cookie for authentication
		const sessionToken = document.cookie
			.split('; ')
			.find(row => row.startsWith('authjs.session-token='))
			?.split('=')[1];

		if (!sessionToken) {
			console.error('No session token found');
			connectionStatus.set('disconnected');
			return;
		}
		
		const wsUrl = `${protocol}//${window.location.hostname}:${wsPort}?token=${sessionToken}`;
		
		try {
			websocket = new WebSocket(wsUrl);
			
			websocket.onopen = () => {
				connectionStatus.set('connected');
				console.log('WebSocket connected');
				startPingInterval();
			};
			
			websocket.onmessage = (event) => {
				try {
					const data = JSON.parse(event.data);
					handleWebSocketMessage(data);
				} catch (error) {
					console.error('Failed to parse WebSocket message:', error);
				}
			};
			
			websocket.onerror = (error) => {
				console.error('WebSocket error:', error);
				connectionStatus.set('disconnected');
			};
			
			websocket.onclose = (event) => {
				console.log('WebSocket closed:', event.code, event.reason);
				connectionStatus.set('disconnected');
				websocket = null;
				
				// Clear ping interval
				if (pingInterval) {
					clearInterval(pingInterval);
					pingInterval = null;
				}
				
				// Attempt to reconnect if not a normal closure
				if (event.code !== 1000) {
					scheduleReconnect();
				}
			};
		} catch (error) {
			console.error('Failed to create WebSocket connection:', error);
			connectionStatus.set('disconnected');
		}
	},

	// Disconnect WebSocket
	disconnect: () => {
		if (reconnectTimeout) {
			clearTimeout(reconnectTimeout);
			reconnectTimeout = null;
		}
		
		if (pingInterval) {
			clearInterval(pingInterval);
			pingInterval = null;
		}
		
		if (websocket) {
			websocket.close(1000, 'User disconnect');
			websocket = null;
		}
		connectionStatus.set('disconnected');
	},

	// Send message through WebSocket
	send: (message: any) => {
		if (websocket && websocket.readyState === WebSocket.OPEN) {
			websocket.send(JSON.stringify(message));
		}
	},

	// Load initial webhook history
	loadHistory: async () => {
		if (!browser) return;
		
		isLoading.set(true);
		try {
			const response = await fetch('/api/webhooks');
			if (response.ok) {
				const data = await response.json();
				webhookEvents.set(data.webhooks);
			}
		} catch (error) {
			console.error('Failed to load webhook history:', error);
		} finally {
			isLoading.set(false);
		}
	},

	// Load relay targets
	loadTargets: async () => {
		if (!browser) return;
		
		try {
			const response = await fetch('/api/relay/targets');
			if (response.ok) {
				const targets = await response.json();
				relayTargets.set(targets);
			}
		} catch (error) {
			console.error('Failed to load relay targets:', error);
		}
	},

	// Add relay target
	addTarget: async (target: string, nickname?: string) => {
		if (!browser) return;
		
		try {
			const response = await fetch('/api/relay/targets', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ target, nickname })
			});
			
			if (response.ok) {
				const newTarget = await response.json();
				relayTargets.update(targets => [...targets, newTarget]);
				return newTarget;
			} else {
				const errorData = await response.json();
				throw new Error(errorData.message || 'Failed to add target');
			}
		} catch (error) {
			console.error('Failed to add relay target:', error);
			throw error;
		}
	},

	// Remove relay target
	removeTarget: async (targetId: string) => {
		if (!browser) return;
		
		try {
			const response = await fetch('/api/relay/targets', {
				method: 'DELETE',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ targetId })
			});
			
			if (response.ok) {
				relayTargets.update(targets => 
					targets.filter(target => target.id !== targetId)
				);
			}
		} catch (error) {
			console.error('Failed to remove relay target:', error);
			throw error;
		}
	}
};

/**
 * Handle incoming WebSocket messages
 */
function handleWebSocketMessage(data: any) {
	switch (data.type) {
		case 'webhook':
			webhookEvents.update(events => [data.data, ...events].slice(0, 100));
			break;
		case 'system':
			console.log('System message:', data.data.message);
			break;
		case 'pong':
			// Connection is alive
			break;
		default:
			console.log('Unknown WebSocket message type:', data.type);
	}
}

/**
 * Start ping interval to keep connection alive
 */
function startPingInterval() {
	if (pingInterval) clearInterval(pingInterval);
	
	pingInterval = setInterval(() => {
		if (websocket && websocket.readyState === WebSocket.OPEN) {
			webhookStore.send({ type: 'ping', timestamp: Date.now() });
		} else if (pingInterval) {
			clearInterval(pingInterval);
			pingInterval = null;
		}
	}, 30000) as any; // Ping every 30 seconds
}

/**
 * Schedule reconnection attempt
 */
function scheduleReconnect() {
	if (reconnectTimeout) return;
	
	reconnectTimeout = setTimeout(() => {
		reconnectTimeout = null;
		console.log('Attempting to reconnect WebSocket...');
		webhookStore.connect();
	}, 3000) as any;
}