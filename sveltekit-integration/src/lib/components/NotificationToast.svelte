<script lang="ts">
	import { notificationStore, type Notification } from '$stores/notifications';
	import { onMount } from 'svelte';

	interface Props {
		notification: Notification;
	}

	let { notification }: Props = $props();

	let isVisible = $state(false);
	let isRemoving = $state(false);

	// Derived styles based on notification type
	let bgColor = $derived(() => {
		switch (notification.type) {
			case 'success': return 'bg-success-50 border-success-200';
			case 'error': return 'bg-danger-50 border-danger-200';
			case 'warning': return 'bg-warning-50 border-warning-200';
			case 'info': return 'bg-primary-50 border-primary-200';
			default: return 'bg-gray-50 border-gray-200';
		}
	});

	let iconColor = $derived(() => {
		switch (notification.type) {
			case 'success': return 'text-success-600';
			case 'error': return 'text-danger-600';
			case 'warning': return 'text-warning-600';
			case 'info': return 'text-primary-600';
			default: return 'text-gray-600';
		}
	});

	let icon = $derived(() => {
		switch (notification.type) {
			case 'success': return 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z';
			case 'error': return 'M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z';
			case 'warning': return 'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z';
			case 'info': return 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z';
			default: return 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z';
		}
	});

	function dismiss() {
		isRemoving = true;
		setTimeout(() => {
			notificationStore.remove(notification.id);
		}, 300);
	}

	onMount(() => {
		// Slide in animation
		setTimeout(() => {
			isVisible = true;
		}, 50);
	});
</script>

<div 
	class="transform transition-all duration-300 ease-in-out {isVisible ? 'translate-x-0 opacity-100' : 'translate-x-full opacity-0'} {isRemoving ? 'translate-x-full opacity-0' : ''}"
>
	<div class="max-w-sm w-full border rounded-lg shadow-lg pointer-events-auto {bgColor}">
		<div class="p-4">
			<div class="flex items-start">
				<div class="flex-shrink-0">
					<svg class="h-5 w-5 {iconColor}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={icon} />
					</svg>
				</div>
				<div class="ml-3 w-0 flex-1">
					<p class="text-sm font-medium text-gray-900">
						{notification.title}
					</p>
					{#if notification.message}
						<p class="mt-1 text-sm text-gray-600">
							{notification.message}
						</p>
					{/if}
				</div>
				<div class="ml-4 flex-shrink-0 flex">
					<button
						onclick={dismiss}
						class="rounded-md inline-flex text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-primary-500 transition-colors"
					>
						<span class="sr-only">Close</span>
						<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>
			</div>
		</div>
	</div>
</div>