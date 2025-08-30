<script lang="ts">
	import { onMount } from 'svelte';
	import { Plus, Trash2, Edit, ExternalLink, ToggleLeft, ToggleRight } from 'lucide-svelte';
	import type { PageData } from './$types';

	export let data: PageData;
	
	let targets = data.targets;
	let showAddForm = false;
	let editingTarget: any = null;
	let formData = {
		target: '',
		nickname: ''
	};

	onMount(() => {
		// Initialize form
		resetForm();
	});

	function resetForm() {
		formData = {
			target: '',
			nickname: ''
		};
		editingTarget = null;
		showAddForm = false;
	}

	async function addTarget() {
		if (!formData.target) return;

		try {
			const response = await fetch('/api/targets', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(formData)
			});

			if (response.ok) {
				const newTarget = await response.json();
				targets = [newTarget, ...targets];
				resetForm();
			} else {
				const error = await response.json();
				alert(error.message || 'Failed to add target');
			}
		} catch (error) {
			console.error('Error adding target:', error);
			alert('Failed to add target');
		}
	}

	async function updateTarget() {
		if (!editingTarget || !formData.target) return;

		try {
			const response = await fetch(`/api/targets/${editingTarget.id}`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(formData)
			});

			if (response.ok) {
				const updatedTarget = await response.json();
				targets = targets.map(t => t.id === updatedTarget.id ? updatedTarget : t);
				resetForm();
			} else {
				const error = await response.json();
				alert(error.message || 'Failed to update target');
			}
		} catch (error) {
			console.error('Error updating target:', error);
			alert('Failed to update target');
		}
	}

	async function deleteTarget(targetId: string) {
		if (!confirm('Are you sure you want to delete this target?')) return;

		try {
			const response = await fetch(`/api/targets/${targetId}`, {
				method: 'DELETE'
			});

			if (response.ok) {
				targets = targets.filter(t => t.id !== targetId);
			} else {
				const error = await response.json();
				alert(error.message || 'Failed to delete target');
			}
		} catch (error) {
			console.error('Error deleting target:', error);
			alert('Failed to delete target');
		}
	}

	async function toggleTarget(targetId: string, currentActive: boolean) {
		try {
			const response = await fetch(`/api/targets/${targetId}/toggle`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ active: !currentActive })
			});

			if (response.ok) {
				const updatedTarget = await response.json();
				targets = targets.map(t => t.id === updatedTarget.id ? updatedTarget : t);
			} else {
				const error = await response.json();
				alert(error.message || 'Failed to toggle target');
			}
		} catch (error) {
			console.error('Error toggling target:', error);
			alert('Failed to toggle target');
		}
	}

	function editTarget(target: any) {
		editingTarget = target;
		formData = {
			target: target.target,
			nickname: target.nickname || ''
		};
		showAddForm = true;
	}

	function formatDate(dateString: string) {
		return new Date(dateString).toLocaleDateString();
	}

	function validateUrl(url: string) {
		try {
			new URL(url);
			return true;
		} catch {
			return false;
		}
	}
</script>

<svelte:head>
	<title>Relay Targets - Webhook Relay</title>
</svelte:head>

