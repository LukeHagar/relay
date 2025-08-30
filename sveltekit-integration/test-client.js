#!/usr/bin/env node

// Comprehensive webhook testing client for the SvelteKit implementation

const testSubdomain = process.argv[2] || 'test-user';
const baseUrl = process.argv[3] || 'http://localhost:5173';

const webhookUrl = `${baseUrl}/api/webhook/${testSubdomain}`;

console.log(`Testing webhook ingestion at: ${webhookUrl}`);
console.log('='.repeat(60));

// Test cases for different webhook types
const testCases = [
	{
		name: 'JSON Webhook (GitHub-style)',
		contentType: 'application/json',
		headers: {
			'X-GitHub-Event': 'push',
			'X-GitHub-Delivery': '12345-67890',
			'User-Agent': 'GitHub-Hookshot/abc123'
		},
		body: {
			ref: 'refs/heads/main',
			before: 'abc123',
			after: 'def456',
			repository: {
				name: 'test-repo',
				full_name: 'user/test-repo',
				private: false
			},
			pusher: {
				name: 'testuser',
				email: 'test@example.com'
			},
			commits: [
				{
					id: 'def456',
					message: 'Test commit',
					author: {
						name: 'Test User',
						email: 'test@example.com'
					}
				}
			]
		}
	},
	{
		name: 'Form Data Webhook (Stripe-style)',
		contentType: 'application/x-www-form-urlencoded',
		headers: {
			'Stripe-Signature': 't=1234567890,v1=test_signature',
			'User-Agent': 'Stripe/1.0'
		},
		body: 'id=evt_test&object=event&type=payment_intent.succeeded&data[object][id]=pi_test&data[object][amount]=2000&data[object][currency]=usd'
	},
	{
		name: 'XML Webhook (PayPal-style)',
		contentType: 'application/xml',
		headers: {
			'PayPal-Auth-Algo': 'SHA256withRSA',
			'User-Agent': 'PayPal/AUHD-214.0-52392296'
		},
		body: '<?xml version="1.0" encoding="UTF-8"?><notification><timestamp>2024-01-01T12:00:00Z</timestamp><event_type>PAYMENT.CAPTURE.COMPLETED</event_type><resource><id>PAYMENT123</id><amount><currency_code>USD</currency_code><value>25.00</value></amount></resource></notification>'
	},
	{
		name: 'Plain Text Webhook',
		contentType: 'text/plain',
		headers: {
			'X-Custom-Header': 'test-value',
			'User-Agent': 'CustomWebhookSender/1.0'
		},
		body: 'Simple plain text webhook payload with some test data'
	},
	{
		name: 'Large JSON Payload',
		contentType: 'application/json',
		headers: {
			'X-Event-Type': 'bulk-update',
			'User-Agent': 'BulkProcessor/2.0'
		},
		body: {
			event: 'bulk_update',
			timestamp: new Date().toISOString(),
			data: Array.from({ length: 100 }, (_, i) => ({
				id: i + 1,
				name: `Item ${i + 1}`,
				value: Math.random() * 1000,
				tags: [`tag${i % 5}`, `category${i % 3}`],
				metadata: {
					created: new Date(Date.now() - Math.random() * 86400000).toISOString(),
					updated: new Date().toISOString()
				}
			}))
		}
	},
	{
		name: 'Webhook with Special Characters',
		contentType: 'application/json',
		headers: {
			'X-Special-Header': 'test with spaces & symbols!',
			'User-Agent': 'SpecialChar-Tester/1.0'
		},
		body: {
			message: 'Test with special characters: éñüñ 中文 🚀 💻',
			symbols: '!@#$%^&*()_+-=[]{}|;:,.<>?',
			unicode: '𝓤𝓷𝓲𝓬𝓸𝓭𝓮 𝓣𝓮𝔁𝓽',
			emoji: '🎉🔥💯✨🚀'
		}
	}
];

async function runTest(testCase) {
	console.log(`\n🧪 Testing: ${testCase.name}`);
	console.log(`   Content-Type: ${testCase.contentType}`);
	
	try {
		const startTime = Date.now();
		
		const response = await fetch(webhookUrl, {
			method: 'POST',
			headers: {
				'Content-Type': testCase.contentType,
				...testCase.headers
			},
			body: typeof testCase.body === 'string' ? testCase.body : JSON.stringify(testCase.body)
		});

		const endTime = Date.now();
		const responseTime = endTime - startTime;
		
		const result = await response.json();
		
		console.log(`   ✅ Status: ${response.status} (${responseTime}ms)`);
		console.log(`   📝 Logged: ${result.logged}`);
		console.log(`   📡 Forwarded: ${result.forwarded}`);
		console.log(`   🆔 Event ID: ${result.eventId}`);
		
		if (result.relayResults && result.relayResults.length > 0) {
			console.log(`   🔄 Relay Results: ${result.relayResults.length} targets`);
		}
		
		return { success: true, responseTime, result };
	} catch (error) {
		console.log(`   ❌ Error: ${error.message}`);
		return { success: false, error: error.message };
	}
}

async function runAllTests() {
	console.log(`🚀 Starting webhook ingestion tests...`);
	console.log(`📍 Target URL: ${webhookUrl}`);
	console.log(`📅 Started at: ${new Date().toISOString()}`);
	
	const results = [];
	let successCount = 0;
	let totalResponseTime = 0;
	
	for (const testCase of testCases) {
		const result = await runTest(testCase);
		results.push({ testCase: testCase.name, ...result });
		
		if (result.success) {
			successCount++;
			totalResponseTime += result.responseTime || 0;
		}
		
		// Small delay between tests
		await new Promise(resolve => setTimeout(resolve, 500));
	}
	
	console.log('\n' + '='.repeat(60));
	console.log('📊 TEST SUMMARY');
	console.log('='.repeat(60));
	console.log(`✅ Successful: ${successCount}/${testCases.length}`);
	console.log(`⏱️  Average Response Time: ${Math.round(totalResponseTime / successCount)}ms`);
	console.log(`📅 Completed at: ${new Date().toISOString()}`);
	
	if (successCount === testCases.length) {
		console.log('\n🎉 All tests passed! Webhook ingestion is working correctly.');
	} else {
		console.log('\n⚠️  Some tests failed. Check the logs above for details.');
	}
	
	return results;
}

// Run tests if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
	runAllTests().catch(console.error);
}

export { runAllTests, runTest, testCases };