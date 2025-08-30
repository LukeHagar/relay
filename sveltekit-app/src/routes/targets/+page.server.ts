import { prisma } from '$lib/db';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return { targets: [] };
	}

	try {
		const targets = await prisma.relayTarget.findMany({
			where: { userId: session.user.id },
			orderBy: { createdAt: 'desc' }
		});

		return { targets };
	} catch (error) {
		console.error('Error loading relay targets:', error);
		return { targets: [] };
	}
};