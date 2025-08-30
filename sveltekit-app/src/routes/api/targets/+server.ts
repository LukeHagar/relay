import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, locals }) => {
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

		const newTarget = await prisma.relayTarget.create({
			data: {
				userId: session.user.id,
				target,
				nickname: nickname || null,
				active: true
			}
		});

		return json(newTarget);
	} catch (error) {
		console.error('Error creating relay target:', error);
		return json({ error: 'Failed to create target' }, { status: 500 });
	}
};