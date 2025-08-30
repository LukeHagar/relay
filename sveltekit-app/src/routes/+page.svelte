<script lang="ts">
	import { onMount } from 'svelte';
	import { createWebSocketClient, type WebhookEvent } from '$lib/websocket';
	import { Activity, Zap, Users, Clock, TrendingUp, AlertCircle } from 'lucide-svelte';
	import type { PageData } from './$types';

	export let data: PageData;
	
	let wsClient: ReturnType<typeof createWebSocketClient>;
	let events: WebhookEvent[] = [];
	let stats = {
		totalEvents: 0,
		recentEvents: 0,
		activeConnections: 0,
		successRate: 0
	};

	onMount(() => {
		if (data.session?.user) {
			// Connect to WebSocket for real-time updates
			wsClient = createWebSocketClient(`ws://localhost:4200/api/relay`);
			
			wsClient.events.subscribe((newEvents) => {
				events = newEvents;
				updateStats();
			});
			
			wsClient.connect();
		}
		
		// Load initial data
		loadInitialData();
		
		return () => {
			if (wsClient) {
				wsClient.disconnect();
			}
		};
	});

	async function loadInitialData() {
		if (!data.session?.user) return;
		
		try {
			const response = await fetch('/api/webhooks/stats');
			if (response.ok) {
				stats = await response.json();
			}
		} catch (error) {
			console.error('Failed to load stats:', error);
		}
	}

	function updateStats() {
		stats.totalEvents = events.length;
		stats.recentEvents = events.filter(e => {
			const eventTime = new Date(e.createdAt);
			const oneHourAgo = new Date(Date.now() - 60 * 60 * 1000);
			return eventTime > oneHourAgo;
		}).length;
	}

	function formatDate(dateString: string) {
		return new Date(dateString).toLocaleString();
	}

	function getMethodColor(method: string) {
		const colors = {
			GET: 'bg-blue-100 text-blue-800',
			POST: 'bg-green-100 text-green-800',
			PUT: 'bg-yellow-100 text-yellow-800',
			DELETE: 'bg-red-100 text-red-800',
			PATCH: 'bg-purple-100 text-purple-800'
		};
		return colors[method as keyof typeof colors] || 'bg-gray-100 text-gray-800';
	}
</script>

<svelte:head>
	<title>Dashboard - Webhook Relay</title>
</svelte:head>

