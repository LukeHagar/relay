<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { webhookStore } from '$stores/webhooks';
	import { notificationStore } from '$stores/notifications';
	import ErrorBoundary from '$components/ErrorBoundary.svelte';
	import NotificationContainer from '$components/NotificationContainer.svelte';
	import '../app.css';

	interface Props {
		data: {
			session?: {
				user?: {
					id: string;
					name?: string;
					email?: string;
					image?: string;
					subdomain?: string;
					username?: string;
				};
			};
		};
	}

	let { data }: Props = $props();

	onMount(() => {
		// Initialize webhook store connection if user is authenticated
		if (data.session?.user) {
			webhookStore.connect();
			webhookStore.loadHistory();
			webhookStore.loadTargets();
		}

		// Cleanup on unmount
		return () => {
			webhookStore.disconnect();
		};
	});

	// Svelte 5 effect for reactive cleanup when session changes
	$effect(() => {
		if (!data.session?.user) {
			webhookStore.disconnect();
		}
	});

	// Svelte 5 effect for WebSocket connection status notifications
	$effect(() => {
		if (webhookStore.status === 'connected') {
			notificationStore.success('Connected', 'Real-time updates are active', 3000);
		} else if (webhookStore.status === 'disconnected') {
			notificationStore.warning('Disconnected', 'Real-time updates are unavailable', 5000);
		}
	});
</script>

<div class="min-h-screen bg-gray-50">
	{#if data.session?.user}
		<nav class="bg-white shadow-sm border-b">
			<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
				<div class="flex justify-between h-16">
					<div class="flex items-center">
						<a href="/dashboard" class="text-xl font-bold text-gray-900">
							Webhook Relay
						</a>
						<div class="ml-8 flex space-x-4">
							<a 
								href="/dashboard" 
								class="px-3 py-2 rounded-md text-sm font-medium transition-colors"
								class:bg-primary-50={$page.url.pathname === '/dashboard'}
								class:text-primary-700={$page.url.pathname === '/dashboard'}
								class:text-gray-500={$page.url.pathname !== '/dashboard'}
							>
								Dashboard
							</a>
							<a 
								href="/dashboard/webhooks" 
								class="px-3 py-2 rounded-md text-sm font-medium transition-colors"
								class:bg-primary-50={$page.url.pathname.startsWith('/dashboard/webhooks')}
								class:text-primary-700={$page.url.pathname.startsWith('/dashboard/webhooks')}
								class:text-gray-500={!$page.url.pathname.startsWith('/dashboard/webhooks')}
							>
								Webhooks
							</a>
							<a 
								href="/dashboard/targets" 
								class="px-3 py-2 rounded-md text-sm font-medium transition-colors"
								class:bg-primary-50={$page.url.pathname.startsWith('/dashboard/targets')}
								class:text-primary-700={$page.url.pathname.startsWith('/dashboard/targets')}
								class:text-gray-500={!$page.url.pathname.startsWith('/dashboard/targets')}
							>
								Relay Targets
							</a>
						</div>
					</div>
					<div class="flex items-center space-x-4">
						<span class="text-sm text-gray-700">
							{data.session.user.name || data.session.user.username}
						</span>
						<form action="/auth/signout" method="post">
							<button 
								type="submit"
								class="btn-secondary"
							>
								Sign Out
							</button>
						</form>
					</div>
				</div>
			</div>
		</nav>
	{/if}

	<main>
		<ErrorBoundary fallback="Application Error - Please refresh the page">
			{#snippet children()}
				<slot />
			{/snippet}
		</ErrorBoundary>
	</main>

	<!-- Global notification container -->
	<NotificationContainer />
</div>