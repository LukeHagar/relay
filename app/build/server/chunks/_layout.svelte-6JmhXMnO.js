import { t as push, x as slot, y as bind_props, v as pop } from './index2-X58sktJS.js';

function _layout($$payload, $$props) {
  push();
  let data = $$props["data"];
  data.session;
  $$payload.out.push(`<!---->`);
  slot($$payload, $$props, "default", {});
  $$payload.out.push(`<!---->`);
  bind_props($$props, { data });
  pop();
}

export { _layout as default };
//# sourceMappingURL=_layout.svelte-6JmhXMnO.js.map
