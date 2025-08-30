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

// Svelte 5 runes for reactive state
let webhookEvents = $state<WebhookEvent[]>([]);
let relayTargets = $state<RelayTarget[]>([]);
let connectionStatus = $state<'connected' | 'disconnected' | 'connecting'>('disconnected');
let isLoading = $state(false);

// Derived state using Svelte 5 runes
export const webhookStore = {
	// Reactive getters
	get events() { return webhookEvents; },
	get targets() { return relayTargets; },
	get status() { return connectionStatus; },
	get loading() { return isLoading; },
	
	// Derived values
	get activeTargets() { 
		return relayTargets.filter(target => target.active); 
	},
	get recentEvents() { 
		return webhookEvents.slice(0, 10); 
	},
	get totalEvents() {
		return webhookEvents.length;
	},
	
	// State setters
	setEvents: (events: WebhookEvent[]) => { webhookEvents = events; },
	addEvent: (event: WebhookEvent) => { 
		webhookEvents = [event, ...webhookEvents].slice(0, 100); 
	},
	setTargets: (targets: RelayTarget[]) => { relayTargets = targets; },
	addTarget: (target: RelayTarget) => { 
		relayTargets = [...relayTargets, target]; 
	},
	removeTarget: (targetId: string) => {
		relayTargets = relayTargets.filter(t => t.id !== targetId);
	},
	setStatus: (status: typeof connectionStatus) => { connectionStatus = status; },
	setLoading: (loading: boolean) => { isLoading = loading; },

// WebSocket Connection management
let websocket: WebSocket | null = null;
let reconnectTimeout: number | null = null;
let pingInterval: number | null = null;

export const webhookStore = {
	// Initialize WebSocket connection
	connect: async () => {
		if (!browser) return;
		
		connectionStatus = 'connecting';
		
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
			connectionStatus = 'disconnected';
			return;
		}
		
		const wsUrl = `${protocol}//${window.location.hostname}:${wsPort}?token=${sessionToken}`;
		
		try {
			websocket = new WebSocket(wsUrl);
			
			websocket.onopen = () => {
				connectionStatus = 'connected';
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
				connectionStatus = 'disconnected';
			};
			
			websocket.onclose = (event) => {
				console.log('WebSocket closed:', event.code, event.reason);
				connectionStatus = 'disconnected';
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
			connectionStatus = 'disconnected';
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
		connectionStatus = 'disconnected';
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
		
		isLoading = true;
		try {
			const response = await fetch('/api/webhooks');
			if (response.ok) {
				const data = await response.json();
				webhookEvents = data.webhooks;
			}
		} catch (error) {
			console.error('Failed to load webhook history:', error);
		} finally {
			isLoading = false;
		}
	},

	// Load relay targets
	loadTargets: async () => {
		if (!browser) return;
		
		try {
			const response = await fetch('/api/relay/targets');
			if (response.ok) {
				const targets = await response.json();
				relayTargets = targets;
			}
		} catch (error) {
			console.error('Failed to load relay targets:', error);
		}
	},

	// Add relay target
	addTargetRemote: async (target: string, nickname?: string) => {
		if (!browser) return;
		
		try {
			const response = await fetch('/api/relay/targets', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ target, nickname })
			});
			
			if (response.ok) {
				const newTarget = await response.json();
				relayTargets = [...relayTargets, newTarget];
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
	removeTargetRemote: async (targetId: string) => {
		if (!browser) return;
		
		try {
			const response = await fetch('/api/relay/targets', {
				method: 'DELETE',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ targetId })
			});
			
			if (response.ok) {
				relayTargets = relayTargets.filter(target => target.id !== targetId);
			}
		} catch (error) {
			console.error('Failed to remove relay target:', error);
			throw error;
		}
	}
};

/**
 * Handle incoming WebSocket messages using Svelte 5 runes
 */
function handleWebSocketMessage(data: any) {
	switch (data.type) {
		case 'webhook':
			webhookEvents = [data.data, ...webhookEvents].slice(0, 100);
			// Optional: Show notification for new webhooks (can be disabled for high volume)
			if (webhookEvents.length <= 10) {
				import('$stores/notifications').then(({ notificationStore }) => {
					notificationStore.info(
						'New Webhook', 
						`${data.data.method} ${data.data.path}`,
						3000
					);
				});
			}
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