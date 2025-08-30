<script>
  import { onMount } from 'svelte';
  let events = [];
  let connected = false;

  async function connect() {
    const res = await fetch('/api/ws-token');
    if (!res.ok) return;
    const { token } = await res.json();
    const wsPort = Number(import.meta.env.VITE_WS_PORT ?? 4210);
    const ws = new WebSocket(`ws://${location.hostname}:${wsPort}/?token=${token}`);
    ws.onopen = () => (connected = true);
    ws.onmessage = (e) => {
      try { events = [JSON.parse(e.data), ...events].slice(0, 200); } catch {}
    };
    ws.onclose = () => (connected = false);
  }

  onMount(connect);
</script>

<div class="min-h-dvh p-6">
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-2xl font-semibold">Live Events</h2>
    <div class="flex items-center gap-3">
      <span class={connected ? 'text-emerald-600' : 'text-rose-600'}>
        {connected ? 'Connected' : 'Disconnected'}
      </span>
      <button class="px-3 py-1.5 rounded bg-black text-white" on:click={connect}>Reconnect</button>
    </div>
  </div>

  <div class="grid gap-3">
    {#each events as e}
      <div class="rounded border p-3 text-sm">
        <div class="font-medium">{e.method} {e.path}{e.query}</div>
        <div class="opacity-70">{new Date(e.createdAt).toLocaleString()}</div>
        <pre class="whitespace-pre-wrap mt-2 text-xs bg-gray-50 p-2 rounded">{e.body}</pre>
      </div>
    {/each}
  </div>
</div>

