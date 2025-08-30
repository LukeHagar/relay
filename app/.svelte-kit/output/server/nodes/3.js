import * as server from '../entries/pages/app/_page.server.ts.js';

export const index = 3;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/app/_page.svelte.js')).default;
export { server };
export const server_id = "src/routes/app/+page.server.ts";
export const imports = ["_app/immutable/nodes/3.9Ps6RpMI.js","_app/immutable/chunks/Bzak7iHL.js","_app/immutable/chunks/ANfDJWEW.js","_app/immutable/chunks/BGrfIEkZ.js","_app/immutable/chunks/B_wF2PSO.js"];
export const stylesheets = [];
export const fonts = [];