{#if !data.session?.user}
	<div class="text-center py-12">
		<div class="max-w-md mx-auto">
			<Zap class="mx-auto h-12 w-12 text-gray-400" />
			<h3 class="mt-2 text-sm font-medium text-gray-900">No session</h3>
			<p class="mt-1 text-sm text-gray-500">Please sign in to access your webhook dashboard.</p>
		</div>
	</div>
{:else}
	<div class="space-y-6">
		<!-- Header -->
		<div class="sm:flex sm:items-center sm:justify-between">
			<div>
				<h1 class="text-2xl font-bold text-gray-900">Dashboard</h1>
				<p class="mt-1 text-sm text-gray-500">
					Welcome back, {data.session.user.name}! Here's what's happening with your webhooks.
				</p>
			</div>
			<div class="mt-4 sm:mt-0">
				<a href="/webhooks" class="btn-primary">
					View All Webhooks
				</a>
			</div>
		</div>

		<!-- Stats Grid -->
		<div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4">
			<div class="card">
				<div class="flex items-center">
					<div class="flex-shrink-0">
						<Activity class="h-6 w-6 text-primary-600" />
					</div>
					<div class="ml-4">
						<div class="text-sm font-medium text-gray-500">Total Events</div>
						<div class="text-2xl font-semibold text-gray-900">{stats.totalEvents}</div>
					</div>
				</div>
			</div>

			<div class="card">
				<div class="flex items-center">
					<div class="flex-shrink-0">
						<Clock class="h-6 w-6 text-green-600" />
					</div>
					<div class="ml-4">
						<div class="text-sm font-medium text-gray-500">Last Hour</div>
						<div class="text-2xl font-semibold text-gray-900">{stats.recentEvents}</div>
					</div>
				</div>
			</div>

			<div class="card">
				<div class="flex items-center">
					<div class="flex-shrink-0">
						<Users class="h-6 w-6 text-blue-600" />
					</div>
					<div class="ml-4">
						<div class="text-sm font-medium text-gray-500">Active Targets</div>
						<div class="text-2xl font-semibold text-gray-900">{stats.activeConnections}</div>
					</div>
				</div>
			</div>

			<div class="card">
				<div class="flex items-center">
					<div class="flex-shrink-0">
						<TrendingUp class="h-6 w-6 text-purple-600" />
					</div>
					<div class="ml-4">
						<div class="text-sm font-medium text-gray-500">Success Rate</div>
						<div class="text-2xl font-semibold text-gray-900">{stats.successRate}%</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Real-time Events -->
		<div class="card">
			<div class="flex items-center justify-between mb-4">
				<h2 class="text-lg font-medium text-gray-900">Recent Webhook Events</h2>
				{#if wsClient}
					<div class="flex items-center space-x-2">
						<div class="connection-status {$wsClient.state.connected ? 'connected' : $wsClient.state.connecting ? 'connecting' : 'disconnected'}">
							{#if $wsClient.state.connected}
								<Activity class="h-3 w-3 mr-1" />
								Connected
							{:else if $wsClient.state.connecting}
								<Clock class="h-3 w-3 mr-1" />
								Connecting...
							{:else}
								<AlertCircle class="h-3 w-3 mr-1" />
								Disconnected
							{/if}
						</div>
					</div>
				{/if}
			</div>

			{#if events.length === 0}
				<div class="text-center py-8">
					<Zap class="mx-auto h-8 w-8 text-gray-400" />
					<p class="mt-2 text-sm text-gray-500">No webhook events yet. Send a webhook to see it here!</p>
				</div>
			{:else}
				<div class="space-y-3 max-h-96 overflow-y-auto">
					{#each events.slice(0, 10) as event}
						<div class="webhook-event">
							<div class="flex items-center justify-between">
								<div class="flex items-center space-x-3">
									<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {getMethodColor(event.method)}">
										{event.method}
									</span>
									<span class="text-sm font-medium text-gray-900">{event.path}</span>
								</div>
								<span class="text-xs text-gray-500">{formatDate(event.createdAt)}</span>
							</div>
							{#if event.body && event.body !== 'null'}
								<div class="mt-2">
									<details class="text-sm">
										<summary class="cursor-pointer text-gray-600 hover:text-gray-900">View Body</summary>
										<pre class="mt-1 p-2 bg-gray-50 rounded text-xs overflow-x-auto">{event.body}</pre>
									</details>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Quick Actions -->
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<div class="card">
				<h3 class="text-lg font-medium text-gray-900 mb-4">Your Webhook URL</h3>
				<div class="bg-gray-50 p-3 rounded-md">
					<code class="text-sm text-gray-800">
						https://{data.session.user.subdomain || 'your-subdomain'}.yourdomain.com
					</code>
				</div>
				<p class="mt-2 text-sm text-gray-500">
					Use this URL to receive webhooks. All incoming requests will be logged and can be relayed to your targets.
				</p>
			</div>

			<div class="card">
				<h3 class="text-lg font-medium text-gray-900 mb-4">Quick Actions</h3>
				<div class="space-y-3">
					<a href="/targets" class="block w-full text-left p-3 border border-gray-200 rounded-md hover:bg-gray-50 transition-colors">
						<div class="font-medium text-gray-900">Add Relay Target</div>
						<div class="text-sm text-gray-500">Configure where to forward your webhooks</div>
					</a>
					<a href="/webhooks" class="block w-full text-left p-3 border border-gray-200 rounded-md hover:bg-gray-50 transition-colors">
						<div class="font-medium text-gray-900">View All Events</div>
						<div class="text-sm text-gray-500">Browse your complete webhook history</div>
					</a>
				</div>
			</div>
		</div>
	</div>
{/if}