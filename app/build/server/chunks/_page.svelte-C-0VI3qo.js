import { t as push, G as ensure_array_like, J as attr_class, z as escape_html, v as pop, K as clsx } from './index2-X58sktJS.js';

function _page($$payload, $$props) {
  push();
  let events = [];
  const each_array = ensure_array_like(events);
  $$payload.out.push(`<div class="min-h-dvh p-6"><div class="flex items-center justify-between mb-6"><h2 class="text-2xl font-semibold">Live Events</h2> <div class="flex items-center gap-3"><span${attr_class(clsx("text-rose-600"))}>${escape_html("Disconnected")}</span> <button class="px-3 py-1.5 rounded bg-black text-white">Reconnect</button></div></div> <div class="grid gap-3"><!--[-->`);
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let e = each_array[$$index];
    $$payload.out.push(`<div class="rounded border p-3 text-sm"><div class="font-medium">${escape_html(e.method)} ${escape_html(e.path)}${escape_html(e.query)}</div> <div class="opacity-70">${escape_html(new Date(e.createdAt).toLocaleString())}</div> <pre class="whitespace-pre-wrap mt-2 text-xs bg-gray-50 p-2 rounded">${escape_html(e.body)}</pre></div>`);
  }
  $$payload.out.push(`<!--]--></div></div>`);
  pop();
}

export { _page as default };
//# sourceMappingURL=_page.svelte-C-0VI3qo.js.map
