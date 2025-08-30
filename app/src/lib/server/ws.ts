import { WebSocketServer, type WebSocket } from 'ws';
import { getPrisma } from '$lib/server/prisma';
import { jwtVerify, importSPKI } from 'jose';

type Subscriber = Set<WebSocket>;
const subdomainToSockets = new Map<string, Subscriber>();

function addSocket(subdomain: string, ws: WebSocket) {
	const set = subdomainToSockets.get(subdomain) ?? new Set<WebSocket>();
	set.add(ws);
	subdomainToSockets.set(subdomain, set);
}

function removeSocket(subdomain: string, ws: WebSocket) {
	const set = subdomainToSockets.get(subdomain);
	if (!set) return;
	set.delete(ws);
	if (set.size === 0) subdomainToSockets.delete(subdomain);
}

export async function broadcastToSubdomain(subdomain: string, payload: unknown) {
	const set = subdomainToSockets.get(subdomain);
	if (!set) return;
	const data = JSON.stringify(payload);
	for (const ws of set) {
		if (ws.readyState === WebSocket.OPEN) {
			ws.send(data);
		}
	}
}

let server: WebSocketServer | undefined;

export async function ensureWebSocketServer() {
	if (server) return server;
	const port = Number(process.env.WS_PORT ?? 4210);
	server = new WebSocketServer({ port });
	// ECDSA public key for verifying JWTs
	const publicKeyPem = process.env.WS_JWT_PUBLIC_KEY;
	const publicKey = publicKeyPem ? await importSPKI(publicKeyPem, 'ES256') : undefined;

	server.on('connection', async (ws, req) => {
		try {
			const url = new URL(req.url ?? '/', 'http://localhost');
			const token = url.searchParams.get('token');
			if (!token || !publicKey) {
				ws.close(1008, 'Unauthorized');
				return;
			}
			const { payload } = await jwtVerify<{
				uid: string;
				subdomain: string;
			}>(token, publicKey, { algorithms: ['ES256'] });
			const prisma = await getPrisma();
			const user = await prisma.user.findUnique({ where: { id: payload.uid } });
			if (!user || user.subdomain !== payload.subdomain) {
				ws.close(1008, 'Unauthorized');
				return;
			}
			addSocket(payload.subdomain, ws);
			ws.on('close', () => removeSocket(payload.subdomain, ws));
		} catch {
			ws.close(1011, 'Error');
		}
	});

	console.log(`WebSocket server listening on ws://localhost:${port}`);
	return server;
}

