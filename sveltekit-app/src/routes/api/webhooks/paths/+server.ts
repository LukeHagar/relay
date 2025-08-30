import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals, url }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		const timeRange = url.searchParams.get('range') || '24h';
		let timeFilter = {};
		
		// Calculate time range
		const now = new Date();
		switch (timeRange) {
			case '1h':
				timeFilter = { gte: new Date(now.getTime() - 60 * 60 * 1000) };
				break;
			case '24h':
				timeFilter = { gte: new Date(now.getTime() - 24 * 60 * 60 * 1000) };
				break;
			case '7d':
				timeFilter = { gte: new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000) };
				break;
			case '30d':
				timeFilter = { gte: new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000) };
				break;
			default:
				timeFilter = { gte: new Date(now.getTime() - 24 * 60 * 60 * 1000) };
		}

		// Get path statistics
		const pathStats = await prisma.webhookEvent.groupBy({
			by: ['path'],
			where: {
				userId: session.user.id,
				createdAt: timeFilter
			},
			_count: {
				path: true
			},
			orderBy: {
				_count: {
					path: 'desc'
				}
			},
			take: 10
		});

		// Get method distribution
		const methodStats = await prisma.webhookEvent.groupBy({
			by: ['method'],
			where: {
				userId: session.user.id,
				createdAt: timeFilter
			},
			_count: {
				method: true
			},
			orderBy: {
				_count: {
					method: 'desc'
				}
			}
		});

		// Get recent paths (last 10 unique paths)
		const recentPaths = await prisma.webhookEvent.findMany({
			where: {
				userId: session.user.id,
				createdAt: timeFilter
			},
			select: {
				path: true,
				createdAt: true
			},
			orderBy: {
				createdAt: 'desc'
			},
			take: 100
		});

		// Get unique paths from recent events
		const uniqueRecentPaths = [...new Set(recentPaths.map(e => e.path))].slice(0, 10);

		return json({
			pathStats: pathStats.map(stat => ({
				path: stat.path,
				count: stat._count.path
			})),
			methodStats: methodStats.map(stat => ({
				method: stat.method,
				count: stat._count.method
			})),
			recentPaths: uniqueRecentPaths,
			timeRange
		});

	} catch (error) {
		console.error('Error fetching webhook path stats:', error);
		return json({ error: 'Failed to fetch path statistics' }, { status: 500 });
	}
};