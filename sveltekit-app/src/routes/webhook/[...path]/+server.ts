import { json } from '@sveltejs/kit';
import { prisma } from '$lib/db';
import { relayWebhookToTargets, storeRelayResults } from '$lib/relay';
import type { RequestHandler } from './$types';

// Store connected WebSocket clients for real-time updates
const clients: Map<string, any[]> = new Map();

export const GET: RequestHandler = async ({ request, params, url }) => {
	return handleWebhook(request, params, url);
};

export const POST: RequestHandler = async ({ request, params, url }) => {
	return handleWebhook(request, params, url);
};

export const PUT: RequestHandler = async ({ request, params, url }) => {
	return handleWebhook(request, params, url);
};

export const DELETE: RequestHandler = async ({ request, params, url }) => {
	return handleWebhook(request, params, url);
};

export const PATCH: RequestHandler = async ({ request, params, url }) => {
	return handleWebhook(request, params, url);
};

async function handleWebhook(request: Request, params: any, url: URL) {
	try {
		// Extract subdomain from hostname
		const hostname = request.headers.get('host') || '';
		const urlParts = hostname.split('.');
		let subdomain = '';
		
		if (urlParts.length > 1) {
			subdomain = urlParts[0];
		}

		if (!subdomain) {
			return json({ error: 'Missing Subdomain' }, { status: 400 });
		}

		// Extract and validate webhook path
		const webhookPath = params.path || '';
		const fullPath = url.pathname;
		
		// Validate path format (optional: add path restrictions)
		if (webhookPath.length > 500) {
			return json({ error: 'Webhook path too long' }, { status: 400 });
		}

		// Find user by subdomain
		const user = await prisma.user.findUnique({
			where: { subdomain },
		});

		if (!user) {
			return json({ error: 'Invalid Subdomain' }, { status: 404 });
		}

		// Get request body
		let body: any = null;
		const contentType = request.headers.get('content-type');
		
		if (contentType?.includes('application/json')) {
			try {
				body = await request.json();
			} catch {
				body = null;
			}
		} else if (contentType?.includes('application/x-www-form-urlencoded')) {
			const formData = await request.formData();
			body = Object.fromEntries(formData);
		} else {
			// Try to get raw body as text
			try {
				body = await request.text();
			} catch {
				body = null;
			}
		}

		// Get and process headers (excluding sensitive ones)
		const headers: Record<string, string> = {};
		const headerEntries = request.headers.entries();
		
		for (const [key, value] of headerEntries) {
			const lowerKey = key.toLowerCase();
			// Exclude sensitive headers and add webhook-specific headers
			if (!['authorization', 'cookie', 'x-forwarded-for', 'x-real-ip'].includes(lowerKey)) {
				headers[key] = value;
			}
		}

		// Add webhook-specific headers for tracking
		headers['X-Webhook-Relay-Subdomain'] = subdomain;
		headers['X-Webhook-Relay-Path'] = webhookPath;
		headers['X-Webhook-Relay-Timestamp'] = new Date().toISOString();
		headers['X-Webhook-Relay-User-Id'] = user.id;

		// Build webhook event with enhanced path handling
		const webhookEvent = {
			userId: user.id,
			method: request.method,
			path: webhookPath, // Store the actual webhook path, not full URL path
			query: url.search,
			body: JSON.stringify(body),
			headers: JSON.stringify(headers),
			createdAt: new Date(),
		};

		// Store in database
		const storedEvent = await prisma.webhookEvent.create({
			data: webhookEvent,
		});

		// Relay to configured targets (async, don't wait for completion)
		const relayResults = await relayWebhookToTargets(user.id, {
			method: request.method,
			path: webhookPath, // Use the actual webhook path
			query: url.search,
			body: body,
			headers: headers
		});

		// Store relay results for analytics
		storeRelayResults(storedEvent.id, relayResults);

		// Broadcast to WebSocket clients
		let messageSent = false;
		if (clients.has(subdomain)) {
			try {
				const userClients = clients.get(subdomain) || [];
				userClients.forEach((client) => {
					if (client.readyState === 1) { // WebSocket.OPEN
						client.send(JSON.stringify({
							...storedEvent,
							user: {
								name: user.name,
								subdomain: user.subdomain
							},
							relayResults: relayResults
						}));
					}
				});
				messageSent = true;
			} catch (error) {
				console.error('Error broadcasting to WebSocket clients:', error);
				messageSent = false;
			}
		}

		// Return success response
		return json({
			success: true,
			logged: true,
			forwarded: messageSent,
			subdomain,
			eventId: storedEvent.id,
			timestamp: storedEvent.createdAt
		}, { status: 200 });

	} catch (error) {
		console.error('Error handling webhook:', error);
		return json({ error: 'Internal Server Error' }, { status: 500 });
	}
}

// Export the clients map for WebSocket handler
export { clients };