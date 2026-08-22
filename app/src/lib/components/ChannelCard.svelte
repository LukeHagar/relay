<script lang="ts">
	import type { ChannelConfig } from "$lib/api.svelte";

	let { descriptor, onsave, ondelete }: {
		descriptor: {
			name: string;
			created_at: string;
			response_mode?: string;
			buffer?: { max_events?: number; max_age_s?: number };
			auth?: { bearer_env: string } | null;
			rate_limit?: { burst: number; refill_per_s: number } | null;
			forward?: { url: string; profile?: string; headers?: [string, string][] }[];
			capture_url_local?: string;
			capture_url_hosted?: string | null;
		};
		onsave: (name: string, cfg: ChannelConfig) => Promise<void>;
		ondelete: (name: string) => void;
	} = $props();

	let cfg = $state<ChannelConfig>({});
	let forwards = $state<NonNullable<ChannelConfig["forward"]>>([]);
	let error = $state("");
	let saved = $state(false);

	// Hydrate from the descriptor whenever a (re)loaded descriptor arrives.
	$effect(() => {
		cfg = {
			response_mode: (descriptor.response_mode as ChannelConfig["response_mode"]) ?? "ack",
			buffer: descriptor.buffer,
			auth: descriptor.auth ?? null,
			rate_limit: descriptor.rate_limit ?? undefined,
			forward: descriptor.forward ?? []
		};
		forwards = structuredClone(descriptor.forward ?? []);
	});

	async function save() {
		error = "";
		saved = false;
		cfg.forward = $state.snapshot(forwards) as ChannelConfig["forward"];
		try {
			await onsave(descriptor.name, $state.snapshot(cfg));
			saved = true;
			setTimeout(() => (saved = false), 2500);
		} catch (e) {
			error = String(e);
		}
	}
</script>

<article class="card">
	<div class="head">
		<h2 class="mono">{descriptor.name}</h2>
		<span class="muted mono small">{descriptor.created_at}</span>
		<button class="danger" onclick={() => ondelete(descriptor.name)}>Delete</button>
	</div>
	<div class="capture mono small muted">
		local → {descriptor.capture_url_local ?? "?"}
		{#if descriptor.capture_url_hosted} · hosted → {descriptor.capture_url_hosted}{/if}
	</div>

	<div class="form">
		<label>
			<span>Response mode</span>
			<select bind:value={cfg.response_mode}>
				<option value="ack">ack — 200 JSON receipt</option>
				<option value="echo">echo — mirror body (Slack challenge)</option>
				<option value="static">static — fixed response</option>
			</select>
		</label>

		<label>
			<span>Rate limit (burst / refill-per-sec)</span>
			<span class="pair">
				<input
					type="number"
					placeholder="300"
					value={cfg.rate_limit?.burst ?? ""}
					oninput={(e) =>
						(cfg.rate_limit = e.currentTarget.value
							? { burst: Number(e.currentTarget.value), refill_per_s: cfg.rate_limit?.refill_per_s ?? 100 }
							: undefined)}
				/>
				<input
					type="number"
					placeholder="100"
					value={cfg.rate_limit?.refill_per_s ?? ""}
					oninput={(e) =>
						(cfg.rate_limit = e.currentTarget.value
							? { burst: cfg.rate_limit?.burst ?? 300, refill_per_s: Number(e.currentTarget.value) }
							: undefined)}
				/>
			</span>
		</label>

		<label>
			<span>Bearer env (channel auth)</span>
			<input
				placeholder="GITHUB_TOKEN"
				value={cfg.auth?.bearer_env ?? ""}
				oninput={(e) => (cfg.auth = e.currentTarget.value ? { bearer_env: e.currentTarget.value } : null)}
			/>
		</label>

		<fieldset>
			<legend>Forwarding rules <span class="muted">(Pro)</span></legend>
			{#each forwards as rule, i (i)}
				<span class="pair">
					<input placeholder="https://target/hook" bind:value={rule.url} />
					<input placeholder="signing profile" bind:value={rule.profile} />
					<button
						class="danger"
						aria-label={"remove forwarding rule " + (rule.url || i + 1)}
						onclick={() => {
							forwards.splice(i, 1);
							forwards = [...forwards];
						}}>✕</button
					>
				</span>
			{:else}
				<span class="muted small">none — captured events stay local only</span>
			{/each}
			<button onclick={() => (forwards = [...forwards, { url: "", headers: [] }])}>+ rule</button>
		</fieldset>

		<span class="row">
			<button class="primary" onclick={save}>Save</button>
			{#if saved}<span class="ok mono small">saved ✓</span>{/if}
			{#if error}<span class="err mono small">{error}</span>{/if}
		</span>
	</div>
</article>

<style>
	article { padding: 16px 18px; display: grid; gap: 12px; }
	.head { display: flex; gap: 12px; align-items: center; justify-content: space-between; }
	.head h2 { margin: 0; font-size: 16px; }
	.capture { word-break: break-all; }
	.form { display: grid; gap: 14px; }
	label { display: grid; gap: 5px; font-size: 12px; color: var(--dim); }
	label span:first-child { font-weight: 500; }
	.pair { display: flex; gap: 8px; }
	.pair input { flex: 1; }
	fieldset { border: 1px solid var(--border); border-radius: 10px; padding: 12px; display: grid; gap: 10px; }
	legend { color: var(--dim); font-size: 12px; padding: 0 6px; }
	.row { display: flex; gap: 12px; align-items: center; }
	.mono { font-family: var(--font-mono); }
	.small { font-size: 11px; }
	.ok { color: var(--green); }
	.err { color: var(--red); }
</style>
