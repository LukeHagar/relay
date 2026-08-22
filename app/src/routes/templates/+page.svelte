<script lang="ts">
	import { onMount } from "svelte";
	import { client } from "$lib/api.svelte";

	type TemplateInfo = {
		ref: string;
		provider: string;
		name: string;
		description: string;
		signing_hint?: string;
		defaults: { vars: Record<string, unknown> };
	};

	let templates = $state<TemplateInfo[]>([]);
	let profiles = $state<string[]>([]);
	let selected = $state<TemplateInfo | null>(null);
	let vars = $state<Record<string, string>>({});
	let target = $state("http://localhost:3000");
	let profile = $state("");
	let resultBody = $state("");
	let resultLine = $state("");

	onMount(async () => {
		console.log("TEMPLATES_MOUNT_FIRED");
		try {
			const t = await client().listTemplates();
			console.log("TEMPLATES_FETCHED", t.templates?.length);
			templates = t.templates;
			const p = await client().listSigningProfiles();
			profiles = p.profiles;
			if (templates.length) pick(templates[0]);
		} catch (e) {
			console.error("TEMPLATES_ERROR", e);
		}
	});

	function pick(t: TemplateInfo) {
		selected = t;
		// Only pre-select the hint when that profile actually exists engine-side.
		profile = t.signing_hint && profiles.includes(t.signing_hint) ? t.signing_hint : "";
		vars = Object.fromEntries(Object.entries(t.defaults?.vars ?? {}).map(([k, v]) => [k, String(v)]));
		resultBody = "";
		resultLine = "";
	}

	async function run() {
		if (!selected) return;
		resultLine = "sending…";
		try {
			const result = await client().replay({
				target_url: target,
				source: { template: selected.ref },
				vars,
				signing: profile ? { profile } : undefined
			});
			resultBody = JSON.stringify(result.sent_event.body ? JSON.parse(result.sent_event.body) : null, null, 2);
			resultLine = result.delivery.error
				? `✗ ${result.delivery.error}`
				: `✓ ${result.delivery.status} in ${result.delivery.duration_ms}ms`;
		} catch (e) {
			resultLine = `✗ ${e}`;
		}
	}
</script>

<div class="wrap">
	<header>
		<h1>Replay from template</h1>
		<p class="muted">Provider-shaped events with your values — signed with engine-side profile secrets.</p>
	</header>
	<div class="grid">
		<aside class="card list">
			{#each templates as t (t.ref)}
				<button class:selected={selected?.ref === t.ref} onclick={() => pick(t)}>
					<span class="mono">{t.provider}/{t.name}</span>
					<span class="muted small">{t.description}</span>
				</button>
			{:else}
				<span class="muted empty">loading…</span>
			{/each}
		</aside>

		<section class="card compose">
			{#if selected}
				<h2 class="mono">{selected.ref}</h2>
				<p class="muted">{selected.description}</p>

				<h3>Variables</h3>
				{#each Object.keys(selected.defaults?.vars ?? {}) as name (name)}
					<label>{name}<input bind:value={vars[name]} /></label>
				{/each}

				<h3>Destination & signing</h3>
				<label>Target origin <span class="muted">(template path appends)</span>
					<input bind:value={target} />
				</label>
				<label>Signing profile
					<select bind:value={profile}>
						<option value="">— none —</option>
						{#each profiles as pname (pname)}
							<option>{pname}</option>
						{/each}
						{#if profile && !profiles.includes(profile)}
							<option>{profile}</option>
						{/if}
					</select>
				</label>

				<button class="primary" onclick={run}>Send ▶</button>
				<p class="mono" class:ok={resultLine.startsWith("✓")}>{resultLine}</p>

				{#if resultBody}
					<h3>Sent payload</h3>
					<pre>{resultBody}</pre>
				{/if}
			{:else}
				<p class="muted empty">Pick a template.</p>
			{/if}
		</section>
	</div>
</div>

<style>
	.wrap { padding: 20px 24px; overflow-y: auto; display: grid; gap: 14px; }
	header h1 { margin: 0 0 4px; font-size: 20px; }
	header p { margin: 0; }
	.grid { display: grid; grid-template-columns: 300px 1fr; gap: 16px; align-items: start; }
	.list { display: flex; flex-direction: column; gap: 2px; padding: 8px; }
	.list button {
		display: grid;
		gap: 2px;
		text-align: left;
		background: none;
		border-color: transparent;
		border-radius: 8px;
		padding: 8px 10px;
	}
	.list button:hover, .list button.selected { background: var(--surface-800); border-color: var(--surface-600); }
	.compose { padding: 18px 20px; display: grid; gap: 12px; max-width: 720px; }
	h2 { margin: 0; font-size: 16px; color: var(--primary-300); }
	h3 { margin: 6px 0 2px; font-size: 13px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--dim); }
	label { display: grid; gap: 4px; font-size: 12px; color: var(--dim); }
	input, select { width: 100%; }
	pre {
		background: var(--surface-950);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 12px;
		white-space: pre-wrap;
		word-break: break-word;
		margin: 0;
	}
	.mono { font-family: var(--font-mono); }
	.ok { color: var(--green); }
	.small { font-size: 11px; }
	.empty { padding: 20px; }
</style>
