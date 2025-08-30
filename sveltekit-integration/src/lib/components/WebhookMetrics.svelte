<script lang="ts">
	import { webhookStore } from '$stores/webhooks';
	import { onMount } from 'svelte';

	// Svelte 5 runes for reactive state
	let metricsData = $state({
		totalEvents: 0,
		eventsToday: 0,
		averageResponseTime: 0,
		successRate: 100,
		topMethods: [] as Array<{ method: string; count: number }>
	});

	let isCalculating = $state(false);

	// Derived calculations using Svelte 5 runes
	let eventsByMethod = $derived.by(() => {
		const methods = new Map<string, number>();
		webhookStore.events.forEach(event => {
			methods.set(event.method, (methods.get(event.method) || 0) + 1);
		});
		return Array.from(methods.entries())
			.map(([method, count]) => ({ method, count }))
			.sort((a, b) => b.count - a.count);
	});

	let eventsToday = $derived.by(() => {
		const today = new Date().toDateString();
		return webhookStore.events.filter(event => 
			new Date(event.createdAt).toDateString() === today
		).length;
	});

	// Effect to update metrics when events change
	$effect(() => {
		if (webhookStore.events.length > 0) {
			updateMetrics();
		}
	});

	function updateMetrics() {
		isCalculating = true;
		
		// Simulate calculation delay for demo
		setTimeout(() => {
			metricsData = {
				totalEvents: webhookStore.totalEvents,
				eventsToday,
				averageResponseTime: Math.floor(Math.random() * 50) + 25, // Simulated
				successRate: 99.8, // Simulated
				topMethods: eventsByMethod.slice(0, 3)
			};
			isCalculating = false;
		}, 300);
	}

	onMount(() => {
		updateMetrics();
	});
</script>

<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
	<!-- Total Events -->
	<div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
		<div class="flex items-center">
			<div class="flex-shrink-0">
				<div class="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center">
					<svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
					</svg>
				</div>
			</div>
			<div class="ml-5 w-0 flex-1">
				<dl>
					<dt class="text-sm font-medium text-gray-500 truncate">Total Events</dt>
					<dd class="text-2xl font-bold text-gray-900">
						{#if isCalculating}
							<div class="loading-shimmer w-16 h-8 rounded"></div>
						{:else}
							{metricsData.totalEvents}
						{/if}
					</dd>
				</dl>
			</div>
		</div>
	</div>

	<!-- Events Today -->
	<div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
		<div class="flex items-center">
			<div class="flex-shrink-0">
				<div class="w-8 h-8 bg-success-500 rounded-lg flex items-center justify-center">
					<svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
					</svg>
				</div>
			</div>
			<div class="ml-5 w-0 flex-1">
				<dl>
					<dt class="text-sm font-medium text-gray-500 truncate">Today</dt>
					<dd class="text-2xl font-bold text-gray-900">
						{#if isCalculating}
							<div class="loading-shimmer w-12 h-8 rounded"></div>
						{:else}
							{metricsData.eventsToday}
						{/if}
					</dd>
				</dl>
			</div>
		</div>
	</div>

	<!-- Average Response Time -->
	<div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
		<div class="flex items-center">
			<div class="flex-shrink-0">
				<div class="w-8 h-8 bg-warning-500 rounded-lg flex items-center justify-center">
					<svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
					</svg>
				</div>
			</div>
			<div class="ml-5 w-0 flex-1">
				<dl>
					<dt class="text-sm font-medium text-gray-500 truncate">Avg Response</dt>
					<dd class="text-2xl font-bold text-gray-900">
						{#if isCalculating}
							<div class="loading-shimmer w-14 h-8 rounded"></div>
						{:else}
							{metricsData.averageResponseTime}ms
						{/if}
					</dd>
				</dl>
			</div>
		</div>
	</div>

	<!-- Success Rate -->
	<div class="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
		<div class="flex items-center">
			<div class="flex-shrink-0">
				<div class="w-8 h-8 bg-success-500 rounded-lg flex items-center justify-center">
					<svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
					</svg>
				</div>
			</div>
			<div class="ml-5 w-0 flex-1">
				<dl>
					<dt class="text-sm font-medium text-gray-500 truncate">Success Rate</dt>
					<dd class="text-2xl font-bold text-gray-900">
						{#if isCalculating}
							<div class="loading-shimmer w-16 h-8 rounded"></div>
						{:else}
							{metricsData.successRate}%
						{/if}
					</dd>
				</dl>
			</div>
		</div>
	</div>
</div>

<!-- Top Methods Chart -->
{#if metricsData.topMethods.length > 0}
	<div class="mt-6 bg-white rounded-xl shadow-sm border border-gray-200 p-6">
		<h3 class="text-lg font-medium text-gray-900 mb-4">Top HTTP Methods</h3>
		<div class="space-y-3">
			{#each metricsData.topMethods as { method, count } (method)}
				<div class="flex items-center justify-between">
					<div class="flex items-center space-x-3">
						<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {method === 'POST' ? 'bg-primary-100 text-primary-800' : method === 'GET' ? 'bg-success-100 text-success-800' : 'bg-gray-100 text-gray-800'}">
							{method}
						</span>
						<span class="text-sm text-gray-600">{count} events</span>
					</div>
					<div class="flex-1 mx-4">
						<div class="bg-gray-200 rounded-full h-2">
							<div 
								class="bg-primary-500 h-2 rounded-full transition-all duration-500"
								style="width: {(count / metricsData.totalEvents) * 100}%"
							></div>
						</div>
					</div>
					<span class="text-sm font-medium text-gray-900">
						{Math.round((count / metricsData.totalEvents) * 100)}%
					</span>
				</div>
			{/each}
		</div>
	</div>
{/if}