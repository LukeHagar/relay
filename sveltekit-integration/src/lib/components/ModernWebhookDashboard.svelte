<script lang="ts">
	import { webhookStore } from '$stores/webhooks';
	import { notificationStore } from '$stores/notifications';
	import WebSocketStatus from './WebSocketStatus.svelte';
	import WebhookEventCard from './WebhookEventCard.svelte';
	import LoadingSpinner from './LoadingSpinner.svelte';
	import { onMount } from 'svelte';

	interface Props {
		user?: {
			subdomain?: string;
			name?: string;
			image?: string;
		};
	}

	let { user }: Props = $props();

	// Svelte 5 reactive state
	let searchQuery = $state('');
	let selectedMethod = $state('all');
	let autoRefresh = $state(true);

	// Derived filtered events using Svelte 5 runes
	let filteredEvents = $derived.by(() => {
		let events = webhookStore.events;

		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			events = events.filter(event => 
				event.path.toLowerCase().includes(query) ||
				event.method.toLowerCase().includes(query) ||
				JSON.stringify(event.body).toLowerCase().includes(query)
			);
		}

		if (selectedMethod !== 'all') {
			events = events.filter(event => 
				event.method.toLowerCase() === selectedMethod.toLowerCase()
			);
		}

		return events;
	});

	// Available HTTP methods for filtering
	let availableMethods = $derived.by(() => {
		const methods = new Set(webhookStore.events.map(e => e.method));
		return ['all', ...Array.from(methods).sort()];
	});

	// Auto-refresh effect
	$effect(() => {
		if (autoRefresh) {
			const interval = setInterval(() => {
				webhookStore.loadHistory();
			}, 30000); // Refresh every 30 seconds

			return () => clearInterval(interval);
		}
	});

	async function sendTestWebhook() {
		if (!user?.subdomain) return;

		try {
			const response = await fetch(`/api/webhook/${user.subdomain}`, {
				method: 'POST',
				headers: { 
					'Content-Type': 'application/json',
					'X-Test-Source': 'dashboard'
				},
				body: JSON.stringify({
					test: true,
					message: 'Test webhook from modern dashboard',
					timestamp: new Date().toISOString(),
					features: ['svelte5', 'sveltekit2', 'tailwind4'],
					metadata: {
						userAgent: navigator.userAgent,
						screenResolution: `${screen.width}x${screen.height}`,
						timezone: Intl.DateTimeFormat().resolvedOptions().timeZone
					}
				})
			});

			if (response.ok) {
				notificationStore.success('Test Sent', 'Test webhook sent successfully');
			} else {
				throw new Error('Failed to send test webhook');
			}
		} catch (error) {
			console.error('Failed to send test webhook:', error);
			notificationStore.error('Test Failed', 'Could not send test webhook');
		}
	}

	function copyWebhookUrl() {
		if (!user?.subdomain) return;
		
		const url = `https://yourdomain.com/api/webhook/${user.subdomain}`;
		navigator.clipboard.writeText(url);
		notificationStore.success('Copied!', 'Webhook URL copied to clipboard');
	}

	onMount(() => {
		// Show welcome notification
		if (user?.name) {
			notificationStore.info(
				`Welcome back, ${user.name}!`,
				'Your webhook relay is ready to receive events'
			);
		}
	});
</script>

