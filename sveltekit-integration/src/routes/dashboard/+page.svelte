<script lang="ts">
	import { onMount } from 'svelte';
	import { webhookStore } from '$stores/webhooks';
	import WebhookMetrics from '$components/WebhookMetrics.svelte';
	import ModernWebhookDashboard from '$components/ModernWebhookDashboard.svelte';
	
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
	let user = $derived(data.session?.user);
</script>

<svelte:head>
	<title>Dashboard - Webhook Relay</title>
</svelte:head>

<div class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
	<div class="px-4 py-6 sm:px-0">
		<div class="mb-8">
			<h1 class="text-2xl font-bold text-gray-900">Webhook Dashboard</h1>
			<p class="mt-2 text-gray-600">
				Monitor and manage your webhook endpoints in real-time
			</p>
		</div>

		<!-- Enhanced Metrics Dashboard -->
		<WebhookMetrics />

		<!-- Modern Webhook Dashboard -->
		<ModernWebhookDashboard {user} />
	</div>
</div>