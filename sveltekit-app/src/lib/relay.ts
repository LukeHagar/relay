import { prisma } from '$lib/db';

export interface RelayResult {
	success: boolean;
	target: string;
	statusCode?: number;
	error?: string;
	responseTime?: number;
}

export async function relayWebhookToTargets(
	userId: string, 
	webhookData: {
		method: string;
		path: string;
		query: string;
		body: any;
		headers: Record<string, string>;
	}
): Promise<RelayResult[]> {
	try {
		// Get all active relay targets for the user
		const targets = await prisma.relayTarget.findMany({
			where: {
				userId,
				active: true
			}
		});

		if (targets.length === 0) {
			return [];
		}

		// Forward webhook to all active targets
		const results = await Promise.allSettled(
			targets.map(async (target) => {
				const startTime = Date.now();
				
				try {
					// Prepare headers for forwarding
					const forwardHeaders: Record<string, string> = {
						'Content-Type': 'application/json',
						'User-Agent': 'WebhookRelay/1.0',
						'X-Webhook-Relay-Source': 'webhook-relay',
						'X-Webhook-Relay-Target': target.nickname || target.id,
						'X-Webhook-Relay-Original-Path': webhookData.path,
						'X-Webhook-Relay-Original-Method': webhookData.method,
						'X-Webhook-Relay-Original-Query': webhookData.query,
						...webhookData.headers
					};

					// Remove headers that shouldn't be forwarded
					delete forwardHeaders['host'];
					delete forwardHeaders['authorization'];
					delete forwardHeaders['cookie'];
					delete forwardHeaders['x-webhook-relay-subdomain'];
					delete forwardHeaders['x-webhook-relay-user-id'];

					// Forward the webhook
					const response = await fetch(target.target, {
						method: webhookData.method,
						headers: forwardHeaders,
						body: webhookData.method !== 'GET' ? JSON.stringify(webhookData.body) : undefined,
						signal: AbortSignal.timeout(30000) // 30 second timeout
					});

					const responseTime = Date.now() - startTime;

					return {
						success: response.ok,
						target: target.target,
						statusCode: response.status,
						responseTime
					} as RelayResult;

				} catch (error) {
					const responseTime = Date.now() - startTime;
					
					return {
						success: false,
						target: target.target,
						error: error instanceof Error ? error.message : 'Unknown error',
						responseTime
					} as RelayResult;
				}
			})
		);

		// Process results
		return results.map((result, index) => {
			if (result.status === 'fulfilled') {
				return result.value;
			} else {
				return {
					success: false,
					target: targets[index]?.target || 'unknown',
					error: result.reason?.message || 'Unknown error'
				} as RelayResult;
			}
		});

	} catch (error) {
		console.error('Error relaying webhook to targets:', error);
		return [];
	}
}

// Optional: Store relay results for analytics
export async function storeRelayResults(
	webhookEventId: string, 
	results: RelayResult[]
): Promise<void> {
	try {
		// You could create a new table for relay results if needed
		// For now, we'll just log them
		console.log('Relay results for webhook:', webhookEventId, results);
		
		// Example of storing results (if you add a RelayResult table):
		// await prisma.relayResult.createMany({
		//   data: results.map(result => ({
		//     webhookEventId,
		//     target: result.target,
		//     success: result.success,
		//     statusCode: result.statusCode,
		//     error: result.error,
		//     responseTime: result.responseTime
		//   }))
		// });
	} catch (error) {
		console.error('Error storing relay results:', error);
	}
}