const load = async ({ locals }) => {
  const session = await locals.auth();
  return { session };
};

var _layout_server_ts = /*#__PURE__*/Object.freeze({
  __proto__: null,
  load: load
});

const index = 0;
let component_cache;
const component = async () => component_cache ??= (await import('./_layout.svelte-6JmhXMnO.js')).default;
const server_id = "src/routes/+layout.server.ts";
const imports = ["_app/immutable/nodes/0.CArEQLZI.js","_app/immutable/chunks/Bzak7iHL.js","_app/immutable/chunks/ANfDJWEW.js","_app/immutable/chunks/BGrfIEkZ.js","_app/immutable/chunks/46J8_SRV.js"];
const stylesheets = ["_app/immutable/assets/0.DqrBY_Hs.css"];
const fonts = [];

export { component, fonts, imports, index, _layout_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=0-DGK5g7Cy.js.map
