import { prisma } from '$lib/db';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, url }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return { events: [], total: 0 };
	}

	const page = parseInt(url.searchParams.get('page') || '1');
	const limit = 20;
	const skip = (page - 1) * limit;

	try {
		const [events, total] = await Promise.all([
			prisma.webhookEvent.findMany({
				where: { userId: session.user.id },
				orderBy: { createdAt: 'desc' },
				skip,
				take: limit,
				include: {
					user: {
						select: {
							name: true,
							subdomain: true
						}
					}
				}
			}),
			prisma.webhookEvent.count({
				where: { userId: session.user.id }
			})
		]);

		return {
			events,
			total,
			page,
			totalPages: Math.ceil(total / limit)
		};
	} catch (error) {
		console.error('Error loading webhook events:', error);
		return { events: [], total: 0, page: 1, totalPages: 0 };
	}
};