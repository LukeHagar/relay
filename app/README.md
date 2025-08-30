# Baton SvelteKit App (Svelte 5, SvelteKit 2, Tailwind 4)

This app integrates a webhook ingest with a WebSocket relay.

Scripts:

- dev: vite dev
- build: vite build
- preview: vite preview

Environment:

- Copy `.env.example` to `.env` and fill values.

WebSockets:

- WS server runs separately (default ws://localhost:4210)
- Client fetches `/api/ws-token` to obtain a short-lived JWT
- Connects with `ws://host:WS_PORT/?token=...`

Ingest:

- POST any webhook to `https://{subdomain}.your-host/ingest`.
- Event is stored in `WebhookEvent` and broadcast to connected clients for that subdomain.