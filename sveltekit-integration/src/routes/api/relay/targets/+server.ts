import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { prisma } from '$db';
import { createRelayTarget, getRelayTargets } from '$lib/server/relay';
import { z } from 'zod';

const createTargetSchema = z.object({
	target: z.string().url('Invalid URL format'),
	nickname: z.string().optional()
});

export const GET: RequestHandler = async (event) => {
	const session = await event.locals.auth();
	
	if (!session?.user?.id) {
		error(401, 'Unauthorized');
	}

	const targets = await getRelayTargets(session.user.id);
	return json(targets);
};

export const POST: RequestHandler = async (event) => {
	const session = await event.locals.auth();
	
	if (!session?.user?.id) {
		error(401, 'Unauthorized');
	}

	try {
		const body = await event.request.json();
		const { target, nickname } = createTargetSchema.parse(body);

		const newTarget = await createRelayTarget(session.user.id, target, nickname);
		
		return json(newTarget, { status: 201 });
	} catch (err) {
		if (err instanceof z.ZodError) {
			error(400, err.errors[0].message);
		}
		error(500, 'Failed to create relay target');
	}
};

export const DELETE: RequestHandler = async (event) => {
	const session = await event.locals.auth();
	
	if (!session?.user?.id) {
		error(401, 'Unauthorized');
	}

	try {
		const { targetId } = await event.request.json();
		
		const target = await prisma.relayTarget.findFirst({
			where: {
				id: targetId,
				userId: session.user.id
			}
		});

		if (!target) {
			error(404, 'Relay target not found');
		}

		await prisma.relayTarget.update({
			where: { id: targetId },
			data: { active: false }
		});

		return json({ success: true });
	} catch (err) {
		error(500, 'Failed to delete relay target');
	}
};