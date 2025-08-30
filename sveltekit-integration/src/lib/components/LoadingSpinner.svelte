<script lang="ts">
	interface Props {
		size?: 'sm' | 'md' | 'lg';
		color?: 'primary' | 'success' | 'warning' | 'danger';
		text?: string;
	}

	let { 
		size = 'md', 
		color = 'primary', 
		text 
	}: Props = $props();

	// Svelte 5 derived values for dynamic classes
	let spinnerSize = $derived(() => {
		switch (size) {
			case 'sm': return 'w-4 h-4';
			case 'lg': return 'w-12 h-12';
			default: return 'w-8 h-8';
		}
	});

	let spinnerColor = $derived(() => {
		switch (color) {
			case 'success': return 'border-success-600';
			case 'warning': return 'border-warning-600';
			case 'danger': return 'border-danger-600';
			default: return 'border-primary-600';
		}
	});

	let textSize = $derived(() => {
		switch (size) {
			case 'sm': return 'text-sm';
			case 'lg': return 'text-lg';
			default: return 'text-base';
		}
	});
</script>

<div class="flex flex-col items-center justify-center space-y-3">
	<div 
		class="animate-spin rounded-full border-2 border-gray-300 border-t-current {spinnerSize} {spinnerColor}"
		role="status"
		aria-label="Loading"
	></div>
	{#if text}
		<p class="text-gray-600 {textSize}">{text}</p>
	{/if}
</div>