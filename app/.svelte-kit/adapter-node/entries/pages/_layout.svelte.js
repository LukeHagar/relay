import { w as slot, x as bind_props, v as pop, t as push } from "../../chunks/index2.js";
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
export {
  _layout as default
};
