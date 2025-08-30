import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import type { RequestHandler } from './$types';

export const PUT: RequestHandler = async ({ params, request, locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		const { target, nickname } = await request.json();

		if (!target) {
			return json({ error: 'Target URL is required' }, { status: 400 });
		}

		// Validate URL
		try {
			new URL(target);
		} catch {
			return json({ error: 'Invalid URL format' }, { status: 400 });
		}

		const updatedTarget = await prisma.relayTarget.update({
			where: {
				id: params.id,
				userId: session.user.id
			},
			data: {
				target,
				nickname: nickname || null
			}
		});

		return json(updatedTarget);
	} catch (error) {
		console.error('Error updating relay target:', error);
		return json({ error: 'Failed to update target' }, { status: 500 });
	}
};

export const DELETE: RequestHandler = async ({ params, locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		await prisma.relayTarget.delete({
			where: {
				id: params.id,
				userId: session.user.id
			}
		});

		return json({ success: true });
	} catch (error) {
		console.error('Error deleting relay target:', error);
		return json({ error: 'Failed to delete target' }, { status: 500 });
	}
};