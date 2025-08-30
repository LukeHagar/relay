<script lang="ts">
	import { signIn } from '@auth/sveltekit/client';
	
	interface Props {
		data: {
			session?: {
				user?: any;
			};
		};
	}
	
	let { data }: Props = $props();
</script>

<svelte:head>
	<title>Webhook Relay - SvelteKit</title>
</svelte:head>

{#if data.session?.user}
	<!-- Redirect authenticated users to dashboard -->
	<script>
		window.location.href = '/dashboard';
	</script>
{:else}
	<div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
		<div class="max-w-md w-full space-y-8">
			<div>
				<h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
					Webhook Relay
				</h2>
				<p class="mt-2 text-center text-sm text-gray-600">
					Receive, monitor, and relay webhooks in real-time
				</p>
			</div>
			<div class="mt-8 space-y-6">
				<div class="rounded-md shadow-sm space-y-4">
					<div class="bg-white p-6 rounded-lg shadow">
						<h3 class="text-lg font-medium text-gray-900 mb-4">Features</h3>
						<ul class="space-y-2 text-sm text-gray-600">
							<li class="flex items-center">
								<span class="w-2 h-2 bg-green-500 rounded-full mr-3"></span>
								Real-time webhook monitoring
							</li>
							<li class="flex items-center">
								<span class="w-2 h-2 bg-green-500 rounded-full mr-3"></span>
								Subdomain-based routing
							</li>
							<li class="flex items-center">
								<span class="w-2 h-2 bg-green-500 rounded-full mr-3"></span>
								Multiple relay targets
							</li>
							<li class="flex items-center">
								<span class="w-2 h-2 bg-green-500 rounded-full mr-3"></span>
								Event history and logging
							</li>
						</ul>
					</div>
				</div>

				<div>
					<button
						onclick={() => signIn('github')}
						class="group relative w-full flex justify-center py-3 px-4 border border-transparent text-sm font-medium rounded-lg text-white bg-gray-900 hover:bg-gray-800 focus-ring transition-colors"
					>
						<svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 20 20">
							<path fill-rule="evenodd" d="M10 0C4.477 0 0 4.484 0 10.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0110 4.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.203 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.942.359.31.678.921.678 1.856 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0020 10.017C20 4.484 15.522 0 10 0z" clip-rule="evenodd"></path>
						</svg>
						Sign in with GitHub
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}