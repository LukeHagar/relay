<script lang="ts">
	import { webhookStore } from '$stores/webhooks';
	import { notificationStore } from '$stores/notifications';
	import { Plus, ExternalLink, Trash2 } from 'lucide-svelte';
	
	interface Props {
		data: {
			session?: {
				user?: any;
			};
		};
	}
	
	let { data }: Props = $props();
	
	let showAddForm = $state(false);
	let newTarget = $state('');
	let newNickname = $state('');
	let isSubmitting = $state(false);
	
	async function addTarget() {
		if (!newTarget.trim()) return;
		
		isSubmitting = true;
		try {
			await webhookStore.addTargetRemote(newTarget.trim(), newNickname.trim() || undefined);
			newTarget = '';
			newNickname = '';
			showAddForm = false;
			notificationStore.success('Target Added', 'Relay target has been successfully configured');
		} catch (error) {
			console.error('Failed to add target:', error);
			notificationStore.error('Failed to Add Target', 'Please check the URL and try again');
		} finally {
			isSubmitting = false;
		}
	}
	
	async function removeTarget(targetId: string) {
		if (!confirm('Are you sure you want to remove this relay target?')) return;
		
		try {
			await webhookStore.removeTargetRemote(targetId);
			notificationStore.success('Target Removed', 'Relay target has been deactivated');
		} catch (error) {
			console.error('Failed to remove target:', error);
			notificationStore.error('Failed to Remove Target', 'Please try again');
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
					onclick={() => showAddForm = !showAddForm}
					class="btn-primary inline-flex items-center"
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
					<form onsubmit={(e) => { e.preventDefault(); addTarget(); }} class="space-y-4">
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
								onclick={() => showAddForm = false}
								class="btn-secondary"
							>
								Cancel
							</button>
							<button
								type="submit"
								disabled={isSubmitting}
								class="btn-primary disabled:opacity-50"
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
					Active Relay Targets ({webhookStore.targets.length})
				</h3>
				
				{#if webhookStore.targets.length > 0}
					<div class="space-y-4">
						{#each webhookStore.targets as target (target.id)}
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
										<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-success-100 text-success-800">
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
										onclick={() => removeTarget(target.id)}
										class="text-danger-400 hover:text-danger-600 transition-colors"
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
							onclick={() => showAddForm = true}
							class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-lg text-primary-600 bg-primary-100 hover:bg-primary-200 transition-colors"
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