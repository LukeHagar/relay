<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { signIn, signOut } from '$lib/auth';
	import { Wifi, WifiOff, Settings, Zap, Users, Home } from 'lucide-svelte';

	export let data;
	$: ({ session } = data);
</script>

<svelte:head>
	<title>Webhook Relay Dashboard</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
	<!-- Navigation -->
	<nav class="bg-white shadow-sm border-b border-gray-200">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between h-16">
				<div class="flex items-center">
					<div class="flex-shrink-0 flex items-center">
						<Zap class="h-8 w-8 text-primary-600" />
						<span class="ml-2 text-xl font-bold text-gray-900">Webhook Relay</span>
					</div>
					<div class="hidden md:ml-6 md:flex md:space-x-8">
						<a href="/" class="text-gray-900 inline-flex items-center px-1 pt-1 border-b-2 border-transparent hover:border-primary-500">
							<Home class="h-4 w-4 mr-1" />
							Dashboard
						</a>
						{#if session?.user}
							<a href="/webhooks" class="text-gray-900 inline-flex items-center px-1 pt-1 border-b-2 border-transparent hover:border-primary-500">
								<Zap class="h-4 w-4 mr-1" />
								Webhooks
							</a>
							<a href="/targets" class="text-gray-900 inline-flex items-center px-1 pt-1 border-b-2 border-transparent hover:border-primary-500">
								<Users class="h-4 w-4 mr-1" />
								Relay Targets
							</a>
							<a href="/settings" class="text-gray-900 inline-flex items-center px-1 pt-1 border-b-2 border-transparent hover:border-primary-500">
								<Settings class="h-4 w-4 mr-1" />
								Settings
							</a>
						{/if}
					</div>
				</div>
				
				<div class="flex items-center space-x-4">
					{#if session?.user}
						<div class="flex items-center space-x-3">
							{#if session.user.image}
								<img class="h-8 w-8 rounded-full" src={session.user.image} alt={session.user.name || 'User'} />
							{/if}
							<div class="hidden md:block">
								<div class="text-sm font-medium text-gray-900">{session.user.name}</div>
								<div class="text-xs text-gray-500">{session.user.email}</div>
							</div>
							<button
								on:click={() => signOut()}
								class="btn-secondary text-sm"
							>
								Sign Out
							</button>
						</div>
					{:else}
						<button
							on:click={() => signIn('github')}
							class="btn-primary"
						>
							Sign In with GitHub
						</button>
					{/if}
				</div>
			</div>
		</div>
	</nav>

	<!-- Main Content -->
	<main class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
		<slot />
	</main>
</div>