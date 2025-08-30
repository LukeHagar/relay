<script lang="ts">
	import { connectionStatus } from '$lib/stores/webhooks';
</script>

<div class="flex items-center">
	{#if $connectionStatus === 'connected'}
		<div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
	{:else if $connectionStatus === 'connecting'}
		<div class="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
	{:else}
		<div class="w-2 h-2 bg-red-500 rounded-full"></div>
	{/if}
</div>