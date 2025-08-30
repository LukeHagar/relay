<script lang="ts">
	import { webhookStore } from '$stores/webhooks';
	import WebhookEventCard from '$components/WebhookEventCard.svelte';
	import WebSocketStatus from '$components/WebSocketStatus.svelte';
	
	interface Props {
		data: {
			session?: {
				user?: {
					id: string;
					name?: string;
					email?: string;
					image?: string;
					subdomain?: string;
					username?: string;
				};
			};
		};
	}
	
	let { data }: Props = $props();
	let user = $derived(data.session?.user);
</script>

<svelte:head>
	<title>Webhooks - Webhook Relay</title>
</svelte:head>

<div class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
	<div class="px-4 py-6 sm:px-0">
		<div class="mb-8">
			<div class="flex items-center justify-between">
				<div>
					<h1 class="text-2xl font-bold text-gray-900">Webhook Events</h1>
					<p class="mt-2 text-gray-600">
						Real-time view of all incoming webhook events
					</p>
				</div>
				<div class="flex items-center space-x-2">
					<WebSocketStatus />
				</div>
			</div>
		</div>

		<!-- Webhook Endpoint Info -->
		<div class="bg-white shadow rounded-lg mb-8">
			<div class="px-4 py-5 sm:p-6">
				<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
					Your Webhook Endpoint
				</h3>
				<div class="space-y-3">
					<div class="bg-gray-50 rounded-md p-4">
						<div class="flex items-center justify-between">
							<code class="text-sm text-gray-800">
								POST https://yourdomain.com/api/webhook/{user?.subdomain}
							</code>
											<button 
					onclick={() => navigator.clipboard.writeText(`https://yourdomain.com/api/webhook/${user?.subdomain}`)}
					class="text-xs text-primary-600 hover:text-primary-500 transition-colors"
				>
					Copy
				</button>
						</div>
					</div>
					<p class="text-sm text-gray-500">
						Configure external services to send webhooks to this endpoint. 
						All events will be logged and forwarded to your relay targets.
					</p>
				</div>
			</div>
		</div>

		<!-- Test Webhook -->
		<div class="bg-white shadow rounded-lg mb-8">
			<div class="px-4 py-5 sm:p-6">
				<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
					Test Your Webhook
				</h3>
				<button 
					onclick={async () => {
						try {
							await fetch(`/api/webhook/${user?.subdomain}`, {
								method: 'POST',
								headers: { 'Content-Type': 'application/json' },
								body: JSON.stringify({
									test: true,
									message: 'Test webhook from dashboard',
									timestamp: new Date().toISOString()
								})
							});
						} catch (error) {
							console.error('Failed to send test webhook:', error);
						}
					}}
					class="btn-primary"
				>
					Send Test Webhook
				</button>
			</div>
		</div>

		<!-- Events List -->
		<div class="bg-white shadow rounded-lg">
			<div class="px-4 py-5 sm:p-6">
				<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
					Event History ({webhookStore.totalEvents})
				</h3>
				
				{#if webhookStore.loading}
					<div class="flex justify-center py-8">
						<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
					</div>
				{:else if webhookStore.events.length > 0}
					<div class="space-y-4 max-h-96 overflow-y-auto custom-scrollbar">
						{#each webhookStore.events as event (event.id)}
							<WebhookEventCard {event} />
						{/each}
					</div>
				{:else}
					<div class="text-center py-8">
						<div class="text-gray-400 mb-2">
							<svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2 2v-5m16 0h-5.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H1" />
							</svg>
						</div>
						<h3 class="text-sm font-medium text-gray-900">No webhook events yet</h3>
						<p class="text-sm text-gray-500 mb-4">
							Send a webhook to your endpoint or use the test button above.
						</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>