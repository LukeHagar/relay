import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import type { RequestHandler } from './$types';

export const PUT: RequestHandler = async ({ request, locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		const { subdomain } = await request.json();

		if (!subdomain) {
			return json({ error: 'Subdomain is required' }, { status: 400 });
		}

		// Validate subdomain format
		const subdomainRegex = /^[a-z0-9-]+$/;
		if (!subdomainRegex.test(subdomain)) {
			return json({ error: 'Subdomain can only contain lowercase letters, numbers, and hyphens' }, { status: 400 });
		}

		if (subdomain.length < 3 || subdomain.length > 63) {
			return json({ error: 'Subdomain must be between 3 and 63 characters' }, { status: 400 });
		}

		// Check if subdomain is already taken
		const existingUser = await prisma.user.findUnique({
			where: { subdomain }
		});

		if (existingUser && existingUser.id !== session.user.id) {
			return json({ error: 'Subdomain is already taken' }, { status: 400 });
		}

		// Update user subdomain
		const updatedUser = await prisma.user.update({
			where: { id: session.user.id },
			data: { subdomain },
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

		return json(updatedUser);
	} catch (error) {
		console.error('Error updating subdomain:', error);
		return json({ error: 'Failed to update subdomain' }, { status: 500 });
	}
};