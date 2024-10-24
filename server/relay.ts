import type { Serve, Server } from "bun";
import { type ServerWebSocket } from "bun";
import { createBunWebSocket } from "hono/bun";
import { clients } from ".";
import { Hono } from "hono";
import { cors } from "hono/cors";
import { HTTPException } from "hono/http-exception";
import { logger } from "hono/logger";
import { authHandler, verifyAuth } from "@hono/auth-js";
import { AuthConfig } from "./auth";
import { prisma } from "./db";
import { getCookie, setCookie } from "hono/cookie";

const { upgradeWebSocket, websocket } = createBunWebSocket<ServerWebSocket>();

const relay = new Hono();

relay.use(logger());
relay.use(cors());

relay.use("*", AuthConfig);

relay.get("/", (c) => {
  const sessionToken = getCookie(c, "authjs.session-token");

  if (!sessionToken) {
    return new Response('Server is up and running!', {
      status: 200
    })
  } else {
    setCookie(c, "authjs.session-token", sessionToken);
    return c.redirect(process.env.REDIRECT_URL!);
  }
});

relay.use("/api/auth/*", authHandler());

relay.use("/api/*", verifyAuth());

relay.get("/api/sessionData", async (c) => {
  const auth = c.get("authUser");
  return c.json(auth);
});

relay.get("/api/ws-session", (c) => {
  const url = new URL(c.req.url);
  return c.html(`
    <script>
  function startWebsocket() {

      var ws
    try {
        ws = new WebSocket("ws://${url.host}/api/relay");
    }catch (e) {
        setTimeout(startWebsocket, 1000);
    }
  

    ws.onmessage = function (event) {
      console.log(JSON.parse(event.data));
    };

    ws.onclose = function (event) {
      // connection closed, discard old websocket and create a new one in 5s
      ws = null;
      startWebsocket()
    };
  }

  startWebsocket();
</script>

<h1>WebSocket Session</h1>

<p>This is a WebSocket session.</p>

<p>Open JavaScript console to see messages.</p>

    `);
});

relay.get(
  "/api/relay",
  upgradeWebSocket(async (c) => {
    const auth = c.get("authUser");

    if (!auth.user || !auth.user.id) {
      throw new HTTPException(401, { message: "Unauthorized" });
    }

    const user = await prisma.user.findUnique({
      where: {
        id: auth.user.id,
      },
    });

    if (!user) {
      throw new HTTPException(401, { message: "Unauthorized" });
    }

    return {
      onOpen(evt, ws) {
        ws.send(
          JSON.stringify({
            message: `Connected to Server as ${user.name} with Subdomain ${user.subdomain}`,
          })
        );

        const currentConnections = [ws];
        if (clients.has(auth.user!.id)) {
          const existingConnections = clients.get(auth.user!.id);

          if (existingConnections) {
            currentConnections.push(...existingConnections);
          }
        }

        clients.set(user.subdomain, currentConnections);
      },
      onMessage(evt, ws) {
        console.log(evt);
        ws.send(JSON.stringify({ message: "Message Received" }));
      },
      onError: (evt, ws) => {
        console.log(evt);
      },
      onClose: () => {
        console.log("Connection closed");
      },
    };
  })
);

export const RelayHandler: Serve = {
  port: 4200,
  fetch: relay.fetch,
  // @ts-expect-error - This is a type bug
  websocket,
};

export function relayReady(server: Server) {
  console.log(`Relay Server started:
    
    Server:             ${server.url.href}

    Login:              ${server.url.href}api/auth/signin

    Logout:             ${server.url.href}api/auth/signout

    User Data Endpoint: ${server.url.href}api/sessionData

    WebSocket Server:   ws://${server.url.host}
    
    WebSocket Test:     ${server.url.href}api/ws-session
  `);
}
