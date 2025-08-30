import { getPrisma } from '$lib/server/prisma';
import { broadcastToSubdomain } from '$lib/server/ws';

function resolveSubdomain(request: Request, url: URL) {
	const parts = url.hostname.split('.');
	if (parts.length > 1) return parts[0];
	const header = request.headers.get('x-baton-subdomain') || request.headers.get('x-subdomain');
	if (header) return header;
	const param = new URLSearchParams(url.search).get('subdomain');
	if (param) return param;
	return undefined;
}

export const GET = async ({ request, url }) => {
	const subdomain = resolveSubdomain(request, url);
	if (!subdomain) return new Response('Missing subdomain', { status: 400 });
	const prisma = await getPrisma();
	const user = await prisma.user.findUnique({ where: { subdomain } });
	if (!user) return new Response('Invalid subdomain', { status: 404 });

	const bodyText = await request.text();
	const headers = Object.fromEntries(request.headers);
	const message = {
		userId: user.id,
		method: request.method,
		path: url.pathname,
		query: url.search,
		body: bodyText,
		headers: JSON.stringify(headers),
		createdAt: new Date()
	};

	await prisma.webhookEvent.create({ data: message }).catch(() => null);
	await broadcastToSubdomain(subdomain, message);

	return new Response(JSON.stringify({ ok: true }), { status: 200 });
};

export const POST = GET;
export const PUT = GET;
export const DELETE = GET;
export const PATCH = GET;
export const OPTIONS = GET;