<div class="space-y-6">
	<!-- Header -->
	<div class="sm:flex sm:items-center sm:justify-between">
		<div>
			<h1 class="text-2xl font-bold text-gray-900">Relay Targets</h1>
			<p class="mt-1 text-sm text-gray-500">
				Configure where your webhooks should be forwarded
			</p>
		</div>
		<div class="mt-4 sm:mt-0">
			<button
				on:click={() => showAddForm = !showAddForm}
				class="btn-primary flex items-center"
			>
				<Plus class="h-4 w-4 mr-2" />
				Add Target
			</button>
		</div>
	</div>

	<!-- Add/Edit Form -->
	{#if showAddForm}
		<div class="card">
			<h2 class="text-lg font-medium text-gray-900 mb-4">
				{editingTarget ? 'Edit Target' : 'Add New Target'}
			</h2>
			<form on:submit|preventDefault={editingTarget ? updateTarget : addTarget} class="space-y-4">
				<div>
					<label for="target" class="block text-sm font-medium text-gray-700 mb-2">
						Target URL *
					</label>
					<input
						id="target"
						type="url"
						bind:value={formData.target}
						placeholder="https://your-endpoint.com/webhook"
						required
						class="input-field"
					/>
					<p class="mt-1 text-sm text-gray-500">
						The URL where webhooks will be forwarded
					</p>
				</div>
				
				<div>
					<label for="nickname" class="block text-sm font-medium text-gray-700 mb-2">
						Nickname (Optional)
					</label>
					<input
						id="nickname"
						type="text"
						bind:value={formData.nickname}
						placeholder="Production Server"
						class="input-field"
					/>
					<p class="mt-1 text-sm text-gray-500">
						A friendly name to identify this target
					</p>
				</div>

				<div class="flex items-center space-x-3">
					<button type="submit" class="btn-primary">
						{editingTarget ? 'Update Target' : 'Add Target'}
					</button>
					<button type="button" on:click={resetForm} class="btn-secondary">
						Cancel
					</button>
				</div>
			</form>
		</div>
	{/if}

	<!-- Targets List -->
	<div class="card">
		<div class="flex items-center justify-between mb-4">
			<h2 class="text-lg font-medium text-gray-900">
				Your Targets ({targets.length})
			</h2>
		</div>

		{#if targets.length === 0}
			<div class="text-center py-8">
				<ExternalLink class="mx-auto h-8 w-8 text-gray-400" />
				<p class="mt-2 text-sm text-gray-500">No relay targets configured yet.</p>
				<p class="text-sm text-gray-500">Add a target to start forwarding webhooks.</p>
			</div>
		{:else}
			<div class="space-y-4">
				{#each targets as target}
					<div class="border border-gray-200 rounded-lg p-4 {target.active ? 'bg-white' : 'bg-gray-50'}">
						<div class="flex items-center justify-between">
							<div class="flex-1">
								<div class="flex items-center space-x-3 mb-2">
									<h3 class="text-sm font-medium text-gray-900">
										{target.nickname || 'Unnamed Target'}
									</h3>
									<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {target.active ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-800'}">
										{target.active ? 'Active' : 'Inactive'}
									</span>
								</div>
								
								<div class="text-sm text-gray-600 mb-2 break-all">
									{target.target}
								</div>
								
								<div class="text-xs text-gray-500">
									Added {formatDate(target.createdAt)}
								</div>
							</div>
							
							<div class="flex items-center space-x-2 ml-4">
								<button
									on:click={() => toggleTarget(target.id, target.active)}
									class="p-2 text-gray-400 hover:text-gray-600 transition-colors"
									title={target.active ? 'Deactivate' : 'Activate'}
								>
									{#if target.active}
										<ToggleRight class="h-4 w-4" />
									{:else}
										<ToggleLeft class="h-4 w-4" />
									{/if}
								</button>
								
								<button
									on:click={() => editTarget(target)}
									class="p-2 text-gray-400 hover:text-gray-600 transition-colors"
									title="Edit"
								>
									<Edit class="h-4 w-4" />
								</button>
								
								<button
									on:click={() => deleteTarget(target.id)}
									class="p-2 text-red-400 hover:text-red-600 transition-colors"
									title="Delete"
								>
									<Trash2 class="h-4 w-4" />
								</button>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Information -->
	<div class="card bg-blue-50 border-blue-200">
		<h3 class="text-sm font-medium text-blue-900 mb-2">How it works</h3>
		<p class="text-sm text-blue-800">
			When a webhook is received at your endpoint, it will be automatically forwarded to all active targets. 
			You can have multiple targets and toggle them on/off as needed. Each target will receive the same 
			webhook data that was originally sent to your endpoint.
		</p>
	</div>
</div>