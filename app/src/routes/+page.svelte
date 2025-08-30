<script>
  import { goto } from '$app/navigation';

  async function go() {
    await goto('/app');
  }
</script>

<div class="min-h-dvh grid place-items-center p-8">
  <div class="max-w-xl w-full space-y-6 text-center">
    <h1 class="text-3xl font-bold">Baton Webhook Relay</h1>
    <p class="text-muted-foreground">SvelteKit 2 + Svelte 5 + Tailwind 4</p>
    <button class="px-4 py-2 rounded bg-black text-white" on:click={go}>Open App</button>
  </div>
  </div>

