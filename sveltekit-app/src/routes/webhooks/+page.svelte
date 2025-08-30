<script lang="ts">
	import { onMount } from 'svelte';
	import { createWebSocketClient, type WebhookEvent } from '$lib/websocket';
	import { Search, Filter, ChevronLeft, ChevronRight, Eye, Copy, Download } from 'lucide-svelte';
	import type { PageData } from './$types';

	export let data: PageData;
	
	let wsClient: ReturnType<typeof createWebSocketClient>;
	let events = data.events;
	let searchTerm = '';
	let selectedMethod = 'all';
	let showFilters = false;
	let currentPage = data.page;
	let totalPages = data.totalPages;

	const methods = ['all', 'GET', 'POST', 'PUT', 'DELETE', 'PATCH'];

	onMount(() => {
		if (data.session?.user) {
			wsClient = createWebSocketClient(`ws://localhost:4200/api/relay`);
			
			wsClient.events.subscribe((newEvents) => {
				// Merge new events with existing ones
				events = [...newEvents, ...events].slice(0, 100);
			});
			
			wsClient.connect();
		}
		
		return () => {
			if (wsClient) {
				wsClient.disconnect();
			}
		};
	});

	$: filteredEvents = events.filter(event => {
		const matchesSearch = event.path.toLowerCase().includes(searchTerm.toLowerCase()) ||
							 event.body.toLowerCase().includes(searchTerm.toLowerCase());
		const matchesMethod = selectedMethod === 'all' || event.method === selectedMethod;
		return matchesSearch && matchesMethod;
	});

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

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
	}

	function downloadEvent(event: WebhookEvent) {
		const dataStr = JSON.stringify(event, null, 2);
		const dataBlob = new Blob([dataStr], { type: 'application/json' });
		const url = URL.createObjectURL(dataBlob);
		const link = document.createElement('a');
		link.href = url;
		link.download = `webhook-${event.id}.json`;
		link.click();
		URL.revokeObjectURL(url);
	}

	function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			window.location.href = `?page=${page}`;
		}
	}
</script>

<svelte:head>
	<title>Webhook Events - Webhook Relay</title>
</svelte:head>

<div class="space-y-6">
	<!-- Header -->
	<div class="sm:flex sm:items-center sm:justify-between">
		<div>
			<h1 class="text-2xl font-bold text-gray-900">Webhook Events</h1>
			<p class="mt-1 text-sm text-gray-500">
				View and manage all your incoming webhook events
			</p>
		</div>
		<div class="mt-4 sm:mt-0 flex space-x-3">
			<button
				on:click={() => showFilters = !showFilters}
				class="btn-secondary flex items-center"
			>
				<Filter class="h-4 w-4 mr-2" />
				Filters
			</button>
			{#if wsClient}
				<div class="connection-status {$wsClient.state.connected ? 'connected' : $wsClient.state.connecting ? 'connecting' : 'disconnected'}">
					{#if $wsClient.state.connected}
						Live Updates
					{:else if $wsClient.state.connecting}
						Connecting...
					{:else}
						Disconnected
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<!-- Filters -->
	{#if showFilters}
		<div class="card">
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<div>
					<label for="search" class="block text-sm font-medium text-gray-700 mb-2">
						Search
					</label>
					<div class="relative">
						<Search class="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
						<input
							id="search"
							type="text"
							bind:value={searchTerm}
							placeholder="Search in path or body..."
							class="input-field pl-10"
						/>
					</div>
				</div>
				<div>
					<label for="method" class="block text-sm font-medium text-gray-700 mb-2">
						HTTP Method
					</label>
					<select
						id="method"
						bind:value={selectedMethod}
						class="input-field"
					>
						{#each methods as method}
							<option value={method}>{method}</option>
						{/each}
					</select>
				</div>
			</div>
		</div>
	{/if}

	<!-- Events List -->
	<div class="card">
		<div class="flex items-center justify-between mb-4">
			<h2 class="text-lg font-medium text-gray-900">
				Events ({filteredEvents.length})
			</h2>
			<div class="text-sm text-gray-500">
				Total: {data.total}
			</div>
		</div>

		{#if filteredEvents.length === 0}
			<div class="text-center py-8">
				<Search class="mx-auto h-8 w-8 text-gray-400" />
				<p class="mt-2 text-sm text-gray-500">
					{#if searchTerm || selectedMethod !== 'all'}
						No events match your filters.
					{:else}
						No webhook events yet.
					{/if}
				</p>
			</div>
		{:else}
			<div class="space-y-4">
				{#each filteredEvents as event}
					<div class="webhook-event">
						<div class="flex items-start justify-between">
							<div class="flex-1">
								<div class="flex items-center space-x-3 mb-2">
									<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {getMethodColor(event.method)}">
										{event.method}
									</span>
									<span class="text-sm font-medium text-gray-900">{event.path}</span>
									{#if event.query}
										<span class="text-sm text-gray-500">?{event.query}</span>
									{/if}
								</div>
								
								<div class="text-xs text-gray-500 mb-2">
									{formatDate(event.createdAt)}
								</div>

								{#if event.body && event.body !== 'null'}
									<details class="text-sm">
										<summary class="cursor-pointer text-gray-600 hover:text-gray-900 mb-2">
											View Request Body
										</summary>
										<pre class="bg-gray-50 p-3 rounded text-xs overflow-x-auto">{event.body}</pre>
									</details>
								{/if}

								{#if event.headers && event.headers !== 'null'}
									<details class="text-sm">
										<summary class="cursor-pointer text-gray-600 hover:text-gray-900">
											View Headers
										</summary>
										<pre class="bg-gray-50 p-3 rounded text-xs overflow-x-auto mt-2">{event.headers}</pre>
									</details>
								{/if}
							</div>
							
							<div class="flex items-center space-x-2 ml-4">
								<button
									on:click={() => copyToClipboard(JSON.stringify(event, null, 2))}
									class="p-2 text-gray-400 hover:text-gray-600 transition-colors"
									title="Copy event data"
								>
									<Copy class="h-4 w-4" />
								</button>
								<button
									on:click={() => downloadEvent(event)}
									class="p-2 text-gray-400 hover:text-gray-600 transition-colors"
									title="Download event"
								>
									<Download class="h-4 w-4" />
								</button>
							</div>
						</div>
					</div>
				{/each}
			</div>

			<!-- Pagination -->
			{#if totalPages > 1}
				<div class="flex items-center justify-between mt-6 pt-6 border-t border-gray-200">
					<div class="text-sm text-gray-500">
						Page {currentPage} of {totalPages}
					</div>
					<div class="flex items-center space-x-2">
						<button
							on:click={() => goToPage(currentPage - 1)}
							disabled={currentPage <= 1}
							class="p-2 text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							<ChevronLeft class="h-4 w-4" />
						</button>
						<button
							on:click={() => goToPage(currentPage + 1)}
							disabled={currentPage >= totalPages}
							class="p-2 text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							<ChevronRight class="h-4 w-4" />
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</div>