<script lang="ts">
	import "../app.css";
	import { conn, settings } from "$lib/api.svelte";
	import { page } from "$app/state";

	let { children } = $props();

	const nav = [
		{ href: "/", label: "Inbox", icon: "📥" },
		{ href: "/channels", label: "Channels", icon: "📡" },
		{ href: "/replay", label: "Replay", icon: "▶" },
		{ href: "/templates", label: "Templates", icon: "🗂" },
		{ href: "/billing", label: "Billing", icon: "💳" },
		{ href: "/settings", label: "Settings", icon: "⚙" }
	];
</script>

<div class="shell">
	<aside>
		<div class="brand">
			<span class="bolt">⚡</span>
			<span class="word">relay</span>
			<span class="badge" class:online={conn.status === "live"} class:offline={conn.status === "down"}>
				<span class="dot"></span>{conn.status.toUpperCase()}
			</span>
		</div>
		<nav>
			{#each nav as item (item.href)}
				<a href={item.href} class:active={page.url.pathname === item.href}>
					<span class="icon">{item.icon}</span>{item.label}
				</a>
			{/each}
		</nav>
		<div class="foot">
			<div
				class="badge"
				class:online={conn.status === "live"}
				class:offline={conn.status === "down"}
			>
				<span class="dot"></span>{conn.status.toUpperCase()}
			</div>
			<p class="muted mono small">{settings.controlUrl}</p>
		</div>
	</aside>
	<main>
		{@render children()}
	</main>
</div>

<style>
	.shell {
		display: grid;
		grid-template-columns: 220px 1fr;
		height: 100vh;
	}
	aside {
		border-right: 1px solid var(--border);
		background: var(--surface-950);
		display: flex;
		flex-direction: column;
		padding: 16px 12px;
		gap: 18px;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 8px;
	}
	.bolt {
		filter: drop-shadow(0 0 6px rgba(255, 217, 122, 0.5));
	}
	.word {
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 18px;
		color: var(--primary-300);
	}
	nav {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	nav a {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--dim);
		padding: 8px 10px;
		border-radius: 8px;
		border: 1px solid transparent;
	}
	nav a:hover {
		color: var(--text);
		background: var(--surface-900);
	}
	nav a.active {
		color: var(--primary-200);
		background: var(--surface-800);
		border-color: var(--surface-600);
	}
	.foot {
		margin-top: auto;
		display: grid;
		gap: 6px;
		padding: 0 8px;
	}
	.small {
		font-size: 11px;
		word-break: break-all;
	}
	main {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
</style>
