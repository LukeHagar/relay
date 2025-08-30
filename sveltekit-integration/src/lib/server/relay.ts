import { prisma } from '$db';

export interface WebhookEvent {
	id: string;
	type: 'webhook' | 'system';
	data: any;
}

// Re-export from websocket server for compatibility
export { 
	broadcastToUser,
	getStats as getConnectionStats
} from './websocket-server';

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