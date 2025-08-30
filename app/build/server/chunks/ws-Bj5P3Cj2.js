import { WebSocketServer } from 'ws';
import { importSPKI, jwtVerify } from 'jose';

let prismaSingleton;
async function getPrisma() {
  if (!prismaSingleton) {
    const { PrismaClient } = await import('@prisma/client');
    prismaSingleton = new PrismaClient();
  }
  return prismaSingleton;
}
const subdomainToSockets = /* @__PURE__ */ new Map();
function addSocket(subdomain, ws) {
  const set = subdomainToSockets.get(subdomain) ?? /* @__PURE__ */ new Set();
  set.add(ws);
  subdomainToSockets.set(subdomain, set);
}
function removeSocket(subdomain, ws) {
  const set = subdomainToSockets.get(subdomain);
  if (!set) return;
  set.delete(ws);
  if (set.size === 0) subdomainToSockets.delete(subdomain);
}
async function broadcastToSubdomain(subdomain, payload) {
  const set = subdomainToSockets.get(subdomain);
  if (!set) return;
  const data = JSON.stringify(payload);
  for (const ws of set) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(data);
    }
  }
}
let server;
async function ensureWebSocketServer() {
  if (server) return server;
  const port = Number(process.env.WS_PORT ?? 4210);
  server = new WebSocketServer({ port });
  const publicKeyPem = process.env.WS_JWT_PUBLIC_KEY;
  const publicKey = publicKeyPem ? await importSPKI(publicKeyPem, "ES256") : void 0;
  server.on("connection", async (ws, req) => {
    try {
      const url = new URL(req.url ?? "/", "http://localhost");
      const token = url.searchParams.get("token");
      if (!token || !publicKey) {
        ws.close(1008, "Unauthorized");
        return;
      }
      const { payload } = await jwtVerify(token, publicKey, { algorithms: ["ES256"] });
      const prisma = await getPrisma();
      const user = await prisma.user.findUnique({ where: { id: payload.uid } });
      if (!user || user.subdomain !== payload.subdomain) {
        ws.close(1008, "Unauthorized");
        return;
      }
      addSocket(payload.subdomain, ws);
      ws.on("close", () => removeSocket(payload.subdomain, ws));
    } catch {
      ws.close(1011, "Error");
    }
  });
  console.log(`WebSocket server listening on ws://localhost:${port}`);
  return server;
}

export { broadcastToSubdomain as b, ensureWebSocketServer as e, getPrisma as g };
//# sourceMappingURL=ws-Bj5P3Cj2.js.map
