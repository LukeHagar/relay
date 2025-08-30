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

// SSE Connection management
let eventSource: EventSource | null = null;

export const webhookStore = {
	// Initialize SSE connection
	connect: () => {
		if (!browser) return;
		
		connectionStatus.set('connecting');
		
		eventSource = new EventSource('/api/relay/events');
		
		eventSource.onopen = () => {
			connectionStatus.set('connected');
		};
		
		eventSource.onmessage = (event) => {
			try {
				const data = JSON.parse(event.data);
				
				if (data.type === 'webhook') {
					webhookEvents.update(events => [data.data, ...events].slice(0, 100));
				}
			} catch (error) {
				console.error('Failed to parse SSE message:', error);
			}
		};
		
		eventSource.onerror = () => {
			connectionStatus.set('disconnected');
			// Attempt to reconnect after 3 seconds
			setTimeout(() => {
				if (eventSource?.readyState === EventSource.CLOSED) {
					webhookStore.connect();
				}
			}, 3000);
		};
	},

	// Disconnect SSE
	disconnect: () => {
		if (eventSource) {
			eventSource.close();
			eventSource = null;
		}
		connectionStatus.set('disconnected');
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