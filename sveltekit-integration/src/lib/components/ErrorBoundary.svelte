<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		fallback?: string;
		children: any;
	}

	let { 
		fallback = 'Something went wrong', 
		children 
	}: Props = $props();

	let hasError = $state(false);
	let errorMessage = $state('');

	// Svelte 5 error handling
	function handleError(error: Error) {
		hasError = true;
		errorMessage = error.message;
		console.error('Component error:', error);
	}

	function retry() {
		hasError = false;
		errorMessage = '';
	}

	onMount(() => {
		// Global error handler for unhandled promise rejections
		const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
			handleError(new Error(event.reason));
		};

		window.addEventListener('unhandledrejection', handleUnhandledRejection);

		return () => {
			window.removeEventListener('unhandledrejection', handleUnhandledRejection);
		};
	});
</script>

{#if hasError}
	<div class="min-h-64 flex items-center justify-center">
		<div class="text-center">
			<div class="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-danger-100">
				<svg class="h-6 w-6 text-danger-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
				</svg>
			</div>
			<h3 class="mt-2 text-sm font-medium text-gray-900">{fallback}</h3>
			{#if errorMessage}
				<p class="mt-1 text-sm text-gray-500">{errorMessage}</p>
			{/if}
			<div class="mt-6">
				<button
					onclick={retry}
					class="btn-primary"
				>
					Try Again
				</button>
			</div>
		</div>
	</div>
{:else}
	{@render children()}
{/if}