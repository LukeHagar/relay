<script lang="ts">
	import { relayTargets, webhookStore } from '$lib/stores/webhooks';
	import { Plus, ExternalLink, Trash2 } from 'lucide-svelte';
	
	export let data;
	
	let showAddForm = false;
	let newTarget = '';
	let newNickname = '';
	let isSubmitting = false;
	
	async function addTarget() {
		if (!newTarget.trim()) return;
		
		isSubmitting = true;
		try {
			await webhookStore.addTarget(newTarget.trim(), newNickname.trim() || undefined);
			newTarget = '';
			newNickname = '';
			showAddForm = false;
		} catch (error) {
			console.error('Failed to add target:', error);
			alert('Failed to add relay target. Please check the URL and try again.');
		} finally {
			isSubmitting = false;
		}
	}
	
	async function removeTarget(targetId: string) {
		if (!confirm('Are you sure you want to remove this relay target?')) return;
		
		try {
			await webhookStore.removeTarget(targetId);
		} catch (error) {
			console.error('Failed to remove target:', error);
			alert('Failed to remove relay target.');
		}
	}
</script>

<svelte:head>
	<title>Relay Targets - Webhook Relay</title>
</svelte:head>

<div class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
	<div class="px-4 py-6 sm:px-0">
		<div class="mb-8">
			<div class="flex items-center justify-between">
				<div>
					<h1 class="text-2xl font-bold text-gray-900">Relay Targets</h1>
					<p class="mt-2 text-gray-600">
						Configure where your webhooks should be forwarded
					</p>
				</div>
				<button
					on:click={() => showAddForm = !showAddForm}
					class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700"
				>
					<Plus class="w-4 h-4 mr-2" />
					Add Target
				</button>
			</div>
		</div>

		<!-- Add Target Form -->
		{#if showAddForm}
			<div class="bg-white shadow rounded-lg mb-8">
				<div class="px-4 py-5 sm:p-6">
					<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
						Add New Relay Target
					</h3>
					<form on:submit|preventDefault={addTarget} class="space-y-4">
						<div>
							<label for="target-url" class="block text-sm font-medium text-gray-700">
								Target URL
							</label>
							<input
								id="target-url"
								type="url"
								bind:value={newTarget}
								placeholder="https://example.com/webhook"
								required
								class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
							/>
						</div>
						<div>
							<label for="nickname" class="block text-sm font-medium text-gray-700">
								Nickname (Optional)
							</label>
							<input
								id="nickname"
								type="text"
								bind:value={newNickname}
								placeholder="My API Server"
								class="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
							/>
						</div>
						<div class="flex justify-end space-x-3">
							<button
								type="button"
								on:click={() => showAddForm = false}
								class="bg-white py-2 px-4 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 hover:bg-gray-50"
							>
								Cancel
							</button>
							<button
								type="submit"
								disabled={isSubmitting}
								class="bg-blue-600 py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
							>
								{isSubmitting ? 'Adding...' : 'Add Target'}
							</button>
						</div>
					</form>
				</div>
			</div>
		{/if}

		<!-- Targets List -->
		<div class="bg-white shadow rounded-lg">
			<div class="px-4 py-5 sm:p-6">
				<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
					Active Relay Targets ({$relayTargets.length})
				</h3>
				
				{#if $relayTargets.length > 0}
					<div class="space-y-4">
						{#each $relayTargets as target (target.id)}
							<div class="flex items-center justify-between p-4 border border-gray-200 rounded-lg">
								<div class="flex-1">
									<div class="flex items-center space-x-3">
										<div class="flex-1">
											{#if target.nickname}
												<h4 class="text-sm font-medium text-gray-900">{target.nickname}</h4>
												<p class="text-sm text-gray-500">{target.target}</p>
											{:else}
												<p class="text-sm font-medium text-gray-900">{target.target}</p>
											{/if}
										</div>
										<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
											Active
										</span>
									</div>
									<p class="text-xs text-gray-400 mt-1">
										Added {new Date(target.createdAt).toLocaleDateString()}
									</p>
								</div>
								<div class="flex items-center space-x-2 ml-4">
									<a
										href={target.target}
										target="_blank"
										rel="noopener noreferrer"
										class="text-gray-400 hover:text-gray-600"
										title="Open in new tab"
									>
										<ExternalLink class="w-4 h-4" />
									</a>
									<button
										on:click={() => removeTarget(target.id)}
										class="text-red-400 hover:text-red-600"
										title="Remove target"
									>
										<Trash2 class="w-4 h-4" />
									</button>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-8">
						<div class="text-gray-400 mb-2">
							<svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
							</svg>
						</div>
						<h3 class="text-sm font-medium text-gray-900">No relay targets configured</h3>
						<p class="text-sm text-gray-500 mb-4">
							Add relay targets to forward incoming webhooks to your services.
						</p>
						<button
							on:click={() => showAddForm = true}
							class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-blue-600 bg-blue-100 hover:bg-blue-200"
						>
							<Plus class="w-4 h-4 mr-2" />
							Add Your First Target
						</button>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>