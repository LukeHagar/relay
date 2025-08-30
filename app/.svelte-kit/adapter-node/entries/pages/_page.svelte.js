import "clsx";
import { v as pop, t as push } from "../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../chunks/exports.js";
import "../../chunks/utils.js";
import "../../chunks/state.svelte.js";
function _page($$payload, $$props) {
  push();
  $$payload.out.push(`<div class="min-h-dvh grid place-items-center p-8"><div class="max-w-xl w-full space-y-6 text-center"><h1 class="text-3xl font-bold">Baton Webhook Relay</h1> <p class="text-muted-foreground">SvelteKit 2 + Svelte 5 + Tailwind 4</p> <button class="px-4 py-2 rounded bg-black text-white">Open App</button></div></div>`);
  pop();
}
export {
  _page as default
};
