import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, locals }) => {
	const session = await locals.getSession();
	
	if (!session?.user?.id) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		const { subdomain, method = 'POST', path = '/test', body = { test: true } } = await request.json();

		if (!subdomain) {
			return json({ error: 'Subdomain is required' }, { status: 400 });
		}

		// Verify the subdomain belongs to the user
		const user = await prisma.user.findUnique({
			where: { 
				id: session.user.id,
				subdomain: subdomain 
			}
		});

		if (!user) {
			return json({ error: 'Invalid subdomain for user' }, { status: 400 });
		}

		// Create a test webhook event
		const webhookEvent = {
			userId: user.id,
			method: method,
			path: path,
			query: '',
			body: JSON.stringify(body),
			headers: JSON.stringify({
				'Content-Type': 'application/json',
				'User-Agent': 'Test-Webhook/1.0',
				'X-Test-Webhook': 'true'
			}),
			createdAt: new Date(),
		};

		// Store in database
		const storedEvent = await prisma.webhookEvent.create({
			data: webhookEvent,
		});

		return json({
			success: true,
			message: 'Test webhook created successfully',
			eventId: storedEvent.id,
			webhookUrl: `https://${subdomain}.yourdomain.com${path}`,
			timestamp: storedEvent.createdAt
		});

	} catch (error) {
		console.error('Error creating test webhook:', error);
		return json({ error: 'Failed to create test webhook' }, { status: 500 });
	}
};