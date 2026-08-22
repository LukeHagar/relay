<script lang="ts">
	import { onMount } from "svelte";
	import { client } from "$lib/api.svelte";
	import ChannelCard from "$lib/components/ChannelCard.svelte";

	interface Descriptor {
		name: string;
		created_at: string;
		response_mode?: string;
		capture_url_local?: string;
		capture_url_hosted?: string | null;
	}

	let descriptors = $state<Descriptor[]>([]);

	async function refresh() {
		descriptors = (await client().listChannels()).channels as Descriptor[];
	}
	onMount(refresh);

	async function save(name: string, cfg: import("$lib/api.svelte").ChannelConfig) {
		await client().upsertChannel(name, cfg);
		await refresh();
	}

	async function remove(name: string) {
		if (!confirm(`Delete channel "${name}"? Buffer and subscriptions are dropped.`)) return;
		await client().deleteChannel(name);
		await refresh();
	}
</script>

<div class="wrap">
	<header>
		<h1>Channels</h1>
		<p class="muted small">Every knob the engine supports, per channel. Changes apply live.</p>
	</header>

	{#each descriptors as d (d.name)}
			<ChannelCard descriptor={d} onsave={save} ondelete={remove} />
	{:else}
		<p class="empty card">No channels yet — create one from the Inbox.</p>
	{/each}
</div>

<style>
	.wrap {
		padding: 20px 24px;
		overflow-y: auto;
		display: grid;
		gap: 16px;
		align-content: start;
	}
	header h1 {
		margin: 0 0 2px;
		font-size: 20px;
	}
	header p {
		margin: 0;
	}
	.empty {
		padding: 24px;
		text-align: center;
		max-width: none;
	}
</style>
