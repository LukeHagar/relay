import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		// Get total events
		const totalEvents = await prisma.webhookEvent.count({
			where: { userId: session.user.id }
		});

		// Get events from last hour
		const oneHourAgo = new Date(Date.now() - 60 * 60 * 1000);
		const recentEvents = await prisma.webhookEvent.count({
			where: {
				userId: session.user.id,
				createdAt: { gte: oneHourAgo }
			}
		});

		// Get active relay targets
		const activeTargets = await prisma.relayTarget.count({
			where: {
				userId: session.user.id,
				active: true
			}
		});

		// Calculate success rate (simplified - you might want to track actual success/failure)
		const successRate = totalEvents > 0 ? 95 : 0; // Placeholder

		return json({
			totalEvents,
			recentEvents,
			activeConnections: activeTargets,
			successRate
		});
	} catch (error) {
		console.error('Error fetching webhook stats:', error);
		return json({ error: 'Failed to fetch stats' }, { status: 500 });
	}
};