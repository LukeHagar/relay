<script lang="ts">
	import { onMount } from "svelte";
	import {
		client,
		capture,
		conn,
		prettyBody,
		settings,
		subscribeLive,
		type Envelope
	} from "$lib/api.svelte";

	let channels = $state<string[]>([]);
	let channel = $state("");
	let filter = $state("");
	let events = $state<Envelope[]>([]);
	let selected = $state<Envelope | null>(null);
	let tab = $state<"body" | "headers" | "replay">("body");
	let dialogEl = $state<HTMLDialogElement>();
	let dialogError = $state("");

	// replay form state
	let target = $state("");
	let profile = $state("");
	let editBody = $state<string | null>(null);
	let replayResult = $state<string>("");

	let stopLive: (() => void) | null = null;
	let listEl = $state<HTMLUListElement>();
	const seenIds = new Set<string>();

	function relTime(iso: string): string {
		const s = (Date.now() - Date.parse(iso)) / 1000;
		if (s < 60) return `${Math.floor(s)}s ago`;
		if (s < 3600) return `${Math.floor(s / 60)}m ago`;
		if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
		return `${Math.floor(s / 86400)}d ago`;
	}

	async function copy(text: string): Promise<void> {
		await navigator.clipboard.writeText(text);
	}

	function moveSelection(delta: number): void {
		if (!filtered.length) return;
		const idx = selected ? filtered.findIndex((e) => e.id === selected!.id) : -1;
		const next = filtered[Math.min(Math.max(idx + delta, 0), filtered.length - 1)];
		selected = next;
		tab = "body";
	}

	onMount(() => {
		refreshChannels();
		return () => stopLive?.();
	});

	async function refreshChannels() {
		const data = await client().listChannels();
		channels = data.channels.map((c) => String(c.name));
		if (!channel && channels.length) select(channels[0]);
	}

	function select(name: string) {
		channel = name;
		events = [];
		selected = null;
		stopLive?.();
		stopLive = subscribeLive({
			channel: () => channel,
			from: () => "oldest",
			onEvent: (env) => {
				events = [env, ...events].slice(0, 500);
				if (!selected) pick(env);
			}
		});
	}

	const filtered = $derived(
		events.filter((e) => {
			const q = filter.toLowerCase();
			return (
				!q ||
				e.path.toLowerCase().includes(q) ||
				e.method.toLowerCase().includes(q) ||
				e.body.toLowerCase().includes(q)
			);
		})
	);

	async function createChannel(name: string) {
		await client().upsertChannel(name, { response_mode: "echo" });
		await refreshChannels();
		select(name);
	}

	function pick(env: Envelope) {
		selected = env;
		editBody = null;
		tab = "body";
	}

	async function runReplay() {
		if (!selected) return;
		replayResult = "sending…";
		try {
			const overrides: Record<string, unknown> = {};
			if (editBody !== null) overrides.body = editBody;
			const result = await client().replay({
				target_url: target,
				source: { event_id: selected.id },
				signing: profile ? { profile } : undefined,
				overrides: Object.keys(overrides).length ? overrides : undefined
			});
			replayResult = result.delivery.error
				? `✗ ${result.delivery.error} (${result.delivery.duration_ms}ms)`
				: `✓ ${result.delivery.status} in ${result.delivery.duration_ms}ms`;
		} catch (e) {
			replayResult = `✗ ${e}`;
		}
	}
</script>