<div class="space-y-6">
	<!-- Header with actions -->
	<div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
		<div>
			<h2 class="text-2xl font-bold text-gray-900">Webhook Dashboard</h2>
			<p class="text-gray-600">Real-time monitoring with Svelte 5 & Tailwind 4</p>
		</div>
		<div class="flex items-center space-x-3">
			<WebSocketStatus />
			<button onclick={sendTestWebhook} class="btn-primary">
				Send Test
			</button>
		</div>
	</div>

	<!-- Webhook URL Card -->
	<div class="bg-gradient-to-r from-primary-50 to-primary-100 rounded-xl p-6 border border-primary-200">
		<div class="flex items-start justify-between">
			<div class="flex-1">
				<h3 class="text-lg font-semibold text-primary-900 mb-2">Your Webhook Endpoint</h3>
				<div class="bg-white/70 backdrop-blur rounded-lg p-4 font-mono text-sm">
					<code class="text-primary-800">
						POST https://yourdomain.com/api/webhook/{user?.subdomain}
					</code>
				</div>
				<p class="text-primary-700 text-sm mt-2">
					Configure external services to send webhooks to this endpoint
				</p>
			</div>
			<button 
				onclick={copyWebhookUrl}
				class="ml-4 p-2 text-primary-600 hover:text-primary-700 hover:bg-primary-200 rounded-lg transition-colors"
				title="Copy webhook URL"
			>
				<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
				</svg>
			</button>
		</div>
	</div>

	<!-- Filters and Controls -->
	<div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
		<div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
			<div class="flex flex-col sm:flex-row gap-4">
				<!-- Search -->
				<div class="relative">
					<input
						bind:value={searchQuery}
						type="text"
						placeholder="Search events..."
						class="pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-primary-500 sm:text-sm"
					/>
					<div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
						<svg class="h-5 w-5 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
						</svg>
					</div>
				</div>

				<!-- Method Filter -->
				<select 
					bind:value={selectedMethod}
					class="border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-primary-500 focus:border-primary-500 sm:text-sm"
				>
					{#each availableMethods as method}
						<option value={method}>
							{method === 'all' ? 'All Methods' : method.toUpperCase()}
						</option>
					{/each}
				</select>
			</div>

			<!-- Auto-refresh toggle -->
			<label class="flex items-center space-x-2 cursor-pointer">
				<input 
					bind:checked={autoRefresh}
					type="checkbox" 
					class="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
				/>
				<span class="text-sm text-gray-700">Auto-refresh</span>
			</label>
		</div>
	</div>

	<!-- Events List -->
	<div class="bg-white rounded-xl shadow-sm border border-gray-200">
		<div class="p-6">
			<div class="flex items-center justify-between mb-6">
				<h3 class="text-lg font-semibold text-gray-900">
					Webhook Events
					{#if searchQuery || selectedMethod !== 'all'}
						<span class="text-sm font-normal text-gray-500">
							({filteredEvents.length} filtered)
						</span>
					{:else}
						<span class="text-sm font-normal text-gray-500">
							({webhookStore.totalEvents} total)
						</span>
					{/if}
				</h3>
				<button 
					onclick={() => webhookStore.loadHistory()}
					class="text-sm text-primary-600 hover:text-primary-700 transition-colors"
				>
					Refresh
				</button>
			</div>

			{#if webhookStore.loading}
				<div class="py-12">
					<LoadingSpinner size="lg" text="Loading webhook events..." />
				</div>
			{:else if filteredEvents.length > 0}
				<div class="space-y-4 max-h-96 overflow-y-auto custom-scrollbar">
					{#each filteredEvents as event (event.id)}
						<div class="animate-fade-in">
							<WebhookEventCard {event} />
						</div>
					{/each}
				</div>
			{:else if searchQuery || selectedMethod !== 'all'}
				<!-- No filtered results -->
				<div class="text-center py-12">
					<svg class="mx-auto h-12 w-12 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
					</svg>
					<h3 class="mt-2 text-sm font-medium text-gray-900">No matching events</h3>
					<p class="mt-1 text-sm text-gray-500">
						Try adjusting your search or filter criteria
					</p>
					<button 
						onclick={() => { searchQuery = ''; selectedMethod = 'all'; }}
						class="mt-4 btn-secondary"
					>
						Clear Filters
					</button>
				</div>
			{:else}
				<!-- No events at all -->
				<div class="text-center py-12">
					<div class="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-gray-100">
						<svg class="h-6 w-6 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2 2v-5m16 0h-5.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H1" />
						</svg>
					</div>
					<h3 class="mt-2 text-sm font-medium text-gray-900">No webhook events yet</h3>
					<p class="mt-1 text-sm text-gray-500">
						Send a webhook to your endpoint or use the test button above
					</p>
					<button 
						onclick={sendTestWebhook}
						class="mt-4 btn-primary"
					>
						Send Test Webhook
					</button>
				</div>
			{/if}
		</div>
	</div>
</div>