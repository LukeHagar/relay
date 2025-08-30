<script lang="ts">
	import { onMount } from 'svelte';
	import { User, Key, Globe, Copy, Check, AlertCircle } from 'lucide-svelte';
	import type { PageData } from './$types';

	export let data: PageData;
	
	let user = data.user;
	let webhookUrl = '';
	let copied = false;
	let showSubdomainForm = false;
	let newSubdomain = '';
	let subdomainError = '';

	onMount(() => {
		if (user?.subdomain) {
			webhookUrl = `https://${user.subdomain}.yourdomain.com`;
		}
	});

	async function copyWebhookUrl() {
		if (webhookUrl) {
			await navigator.clipboard.writeText(webhookUrl);
			copied = true;
			setTimeout(() => copied = false, 2000);
		}
	}

	async function updateSubdomain() {
		if (!newSubdomain) return;

		try {
			const response = await fetch('/api/settings/subdomain', {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ subdomain: newSubdomain })
			});

			if (response.ok) {
				const updatedUser = await response.json();
				user = updatedUser;
				webhookUrl = `https://${updatedUser.subdomain}.yourdomain.com`;
				showSubdomainForm = false;
				newSubdomain = '';
				subdomainError = '';
			} else {
				const error = await response.json();
				subdomainError = error.message || 'Failed to update subdomain';
			}
		} catch (error) {
			console.error('Error updating subdomain:', error);
			subdomainError = 'Failed to update subdomain';
		}
	}

	function formatDate(dateString: string) {
		return new Date(dateString).toLocaleDateString();
	}

	function validateSubdomain(subdomain: string) {
		const regex = /^[a-z0-9-]+$/;
		return regex.test(subdomain) && subdomain.length >= 3 && subdomain.length <= 63;
	}
</script>

<svelte:head>
	<title>Settings - Webhook Relay</title>
</svelte:head>

