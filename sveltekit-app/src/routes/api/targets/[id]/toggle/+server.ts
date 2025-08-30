import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import type { RequestHandler } from './$types';

export const PUT: RequestHandler = async ({ params, request, locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		const { active } = await request.json();

		const updatedTarget = await prisma.relayTarget.update({
			where: {
				id: params.id,
				userId: session.user.id
			},
			data: {
				active: Boolean(active)
			}
		});

		return json(updatedTarget);
	} catch (error) {
		console.error('Error toggling relay target:', error);
		return json({ error: 'Failed to toggle target' }, { status: 500 });
	}
};