<div class="grid">
	<section class="inbox card">
		<div class="bar">
			<select aria-label="Channel" bind:value={channel} onchange={() => channel && select(channel)}>
				{#each channels as c (c)}
					<option>{c}</option>
				{/each}
			</select>
			<button onclick={() => dialogEl?.showModal()}>+ channel</button>
		</div>
		<div class="bar">
			<input aria-label="Filter events" placeholder="filter path / body…" bind:value={filter} />
		</div>
		<ul bind:this={listEl} tabindex="0" role="listbox" aria-label="Captured events"
			onkeydown={(e) => {
				if (e.key === "ArrowDown") { e.preventDefault(); moveSelection(1); }
				else if (e.key === "ArrowUp") { e.preventDefault(); moveSelection(-1); }
			}}>
			{#each filtered as env (env.id)}
				<li class:selected={selected?.id === env.id} aria-selected={selected?.id === env.id}>
					<button
						onclick={() => pick(env)}
						onkeydown={(e) => e.key === "Enter" && pick(env)}>
						<span class="method">{env.method}</span>
						<span class="path">{env.path || "/"}{env.truncated ? " ⚠" : ""}</span>
						<span class="seq">#{env.seq}</span>
					</button>
				</li>
			{:else}
				<li class="empty muted">no events yet — point a provider at the capture URL</li>
			{/each}
		</ul>
		<div class="bar muted mono small" role="status" aria-live="polite">
			capture → {settings.captureUrl}/c/{channel || "<channel>"}/… · {events.length} events
		</div>
		<div class="bar muted mono small">
			{conn.status === "live" ? `${events.length} events · live` : `offline · ${events.length} cached`}
		</div>
	</section>

	<section class="detail card">
		{#if selected}
			<div class="bar head">
				<h2>{selected.method} {selected.path || "/"}</h2>
			</div>
			<div class="meta mono">
				#{selected.seq} · {relTime(selected.received_at)} ({selected.received_at}) · status {selected.status_sent}
				{#if selected.host}· {selected.host}{/if}
				<button class="copy" title="Copy event id" onclick={() => copy(selected.id)}>⧉ {selected.id.slice(0, 8)}</button>
			</div>
			<nav class="tabs" role="tablist" aria-label="Event detail views">
				<button role="tab" aria-selected={tab === "body"} class:active={tab === "body"} onclick={() => (tab = "body")}>Body</button>
				<button role="tab" aria-selected={tab === "headers"} class:active={tab === "headers"} onclick={() => (tab = "headers")}>Headers</button>
				<button role="tab" aria-selected={tab === "replay"} class:active={tab === "replay"} onclick={() => (tab = "replay")}>Replay ▶</button>
			</nav>
			<div class="content">
				{#if tab === "body"}
					<pre>{prettyBody(selected)}</pre>
				{:else if tab === "headers"}
					<table>
						<tbody>
							{#each selected.headers as [k, v] (k + v)}
								<tr><td class="mono">{k}</td><td>{v}</td></tr>
							{/each}
						</tbody>
					</table>
				{:else}
					<div class="replay-form">
						<label>
							<span>Target origin <span class="muted">(source path appends)</span></span>
							<input bind:value={target} placeholder="http://localhost:3000" />
						</label>
						<label>
							<span>Signing profile</span>
							<input bind:value={profile} placeholder="stripe-dev" />
						</label>
						<details>
							<summary class="muted">Edit body before replay</summary>
							<textarea
								rows="10"
								value={editBody ?? prettyBody(selected)}
								oninput={(e) => (editBody = e.currentTarget.value)}
							></textarea>
						</details>
						<div class="row">
							<button class="primary" onclick={runReplay}>Replay ▶</button>
							<span class="mono">{replayResult}</span>
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<div class="empty muted">Select an event to inspect it.</div>
		{/if}
	</section>
</div>

<dialog bind:this={dialogEl} aria-labelledby="new-channel-title">
	<form
		method="dialog"
		onsubmit={async (e) => {
			e.preventDefault();
			const form = e.currentTarget;
			const data = new FormData(form);
			try {
				await createChannel(String(data.get("name")));
				dialogError = "";
				form.closest("dialog")?.close();
			} catch (err) {
				dialogError = String(err);
			}
		}}
	>
		<h3 id="new-channel-title">New capture channel</h3>
		<label>Name (a-z 0-9 _ -)<input name="name" required pattern="[A-Za-z0-9_-]{1,64}" /></label>
		<label>Response mode
			<select name="mode">
				<option value="ack">ack</option>
				<option value="echo">echo (Slack challenge)</option>
				<option value="static">static</option>
			</select>
		</label>
		{#if dialogError}<p class="dialog-err mono">{dialogError}</p>{/if}
		<menu>
			<button formnovalidate onclick={() => dialogEl?.close()}>Cancel</button>
			<button class="primary">Create</button>
		</menu>
	</form>
</dialog>

<style>
	.grid {
		flex: 1;
		display: grid;
		grid-template-columns: minmax(320px, 40%) 1fr;
		overflow: hidden;
	}
	.inbox { display: flex; flex-direction: column; border-radius: 0; border: none; border-right: 1px solid var(--border); }
	.bar { display: flex; gap: 8px; padding: 10px; align-items: center; }
	.bar input, .bar select { flex: 1; }
	ul { list-style: none; margin: 0; padding: 0; overflow-y: auto; flex: 1; }
	li button {
		width: 100%;
		display: grid;
		grid-template-columns: auto 1fr auto;
		gap: 10px;
		text-align: left;
		background: none;
		border: none;
		border-bottom: 1px solid var(--border);
		border-radius: 0;
		padding: 9px 12px;
	}
	li button:hover, li.selected button { background: var(--surface-800); }
	.method { color: var(--primary-300); font-weight: 600; }
	.path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.seq { color: var(--dim); }
	.detail { display: flex; flex-direction: column; border-radius: 0; border: none; overflow-y: auto; }
	.head h2 { margin: 0; font-size: 15px; }
	.meta { padding: 8px 14px; border-bottom: 1px solid var(--border); color: var(--dim); font-size: 12px; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
	.meta .copy { padding: 2px 8px; font-size: 11px; }
	.tabs { display: flex; gap: 4px; padding: 8px 12px 0; }
	.tabs button.active {
		background: var(--surface-800);
		border-color: var(--primary-400);
		color: var(--primary-200);
	}
	.content { padding: 12px 16px; overflow-y: auto; }
	pre { white-space: pre-wrap; word-break: break-word; margin: 0; }
	table { width: 100%; border-collapse: collapse; font-size: 12px; }
	td { padding: 4px 8px; border-bottom: 1px solid var(--border); vertical-align: top; }
	td:first-child { color: var(--primary-300); white-space: nowrap; }
	.replay-form { display: flex; flex-direction: column; gap: 10px; max-width: 560px; }
	.replay-form label { font-size: 12px; color: var(--dim); display: block; margin-bottom: 4px; }
	.replay-form input, .replay-form textarea { width: 100%; }
	.row { display: flex; gap: 12px; align-items: center; }
	.mono { font-family: var(--font-mono); }
	.small { font-size: 11px; }
	.empty { padding: 28px; text-align: center; }
	dialog { background: var(--surface-900); color: var(--text); border: 1px solid var(--border); border-radius: 12px; }
	dialog::backdrop { background: rgba(0, 0, 0, 0.6); }
	dialog form { display: grid; gap: 12px; min-width: 340px; padding: 8px; }
	dialog h3 { margin: 0; }
	dialog label { display: grid; gap: 4px; font-size: 12px; color: var(--dim); }
	dialog menu { display: flex; gap: 8px; justify-content: flex-end; margin: 0; padding: 0; }
</style>
