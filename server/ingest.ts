import type { Serve, Server } from "bun";
import { HTTPException } from "hono/http-exception";
import { clients } from "./index";
import { prisma } from "./db";
import { Hono } from "hono";
import { logger } from "hono/logger";
import { cors } from 'hono/cors'

const ingest = new Hono();

ingest.use(logger());
ingest.use(cors());

ingest.all("*", async (c) => {
  // Subdomain
  const url = new URL(c.req.url);
  const urlParts = url.hostname.split(".");
  let subdomain;
  if (urlParts.length > 1) {
    subdomain = urlParts[0];
  }

  if (!subdomain) {
    throw new HTTPException(400, { message: "Missing Subdomain" });
  }

  // Identify user
  const user = await prisma.user.findUnique({
    where: {
      subdomain,
    },
  });

  if (!user) {
    throw new HTTPException(404, { message: "Invalid Subdomain" });
  }

  // Get Body
  let body: any;
  if (c.req.raw.body) {
    body = await c.req.json();
  }

  // Get Headers
  const headers = c.req.raw.headers.toJSON();

  // Build Message
  const message = {
    userId: user.id,
    method: c.req.method,
    path: url.pathname,
    query: url.search,
    body: JSON.stringify(body),
    headers: JSON.stringify(headers),
    createdAt: new Date(),
  };

  const messageStored = prisma.webhookEvent
    .create({
      data: message,
    })
    .then((event) => {
      return true;
    })
    .catch((e) => {
      console.log(e);
      return false;
    });

  // Broadcast the message to all WebSocket clients
  let messageSent = false;
  if (subdomain && clients.has(subdomain)) {
    try {
      clients.get(subdomain)?.forEach((ws) => {
        ws.send(JSON.stringify(message));
      });
      messageSent = true;
    } catch (e) {
      messageSent = false;
      console.log(e);
    }
  }

  return c.json(
    {
      logged: await messageStored,
      forwarded: messageSent,
      subdomain,
    },
    200
  );
});

export const IngestHandler: Serve = { port: 4000, fetch: ingest.fetch };

export function ingestReady(server: Server) {
  console.log(`Ingest Server started:
    
    Ingest Server:      ${server.url.href}

    Test Ingest URL:    http://${process.env.GITHUB_USER_ID}.${server.url.host}
  `);
}
