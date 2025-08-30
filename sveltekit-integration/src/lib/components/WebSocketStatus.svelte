<script lang="ts">
	import { connectionStatus, webhookStore } from '$lib/stores/webhooks';
	import { onMount } from 'svelte';

	let reconnectAttempts = 0;
	const maxReconnectAttempts = 5;

	onMount(() => {
		// Auto-connect when component mounts
		webhookStore.connect();
	});

	function handleReconnect() {
		if (reconnectAttempts < maxReconnectAttempts) {
			reconnectAttempts++;
			webhookStore.connect();
		}
	}

	$: if ($connectionStatus === 'connected') {
		reconnectAttempts = 0; // Reset on successful connection
	}
</script>

<div class="flex items-center space-x-2">
	<!-- Status Indicator -->
	<div class="flex items-center">
		{#if $connectionStatus === 'connected'}
			<div class="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>
		{:else if $connectionStatus === 'connecting'}
			<div class="w-3 h-3 bg-yellow-500 rounded-full animate-spin"></div>
		{:else}
			<div class="w-3 h-3 bg-red-500 rounded-full"></div>
		{/if}
		<span class="ml-2 text-sm font-medium text-gray-700 capitalize">
			{$connectionStatus}
		</span>
	</div>

	<!-- Reconnect Button (only show when disconnected) -->
	{#if $connectionStatus === 'disconnected' && reconnectAttempts < maxReconnectAttempts}
		<button
			on:click={handleReconnect}
			class="text-xs text-blue-600 hover:text-blue-500 underline"
		>
			Reconnect
		</button>
	{/if}

	<!-- WebSocket Info -->
	{#if $connectionStatus === 'connected'}
		<span class="text-xs text-gray-500">
			WebSocket Active
		</span>
	{:else if $connectionStatus === 'disconnected'}
		<span class="text-xs text-red-500">
			Real-time updates unavailable
		</span>
	{/if}
</div>