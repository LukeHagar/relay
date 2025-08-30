import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { prisma } from '$db';
import { broadcastToUser, forwardToRelayTargets } from '$lib/server/relay';

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

		// Enhanced body parsing to handle all webhook types
		let body: any = null;
		let rawBody = '';
		const contentType = request.headers.get('content-type') || '';
		
		try {
			// Always get raw body first for signature verification
			rawBody = await request.text();
			
			if (contentType.includes('application/json')) {
				body = JSON.parse(rawBody);
			} else if (contentType.includes('application/x-www-form-urlencoded')) {
				const formData = new URLSearchParams(rawBody);
				body = Object.fromEntries(formData);
			} else if (contentType.includes('multipart/form-data')) {
				// For multipart, we need to re-read as FormData
				const clonedRequest = request.clone();
				const formData = await clonedRequest.formData();
				body = Object.fromEntries(formData);
			} else if (contentType.includes('application/xml') || contentType.includes('text/xml')) {
				body = rawBody; // Keep XML as string
			} else {
				body = rawBody; // Keep as raw text for other types
			}
		} catch (parseError) {
			// If parsing fails, keep raw body
			body = rawBody;
		}

		// Collect all headers (excluding sensitive ones)
		const headers: Record<string, string> = {};
		request.headers.forEach((value, key) => {
			const lowerKey = key.toLowerCase();
			if (!lowerKey.includes('authorization') && 
			    !lowerKey.includes('cookie') &&
			    !lowerKey.includes('session')) {
				headers[key] = value;
			}
		});

		// Create comprehensive webhook event record
		const webhookEvent = {
			userId: user.id,
			method: request.method,
			path: url.pathname,
			query: url.search || '',
			body: typeof body === 'string' ? body : JSON.stringify(body),
			headers: JSON.stringify(headers),
			createdAt: new Date(),
		};

		// Store in database with error handling
		let savedEvent;
		try {
			savedEvent = await prisma.webhookEvent.create({
				data: webhookEvent
			});
		} catch (dbError) {
			console.error('Database storage error:', dbError);
			// Continue processing even if DB fails
			savedEvent = { id: 'temp-' + Date.now(), ...webhookEvent };
		}

		// Prepare broadcast data
		const broadcastData = {
			id: savedEvent.id,
			type: 'webhook' as const,
			data: {
				...webhookEvent,
				body: body, // Send parsed body for frontend
				headers: headers, // Send parsed headers
				timestamp: savedEvent.createdAt.toISOString(),
				contentType
			}
		};

		// Broadcast to connected WebSocket clients
		const broadcastSuccess = await broadcastToUser(user.id, broadcastData);

		// Forward to relay targets if configured
		let forwardResults: any[] = [];
		try {
			forwardResults = await forwardToRelayTargets(user.id, {
				...broadcastData.data,
				originalUrl: url.href,
				userAgent: headers['user-agent'] || 'Unknown'
			});
		} catch (forwardError) {
			console.error('Relay forwarding error:', forwardError);
		}

		// Return comprehensive response
		return json({
			success: true,
			logged: !!savedEvent.id && !savedEvent.id.startsWith('temp-'),
			forwarded: broadcastSuccess,
			relayResults: forwardResults,
			subdomain,
			eventId: savedEvent.id,
			timestamp: new Date().toISOString()
		});

	} catch (err) {
		console.error('Webhook processing error:', err);
		
		// Still return success for webhook senders, but log the error
		return json({
			success: true,
			logged: false,
			forwarded: false,
			error: 'Internal processing error',
			subdomain,
			timestamp: new Date().toISOString()
		}, { status: 200 }); // Return 200 to prevent webhook retries
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