<div class="space-y-6">
	<!-- Header -->
	<div>
		<h1 class="text-2xl font-bold text-gray-900">Settings</h1>
		<p class="mt-1 text-sm text-gray-500">
			Manage your account and webhook configuration
		</p>
	</div>

	{#if user}
		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			<!-- Profile Information -->
			<div class="card">
				<div class="flex items-center space-x-4 mb-6">
					<User class="h-6 w-6 text-primary-600" />
					<h2 class="text-lg font-medium text-gray-900">Profile Information</h2>
				</div>

				<div class="space-y-4">
					<div>
						<label class="block text-sm font-medium text-gray-700">Name</label>
						<div class="mt-1 text-sm text-gray-900">{user.name || 'Not provided'}</div>
					</div>

					<div>
						<label class="block text-sm font-medium text-gray-700">Email</label>
						<div class="mt-1 text-sm text-gray-900">{user.email}</div>
					</div>

					<div>
						<label class="block text-sm font-medium text-gray-700">Username</label>
						<div class="mt-1 text-sm text-gray-900">{user.username}</div>
					</div>

					<div>
						<label class="block text-sm font-medium text-gray-700">Member Since</label>
						<div class="mt-1 text-sm text-gray-900">{formatDate(user.createdAt)}</div>
					</div>
				</div>
			</div>

			<!-- Webhook Configuration -->
			<div class="card">
				<div class="flex items-center space-x-4 mb-6">
					<Globe class="h-6 w-6 text-primary-600" />
					<h2 class="text-lg font-medium text-gray-900">Webhook Configuration</h2>
				</div>

				<div class="space-y-4">
					<div>
						<label class="block text-sm font-medium text-gray-700 mb-2">Your Webhook URL</label>
						<div class="flex items-center space-x-2">
							<div class="flex-1 bg-gray-50 p-3 rounded-md">
								<code class="text-sm text-gray-800 break-all">{webhookUrl}</code>
							</div>
							<button
								on:click={copyWebhookUrl}
								class="p-2 text-gray-400 hover:text-gray-600 transition-colors"
								title="Copy webhook URL"
							>
								{#if copied}
									<Check class="h-4 w-4 text-green-600" />
								{:else}
									<Copy class="h-4 w-4" />
								{/if}
							</button>
						</div>
						<p class="mt-1 text-sm text-gray-500">
							Use this URL to receive webhooks. All requests will be logged and can be relayed to your targets.
						</p>
					</div>

					<div>
						<label class="block text-sm font-medium text-gray-700 mb-2">Subdomain</label>
						<div class="flex items-center space-x-3">
							<div class="flex-1">
								<div class="text-sm text-gray-900">{user.subdomain || 'Not set'}</div>
							</div>
							<button
								on:click={() => showSubdomainForm = !showSubdomainForm}
								class="btn-secondary text-sm"
							>
								{showSubdomainForm ? 'Cancel' : 'Change'}
							</button>
						</div>
					</div>

					{#if showSubdomainForm}
						<div class="border-t pt-4">
							<form on:submit|preventDefault={updateSubdomain} class="space-y-3">
								<div>
									<label for="subdomain" class="block text-sm font-medium text-gray-700 mb-2">
										New Subdomain
									</label>
									<input
										id="subdomain"
										type="text"
										bind:value={newSubdomain}
										placeholder="your-subdomain"
										class="input-field"
										pattern="[a-z0-9-]+"
										minlength="3"
										maxlength="63"
									/>
									<p class="mt-1 text-sm text-gray-500">
										Only lowercase letters, numbers, and hyphens allowed (3-63 characters)
									</p>
									{#if subdomainError}
										<p class="mt-1 text-sm text-red-600 flex items-center">
											<AlertCircle class="h-4 w-4 mr-1" />
											{subdomainError}
										</p>
									{/if}
								</div>
								<div class="flex items-center space-x-3">
									<button
										type="submit"
										disabled={!validateSubdomain(newSubdomain)}
										class="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
									>
										Update Subdomain
									</button>
								</div>
							</form>
						</div>
					{/if}
				</div>
			</div>

			<!-- Security -->
			<div class="card">
				<div class="flex items-center space-x-4 mb-6">
					<Key class="h-6 w-6 text-primary-600" />
					<h2 class="text-lg font-medium text-gray-900">Security</h2>
				</div>

				<div class="space-y-4">
					<div>
						<label class="block text-sm font-medium text-gray-700">Authentication</label>
						<div class="mt-1 text-sm text-gray-900">GitHub OAuth</div>
					</div>

					<div>
						<label class="block text-sm font-medium text-gray-700">Session</label>
						<div class="mt-1 text-sm text-gray-900">Active</div>
					</div>

					<div class="pt-4">
						<button
							on:click={() => signOut()}
							class="btn-secondary"
						>
							Sign Out
						</button>
					</div>
				</div>
			</div>

			<!-- Usage Statistics -->
			<div class="card">
				<div class="flex items-center space-x-4 mb-6">
					<Globe class="h-6 w-6 text-primary-600" />
					<h2 class="text-lg font-medium text-gray-900">Usage</h2>
				</div>

				<div class="space-y-4">
					<div>
						<label class="block text-sm font-medium text-gray-700">Account Type</label>
						<div class="mt-1 text-sm text-gray-900">Free Tier</div>
					</div>

					<div>
						<label class="block text-sm font-medium text-gray-700">Webhook Events</label>
						<div class="mt-1 text-sm text-gray-900">Unlimited</div>
					</div>

					<div>
						<label class="block text-sm font-medium text-gray-700">Relay Targets</label>
						<div class="mt-1 text-sm text-gray-900">Unlimited</div>
					</div>
				</div>
			</div>
		</div>
	{:else}
		<div class="text-center py-12">
			<div class="max-w-md mx-auto">
				<User class="mx-auto h-12 w-12 text-gray-400" />
				<h3 class="mt-2 text-sm font-medium text-gray-900">No session</h3>
				<p class="mt-1 text-sm text-gray-500">Please sign in to access your settings.</p>
			</div>
		</div>
	{/if}
</div>