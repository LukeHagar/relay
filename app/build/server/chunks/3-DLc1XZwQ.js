import { r as redirect } from './index-Djsj11qr.js';

const load = async ({ locals }) => {
  const session = await locals.auth();
  if (!session) throw redirect(302, "/api/auth/signin");
  return {};
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
  __proto__: null,
  load: load
});

const index = 3;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-C-0VI3qo.js')).default;
const server_id = "src/routes/app/+page.server.ts";
const imports = ["_app/immutable/nodes/3.9Ps6RpMI.js","_app/immutable/chunks/Bzak7iHL.js","_app/immutable/chunks/ANfDJWEW.js","_app/immutable/chunks/BGrfIEkZ.js","_app/immutable/chunks/B_wF2PSO.js"];
const stylesheets = [];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=3-DLc1XZwQ.js.map
