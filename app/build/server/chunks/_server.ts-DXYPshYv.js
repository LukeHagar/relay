import { g as getPrisma, b as broadcastToSubdomain } from './ws-Bj5P3Cj2.js';
import 'ws';
import 'jose';

const GET = async ({ request, url }) => {
  const [subdomain] = url.hostname.split(".");
  if (!subdomain) return new Response("Missing subdomain", { status: 400 });
  const prisma = await getPrisma();
  const user = await prisma.user.findUnique({ where: { subdomain } });
  if (!user) return new Response("Invalid subdomain", { status: 404 });
  const bodyText = await request.text();
  const headers = Object.fromEntries(request.headers);
  const message = {
    userId: user.id,
    method: request.method,
    path: url.pathname,
    query: url.search,
    body: bodyText,
    headers: JSON.stringify(headers),
    createdAt: /* @__PURE__ */ new Date()
  };
  await prisma.webhookEvent.create({ data: message }).catch(() => null);
  await broadcastToSubdomain(subdomain, message);
  return new Response(JSON.stringify({ ok: true }), { status: 200 });
};
const POST = GET;
const PUT = GET;
const DELETE = GET;
const PATCH = GET;
const OPTIONS = GET;

export { DELETE, GET, OPTIONS, PATCH, POST, PUT };
//# sourceMappingURL=_server.ts-DXYPshYv.js.map
