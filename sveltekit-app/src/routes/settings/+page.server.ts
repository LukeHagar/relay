import { prisma } from '$lib/db';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return { user: null };
	}

	try {
		const user = await prisma.user.findUnique({
			where: { id: session.user.id },
			select: {
				id: true,
				name: true,
				email: true,
				subdomain: true,
				username: true,
				image: true,
				createdAt: true
			}
		});

		return { user };
	} catch (error) {
		console.error('Error loading user settings:', error);
		return { user: null };
	}
};