<script lang="ts">
	import { onMount } from "svelte";
	import { capture, conn, persistSettings, settings, type Settings } from "$lib/api.svelte";

	let draft = $state<Settings>({ ...settings });
	let savedAt = $state("");
	let health = $state<{ ok?: boolean; version?: string } | null>(null);
	let ready = $state<null | boolean>(null);
	let error = $state("");

	async function probe() {
		persistSettings(draft);
		error = "";
		const c = client();
		try {
			health = await c.health();
		} catch (e) {
			health = null;
			error = `control plane unreachable: ${e}`;
			return;
		}
		try {
			ready = (await c.ready()).ready;
		} catch {
			ready = false;
		}
	}

	onMount(probe);

	function save() {
		persistSettings(draft);
		savedAt = new Date().toLocaleTimeString();
		setTimeout(() => (savedAt = ""), 2500);
		location.reload(); // reconnect everything against the new engine
	}

	async function testCapture() {
		try {
			const res = await capture("__doctor__", "", "");
			error = res.status === 404 ? "" : `capture plane answered ${res.status}`;
			if (res.status === 404) await probe();
		} catch (e) {
			error = `capture plane unreachable: ${e}`;
		}
	}
</script>

<div class="wrap">
	<header>
		<h1>Settings</h1>
		{#if savedAt}<span class="ok mono small">saved {savedAt} — reconnected</span>{/if}
	</header>

	<form class="card" onsubmit={(e) => { e.preventDefault(); save(); }}>
		<label>
			<span>Control plane URL</span>
			<input bind:value={draft.controlUrl} placeholder="http://127.0.0.1:8000" />
		</label>
		<label>
			<span>Capture plane URL</span>
			<input bind:value={draft.captureUrl} placeholder="http://127.0.0.1:8001" />
		</label>
		<label>
			<span>Bearer token <span class="muted">(engine --token)</span></span>
			<input bind:value={draft.token} placeholder="(none)" />
		</label>
		<div class="row">
			<button class="primary" type="submit">Save & reconnect</button>
			<button type="button" onclick={probe}>Probe now</button>
			<button type="button" onclick={testCapture}>Test capture plane</button>
		</div>
		{#if error}<p class="err mono small">{error}</p>{/if}
	</form>

	<section class="card status">
		<h2>Engine</h2>
		<table>
			<tbody>
				<tr><td>version</td><td class="mono">{health?.version ?? "?"}</td></tr>
				<tr><td>liveness</td><td>{health?.ok ? "✓ healthy" : "✗"}</td></tr>
				<tr><td>readiness</td><td>{ready === null ? "?" : ready ? "✓ ready" : "✗ not ready"}</td></tr>
				<tr><td>live stream</td><td>{conn.status}</td></tr>
			</tbody>
		</table>
		<p class="muted small">
			Docs: <a href="{draft.controlUrl}/api/openapi.yaml" target="_blank">OpenAPI spec</a> ·
			<a href="{draft.controlUrl}/api/templates" target="_blank">templates</a>
		</p>
	</section>
</div>

<style>
	.wrap { padding: 20px 24px; overflow-y: auto; display: grid; gap: 16px; align-content: start; }
	header { display: flex; gap: 14px; align-items: baseline; }
	h1 { margin: 0; font-size: 20px; }
	form { display: grid; gap: 14px; max-width: 560px; padding: 18px; }
	label { display: grid; gap: 5px; font-size: 12px; color: var(--dim); }
	label span:first-child { font-weight: 500; }
	input { width: 100%; }
	.row { display: flex; gap: 10px; }
	.status { padding: 16px 18px; max-width: 560px; display: grid; gap: 8px; }
	h2 { margin: 0; font-size: 14px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--dim); }
	table { border-collapse: collapse; font-size: 13px; }
	td { padding: 4px 10px 4px 0; }
	td:first-child { color: var(--dim); width: 120px; }
	.mono { font-family: var(--font-mono); }
	.small { font-size: 11px; }
	.ok { color: var(--green); }
	.err { color: var(--red); word-break: break-all; }
	a { color: var(--primary-300); }
</style>
