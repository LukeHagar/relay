import { prisma } from '$db';

// Store for Server-Sent Events connections
const sseConnections = new Map<string, Set<ReadableStreamDefaultController>>();

export interface WebhookEvent {
	id: string;
	type: 'webhook' | 'system';
	data: any;
}

/**
 * Add an SSE connection for a user
 */
export function addSSEConnection(userId: string, controller: ReadableStreamDefaultController) {
	if (!sseConnections.has(userId)) {
		sseConnections.set(userId, new Set());
	}
	sseConnections.get(userId)!.add(controller);
	
	// Send initial connection message
	sendSSEMessage(controller, {
		id: crypto.randomUUID(),
		type: 'system',
		data: { message: 'Connected to webhook relay', timestamp: new Date().toISOString() }
	});
}

/**
 * Remove an SSE connection for a user
 */
export function removeSSEConnection(userId: string, controller: ReadableStreamDefaultController) {
	const userConnections = sseConnections.get(userId);
	if (userConnections) {
		userConnections.delete(controller);
		if (userConnections.size === 0) {
			sseConnections.delete(userId);
		}
	}
}

/**
 * Send message to a specific SSE connection
 */
function sendSSEMessage(controller: ReadableStreamDefaultController, event: WebhookEvent) {
	try {
		const message = `data: ${JSON.stringify(event)}\n\n`;
		controller.enqueue(new TextEncoder().encode(message));
	} catch (error) {
		console.error('Failed to send SSE message:', error);
	}
}

/**
 * Broadcast event to all connections for a specific user
 */
export async function broadcastToUser(userId: string, event: WebhookEvent): Promise<boolean> {
	const userConnections = sseConnections.get(userId);
	
	if (!userConnections || userConnections.size === 0) {
		return false;
	}

	let successCount = 0;
	const totalConnections = userConnections.size;

	userConnections.forEach(controller => {
		try {
			sendSSEMessage(controller, event);
			successCount++;
		} catch (error) {
			console.error('Failed to broadcast to connection:', error);
			// Remove failed connection
			userConnections.delete(controller);
		}
	});

	return successCount > 0;
}

/**
 * Get recent webhook events for a user
 */
export async function getRecentWebhooks(userId: string, limit = 50) {
	return await prisma.webhookEvent.findMany({
		where: { userId },
		orderBy: { createdAt: 'desc' },
		take: limit,
		select: {
			id: true,
			method: true,
			path: true,
			query: true,
			body: true,
			headers: true,
			createdAt: true
		}
	});
}

/**
 * Get relay targets for a user
 */
export async function getRelayTargets(userId: string) {
	return await prisma.relayTarget.findMany({
		where: { userId, active: true },
		orderBy: { createdAt: 'desc' }
	});
}

/**
 * Create a new relay target
 */
export async function createRelayTarget(userId: string, target: string, nickname?: string) {
	return await prisma.relayTarget.create({
		data: {
			userId,
			target,
			nickname,
			active: true
		}
	});
}

/**
 * Forward webhook to relay targets
 */
export async function forwardToRelayTargets(userId: string, webhookData: any) {
	const targets = await getRelayTargets(userId);
	
	const forwardPromises = targets.map(async (target) => {
		try {
			const response = await fetch(target.target, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'X-Forwarded-By': 'webhook-relay-sveltekit'
				},
				body: JSON.stringify(webhookData)
			});
			
			return {
				targetId: target.id,
				success: response.ok,
				status: response.status
			};
		} catch (error) {
			return {
				targetId: target.id,
				success: false,
				error: error instanceof Error ? error.message : 'Unknown error'
			};
		}
	});

	return await Promise.all(forwardPromises);
}