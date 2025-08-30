import { writable, type Writable } from 'svelte/store';

export interface WebhookEvent {
	id: string;
	userId: string;
	method: string;
	path: string;
	query: string;
	body: string;
	headers: string;
	createdAt: string;
}

export interface WebSocketState {
	connected: boolean;
	connecting: boolean;
	error: string | null;
}

class WebSocketClient {
	private ws: WebSocket | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;
	private reconnectDelay = 1000;
	private url: string;

	public events: Writable<WebhookEvent[]> = writable([]);
	public state: Writable<WebSocketState> = writable({
		connected: false,
		connecting: false,
		error: null
	});

	constructor(url: string) {
		this.url = url;
	}

	connect() {
		if (this.ws?.readyState === WebSocket.OPEN) {
			return;
		}

		this.state.update(s => ({ ...s, connecting: true, error: null }));

		try {
			this.ws = new WebSocket(this.url);

			this.ws.onopen = () => {
				this.state.update(s => ({ 
					connected: true, 
					connecting: false, 
					error: null 
				}));
				this.reconnectAttempts = 0;
				console.log('WebSocket connected');
			};

			this.ws.onmessage = (event) => {
				try {
					const data = JSON.parse(event.data);
					if (data.userId) {
						// This is a webhook event
						this.events.update(events => [data, ...events.slice(0, 99)]); // Keep last 100 events
					}
				} catch (error) {
					console.error('Failed to parse WebSocket message:', error);
				}
			};

			this.ws.onclose = () => {
				this.state.update(s => ({ 
					connected: false, 
					connecting: false 
				}));
				console.log('WebSocket disconnected');
				this.attemptReconnect();
			};

			this.ws.onerror = (error) => {
				this.state.update(s => ({ 
					connected: false, 
					connecting: false, 
					error: 'Connection failed' 
				}));
				console.error('WebSocket error:', error);
			};

		} catch (error) {
			this.state.update(s => ({ 
				connected: false, 
				connecting: false, 
				error: 'Failed to create connection' 
			}));
			console.error('Failed to create WebSocket connection:', error);
		}
	}

	private attemptReconnect() {
		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			this.state.update(s => ({ 
				...s, 
				error: 'Max reconnection attempts reached' 
			}));
			return;
		}

		this.reconnectAttempts++;
		const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

		setTimeout(() => {
			console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
			this.connect();
		}, delay);
	}

	disconnect() {
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
		this.state.update(s => ({ 
			connected: false, 
			connecting: false, 
			error: null 
		}));
	}

	send(message: any) {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(message));
		}
	}

	clearEvents() {
		this.events.set([]);
	}
}

export function createWebSocketClient(url: string) {
	return new WebSocketClient(url);
}