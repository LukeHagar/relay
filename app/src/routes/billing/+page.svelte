<script lang="ts">
	import { onMount } from "svelte";
	import { client } from "$lib/api.svelte";

	let plans = $state<{ id: string; name: string; max_channels: number | string; price_monthly_usd: number }[]>([]);
	let subscription = $state<{ plan_id: string; status: string; current_period_end_unix?: number } | null>(null);
	let enabled = $state(false);
	let successUrl = $state("http://127.0.0.1:8000/billing?success=1");
	let cancelUrl = $state("http://127.0.0.1:8000/billing");
	let message = $state("");
	let busy = $state(false);

	async function refresh() {
		const data = await client().billingPlans();
		plans = data.plans;
		subscription = data.subscription;
		enabled = data.billing_enabled;
	}
	onMount(refresh);

	function isCurrent(planId: string): boolean {
		return subscription?.plan_id === planId && !!subscription && ["active", "trialing"].includes(subscription.status);
	}

	async function checkout() {
		message = "";
		busy = true;
		try {
			const { url } = await client().billingCheckout(successUrl, cancelUrl);
			message = "opening Stripe Checkout…";
			window.open(url, "_blank");
		} catch (e) {
			message = `✗ ${e}`;
		} finally {
			busy = false;
		}
	}
</script>

<div class="wrap">
	<header>
		<h1>Billing</h1>
		{#if !enabled}
			<span class="badge offline"><span class="dot"></span>disabled — start the engine with --billing-enabled</span>
		{:else if isCurrent("pro")}
			<span class="badge gold"><span class="dot"></span>PRO · {subscription?.status}</span>
		{:else}
			<span class="badge online"><span class="dot"></span>FREE · {subscription?.status ?? "active"}</span>
		{/if}
	</header>

	{#if subscription?.current_period_end_unix}
		<p class="muted mono small">renews at {new Date((subscription?.current_period_end_unix ?? 0) * 1000).toISOString().slice(0, 10)}</p>
	{/if}
	{#if message}<p class="mono">{message}</p>{/if}

	<div class="plans">
		{#each plans as plan (plan.id)}
			<article class="card plan" class:pro={plan.id === "pro"} class:current={isCurrent(plan.id)}>
				<h2>{plan.name}</h2>
				<p class="price">${plan.price_monthly_usd}<span class="muted">/mo</span></p>
				<ul>
					<li>{typeof plan.max_channels === "number" && plan.max_channels < 100 ? `${plan.max_channels} channels` : "unlimited channels"}</li>
					<li>capture · inspect · replay</li>
					<li>templates & signing profiles</li>
					{#if plan.id === "pro"}
						<li class="gold">forwarding rules</li>
						<li class="gold">unlimited everything</li>
					{/if}
				</ul>
				{#if isCurrent(plan.id)}
					<button disabled>current plan ✓</button>
				{:else if plan.id === "pro"}
					<button class="gold" disabled={busy || !enabled} onclick={checkout}>Upgrade with Stripe →</button>
				{:else}
					<button disabled>included</button>
				{/if}
			</article>
		{/each}
	</div>

	<p class="muted small">
		Checkout runs through Stripe. The engine never sees card data — it only receives
		signature-verified webhooks (<span class="mono">checkout.session.completed</span>,
		<span class="mono">customer.subscription.*</span>). Self-hosted always works without an account.
	</p>
</div>

<style>
	.wrap { padding: 20px 24px; overflow-y: auto; display: grid; gap: 16px; align-content: start; }
	header { display: flex; gap: 14px; align-items: center; }
	h1 { margin: 0; font-size: 20px; }
	.plans { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 320px)); gap: 16px; }
	.plan { padding: 20px; display: grid; gap: 12px; align-content: start; }
	.plan.pro { border-color: var(--gold-500); box-shadow: var(--gold-glow); }
	.plan.current { outline: 1px solid var(--primary-400); }
	h2 { margin: 0; font-family: var(--font-display); }
	.price { font-size: 30px; font-family: var(--font-display); margin: 0; }
	.price .muted { font-size: 13px; }
	ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; color: var(--dim); }
	li::before { content: "— "; color: var(--surface-600); }
	li.gold { color: var(--gold-300); }
	button:disabled { opacity: 0.55; cursor: default; }
	.mono { font-family: var(--font-mono); }
</style>
