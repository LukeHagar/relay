export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set([]),
	mimeTypes: {},
	_: {
		client: {start:"_app/immutable/entry/start.BKmjMivv.js",app:"_app/immutable/entry/app.DIpoMANL.js",imports:["_app/immutable/entry/start.BKmjMivv.js","_app/immutable/chunks/D80dHVgU.js","_app/immutable/chunks/B_wF2PSO.js","_app/immutable/chunks/BGrfIEkZ.js","_app/immutable/entry/app.DIpoMANL.js","_app/immutable/chunks/BGrfIEkZ.js","_app/immutable/chunks/B_wF2PSO.js","_app/immutable/chunks/Bzak7iHL.js","_app/immutable/chunks/46J8_SRV.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js')),
			__memo(() => import('./nodes/3.js'))
		],
		remotes: {
			
		},
		routes: [
			{
				id: "/",
				pattern: /^\/$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 2 },
				endpoint: null
			},
			{
				id: "/api/auth/[...auth]",
				pattern: /^\/api\/auth(?:\/([^]*))?\/?$/,
				params: [{"name":"auth","optional":false,"rest":true,"chained":true}],
				page: null,
				endpoint: __memo(() => import('./entries/endpoints/api/auth/_...auth_/_server.ts.js'))
			},
			{
				id: "/api/ws-token",
				pattern: /^\/api\/ws-token\/?$/,
				params: [],
				page: null,
				endpoint: __memo(() => import('./entries/endpoints/api/ws-token/_server.ts.js'))
			},
			{
				id: "/app",
				pattern: /^\/app\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 3 },
				endpoint: null
			},
			{
				id: "/ingest",
				pattern: /^\/ingest\/?$/,
				params: [],
				page: null,
				endpoint: __memo(() => import('./entries/endpoints/ingest/_server.ts.js'))
			}
		],
		prerendered_routes: new Set([]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();

export const prerendered = new Set([]);

export const base = "";