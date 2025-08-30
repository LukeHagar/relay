import { j as json } from './index-Djsj11qr.js';
import { importPKCS8, SignJWT } from 'jose';

const GET = async ({ locals }) => {
  const session = await locals.auth();
  if (!session?.user?.id || !session.user.subdomain) {
    return new Response("Unauthorized", { status: 401 });
  }
  const privateKeyPem = process.env.WS_JWT_PRIVATE_KEY;
  if (!privateKeyPem) return new Response("Server key missing", { status: 500 });
  const key = await importPKCS8(privateKeyPem, "ES256");
  const token = await new SignJWT({ uid: session.user.id, subdomain: session.user.subdomain }).setProtectedHeader({ alg: "ES256" }).setIssuedAt().setExpirationTime("30m").sign(key);
  return json({ token });
};

export { GET };
//# sourceMappingURL=_server.ts-BODKK2S1.js.map
