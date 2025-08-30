<script lang="ts">
	import { webhookStore } from '$stores/webhooks';
	import { onMount } from 'svelte';

	let reconnectAttempts = $state(0);
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

	// Svelte 5 effect to reset reconnect attempts on successful connection
	$effect(() => {
		if (webhookStore.status === 'connected') {
			reconnectAttempts = 0;
		}
	});
</script>

<div class="flex items-center space-x-2">
	<!-- Status Indicator -->
	<div class="flex items-center">
		{#if webhookStore.status === 'connected'}
			<div class="w-3 h-3 bg-success-500 rounded-full animate-pulse"></div>
		{:else if webhookStore.status === 'connecting'}
			<div class="w-3 h-3 bg-warning-500 rounded-full animate-spin"></div>
		{:else}
			<div class="w-3 h-3 bg-danger-500 rounded-full"></div>
		{/if}
		<span class="ml-2 text-sm font-medium text-gray-700 capitalize">
			{webhookStore.status}
		</span>
	</div>

	<!-- Reconnect Button (only show when disconnected) -->
	{#if webhookStore.status === 'disconnected' && reconnectAttempts < maxReconnectAttempts}
		<button
			onclick={handleReconnect}
			class="text-xs text-primary-600 hover:text-primary-500 underline transition-colors"
		>
			Reconnect
		</button>
	{/if}

	<!-- WebSocket Info -->
	{#if webhookStore.status === 'connected'}
		<span class="text-xs text-success-600">
			WebSocket Active
		</span>
	{:else if webhookStore.status === 'disconnected'}
		<span class="text-xs text-danger-500">
			Real-time updates unavailable
		</span>
	{/if}
</div>