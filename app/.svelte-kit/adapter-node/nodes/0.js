import * as server from '../entries/pages/_layout.server.ts.js';

export const index = 0;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_layout.svelte.js')).default;
export { server };
export const server_id = "src/routes/+layout.server.ts";
export const imports = ["_app/immutable/nodes/0.CArEQLZI.js","_app/immutable/chunks/Bzak7iHL.js","_app/immutable/chunks/ANfDJWEW.js","_app/immutable/chunks/BGrfIEkZ.js","_app/immutable/chunks/46J8_SRV.js"];
export const stylesheets = ["_app/immutable/assets/0.DqrBY_Hs.css"];
export const fonts = [];
