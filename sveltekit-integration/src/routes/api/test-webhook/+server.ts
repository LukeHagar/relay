import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, locals }) => {
	const session = await locals.auth();
	
	if (!session?.user?.subdomain) {
		return json({ error: 'Authentication required' }, { status: 401 });
	}

	const subdomain = session.user.subdomain;
	
	try {
		// Test different webhook payload types
		const testPayloads = [
			{
				type: 'json',
				contentType: 'application/json',
				payload: {
					test: true,
					message: 'Test JSON webhook',
					timestamp: new Date().toISOString(),
					data: {
						nested: {
							value: 'test'
						},
						array: [1, 2, 3]
					}
				}
			},
			{
				type: 'form',
				contentType: 'application/x-www-form-urlencoded',
				payload: 'test=true&message=Test+form+webhook&timestamp=' + encodeURIComponent(new Date().toISOString())
			},
			{
				type: 'text',
				contentType: 'text/plain',
				payload: 'Test plain text webhook payload'
			}
		];

		const results = [];

		for (const test of testPayloads) {
			try {
				const response = await fetch(`${request.url.origin}/api/webhook/${subdomain}`, {
					method: 'POST',
					headers: {
						'Content-Type': test.contentType,
						'User-Agent': 'WebhookRelay-Test/1.0',
						'X-Test-Type': test.type
					},
					body: typeof test.payload === 'string' ? test.payload : JSON.stringify(test.payload)
				});

				const result = await response.json();
				results.push({
					type: test.type,
					success: response.ok,
					result
				});
			} catch (error) {
				results.push({
					type: test.type,
					success: false,
					error: error instanceof Error ? error.message : 'Unknown error'
				});
			}
		}

		return json({
			message: 'Webhook ingestion tests completed',
			subdomain,
			results,
			timestamp: new Date().toISOString()
		});

	} catch (error) {
		return json({
			error: 'Test failed',
			details: error instanceof Error ? error.message : 'Unknown error'
		}, { status: 500 });
	}
};