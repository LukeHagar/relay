<script lang="ts">
	import { onMount } from 'svelte';
	import { webhookEvents, connectionStatus, recentEvents } from '$lib/stores/webhooks';
	import ConnectionStatus from '$lib/components/ConnectionStatus.svelte';
	import WebhookEventCard from '$lib/components/WebhookEventCard.svelte';
	
	export let data;

	$: user = data.session?.user;
</script>

<svelte:head>
	<title>Dashboard - Webhook Relay</title>
</svelte:head>

<div class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
	<div class="px-4 py-6 sm:px-0">
		<div class="mb-8">
			<h1 class="text-2xl font-bold text-gray-900">Webhook Dashboard</h1>
			<p class="mt-2 text-gray-600">
				Monitor and manage your webhook endpoints in real-time
			</p>
		</div>

		<!-- User Info & Connection Status -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
			<div class="bg-white overflow-hidden shadow rounded-lg">
				<div class="p-5">
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<img class="h-10 w-10 rounded-full" src={user?.image} alt={user?.name} />
						</div>
						<div class="ml-5 w-0 flex-1">
							<dl>
								<dt class="text-sm font-medium text-gray-500 truncate">Subdomain</dt>
								<dd class="text-lg font-medium text-gray-900">{user?.subdomain}</dd>
							</dl>
						</div>
					</div>
				</div>
			</div>

			<div class="bg-white overflow-hidden shadow rounded-lg">
				<div class="p-5">
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<ConnectionStatus />
						</div>
						<div class="ml-5 w-0 flex-1">
							<dl>
								<dt class="text-sm font-medium text-gray-500 truncate">Connection</dt>
								<dd class="text-lg font-medium text-gray-900 capitalize">{$connectionStatus}</dd>
							</dl>
						</div>
					</div>
				</div>
			</div>

			<div class="bg-white overflow-hidden shadow rounded-lg">
				<div class="p-5">
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<div class="w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center">
								<span class="text-white text-sm font-medium">{$webhookEvents.length}</span>
							</div>
						</div>
						<div class="ml-5 w-0 flex-1">
							<dl>
								<dt class="text-sm font-medium text-gray-500 truncate">Total Events</dt>
								<dd class="text-lg font-medium text-gray-900">Today</dd>
							</dl>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Webhook Endpoint Info -->
		<div class="bg-white shadow rounded-lg mb-8">
			<div class="px-4 py-5 sm:p-6">
				<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
					Your Webhook Endpoint
				</h3>
				<div class="bg-gray-50 rounded-md p-4">
					<code class="text-sm text-gray-800">
						POST https://yourdomain.com/api/webhook/{user?.subdomain}
					</code>
				</div>
				<p class="mt-2 text-sm text-gray-500">
					Send webhooks to this endpoint. All events will be logged and forwarded to your connected relay targets.
				</p>
			</div>
		</div>

		<!-- Recent Events -->
		<div class="bg-white shadow rounded-lg">
			<div class="px-4 py-5 sm:p-6">
				<div class="flex items-center justify-between mb-4">
					<h3 class="text-lg leading-6 font-medium text-gray-900">
						Recent Webhook Events
					</h3>
					<a 
						href="/dashboard/webhooks" 
						class="text-sm text-blue-600 hover:text-blue-500"
					>
						View all →
					</a>
				</div>
				
				{#if $recentEvents.length > 0}
					<div class="space-y-4">
						{#each $recentEvents as event (event.id)}
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
						<p class="text-sm text-gray-500">
							Send a test webhook to your endpoint to get started.
						</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>