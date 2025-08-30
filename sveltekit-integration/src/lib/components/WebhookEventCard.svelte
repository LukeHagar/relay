<script lang="ts">
	import type { WebhookEvent } from '$lib/stores/webhooks';
	
	export let event: WebhookEvent;
	
	$: formattedTime = new Date(event.createdAt).toLocaleString();
	$: methodColor = getMethodColor(event.method);
	
	function getMethodColor(method: string) {
		switch (method.toLowerCase()) {
			case 'get': return 'bg-green-100 text-green-800';
			case 'post': return 'bg-blue-100 text-blue-800';
			case 'put': return 'bg-yellow-100 text-yellow-800';
			case 'patch': return 'bg-orange-100 text-orange-800';
			case 'delete': return 'bg-red-100 text-red-800';
			default: return 'bg-gray-100 text-gray-800';
		}
	}
	
	function formatJson(obj: any) {
		try {
			return JSON.stringify(obj, null, 2);
		} catch {
			return String(obj);
		}
	}
	
	let expanded = false;
</script>

<div class="border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow">
	<div class="flex items-center justify-between">
		<div class="flex items-center space-x-3">
			<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {methodColor}">
				{event.method}
			</span>
			<span class="text-sm font-medium text-gray-900">{event.path}</span>
			{#if event.query}
				<span class="text-xs text-gray-500">{event.query}</span>
			{/if}
		</div>
		<div class="flex items-center space-x-2">
			<span class="text-xs text-gray-500">{formattedTime}</span>
			<button 
				on:click={() => expanded = !expanded}
				class="text-gray-400 hover:text-gray-600"
			>
				<svg 
					class="w-4 h-4 transition-transform" 
					class:rotate-180={expanded}
					fill="none" 
					viewBox="0 0 24 24" 
					stroke="currentColor"
				>
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
				</svg>
			</button>
		</div>
	</div>
	
	{#if expanded}
		<div class="mt-4 space-y-3">
			{#if event.body && event.body !== 'null'}
				<div>
					<h4 class="text-sm font-medium text-gray-700 mb-2">Request Body</h4>
					<pre class="bg-gray-50 rounded p-3 text-xs overflow-x-auto">{formatJson(event.body)}</pre>
				</div>
			{/if}
			
			{#if event.headers}
				<div>
					<h4 class="text-sm font-medium text-gray-700 mb-2">Headers</h4>
					<pre class="bg-gray-50 rounded p-3 text-xs overflow-x-auto">{formatJson(event.headers)}</pre>
				</div>
			{/if}
		</div>
	{/if}
</div>