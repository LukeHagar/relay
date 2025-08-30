import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { prisma } from '$db';
import { broadcastToUser } from '$lib/server/relay';

export const POST: RequestHandler = async ({ request, params, url }) => {
	const { subdomain } = params;
	
	if (!subdomain) {
		throw error(400, 'Missing subdomain');
	}

	try {
		// Find user by subdomain
		const user = await prisma.user.findUnique({
			where: { subdomain }
		});

		if (!user) {
			throw error(404, 'Invalid subdomain');
		}

		// Parse request body
		let body: any = null;
		const contentType = request.headers.get('content-type');
		
		if (contentType?.includes('application/json')) {
			body = await request.json();
		} else if (contentType?.includes('application/x-www-form-urlencoded')) {
			const formData = await request.formData();
			body = Object.fromEntries(formData);
		} else {
			body = await request.text();
		}

		// Collect headers (excluding sensitive ones)
		const headers: Record<string, string> = {};
		request.headers.forEach((value, key) => {
			if (!key.toLowerCase().includes('authorization') && 
			    !key.toLowerCase().includes('cookie')) {
				headers[key] = value;
			}
		});

		// Create webhook event record
		const webhookEvent = {
			userId: user.id,
			method: request.method,
			path: url.pathname,
			query: url.search,
			body: JSON.stringify(body),
			headers: JSON.stringify(headers),
			createdAt: new Date(),
		};

		// Store in database
		const savedEvent = await prisma.webhookEvent.create({
			data: webhookEvent
		});

		// Broadcast to connected clients via SSE
		const broadcastSuccess = await broadcastToUser(user.id, {
			id: savedEvent.id,
			type: 'webhook',
			data: {
				...webhookEvent,
				timestamp: savedEvent.createdAt.toISOString()
			}
		});

		return json({
			success: true,
			logged: true,
			forwarded: broadcastSuccess,
			subdomain,
			eventId: savedEvent.id
		});

	} catch (err) {
		console.error('Webhook processing error:', err);
		throw error(500, 'Failed to process webhook');
	}
};

// Handle other HTTP methods
export const GET: RequestHandler = async ({ params }) => {
	return json({ 
		message: `Webhook endpoint for ${params.subdomain}`,
		methods: ['POST'],
		timestamp: new Date().toISOString()
	});
};

export const PUT = POST;
export const PATCH = POST;
export const DELETE = POST;