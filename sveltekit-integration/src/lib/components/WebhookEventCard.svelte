<script lang="ts">
	import type { WebhookEvent } from '$stores/webhooks';
	
	interface Props {
		event: WebhookEvent;
	}
	
	let { event }: Props = $props();
	
	// Svelte 5 derived state using runes
	let formattedTime = $derived(new Date(event.createdAt).toLocaleString());
	let methodColor = $derived(getMethodColor(event.method));
	
	function getMethodColor(method: string) {
		switch (method.toLowerCase()) {
			case 'get': return 'bg-success-100 text-success-800';
			case 'post': return 'bg-primary-100 text-primary-800';
			case 'put': return 'bg-warning-100 text-warning-800';
			case 'patch': return 'bg-warning-100 text-warning-700';
			case 'delete': return 'bg-danger-100 text-danger-800';
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
	
	let expanded = $state(false);
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
			onclick={() => expanded = !expanded}
			class="text-gray-400 hover:text-gray-600 transition-colors"
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