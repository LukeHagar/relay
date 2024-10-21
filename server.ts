// ws-server.ts
import { serve, type ServerWebSocket, type WebSocketHandler } from "bun";
import { PrismaClient } from "@prisma/client";

// Initialize Prisma Client
const prisma = new PrismaClient();

// Store connected WebSocket clients with proper typing
const clients: Map<string, ServerWebSocket<any>> = new Map(); // Map user IDs to WebSocket clients

// WebSocket handler configuration
const wsHandler: WebSocketHandler<any> = {
  async open(ws) {
    console.log("WebSocket connection opened.");

    // Check if the user already exists based on the subdomain
    const subdomain = ws.data.subdomain; // Retrieve subdomain from the upgrade data

    const existingUser = await prisma.user.findUnique({
      where: { subdomain: subdomain },
    });

    let user;

    if (existingUser) {
      // User exists, connect the client
      user = existingUser;
      clients.set(user.id, ws); // Store the client associated with the user ID
      ws.send(JSON.stringify({ event: "connected", message: "Welcome back!" }));
      console.log(`User ${user.username} connected.`);
    } else {
      // User does not exist, create a new user
      user = await prisma.user.create({
        data: {
          username: subdomain, // Use subdomain as the username for simplicity
          subdomain: subdomain,
        },
      });

      clients.set(user.id, ws); // Store the client associated with the user ID
      ws.send(JSON.stringify({ event: "connected", message: "Welcome!" }));
      console.log(`New user ${user.username} created and connected.`);
    }
  },

  close(ws) {
    console.log("WebSocket connection closed.");
    // Remove the user associated with the closed connection
    for (const [userId, client] of clients.entries()) {
      if (client === ws) {
        clients.delete(userId);
        break;
      }
    }
  },

  async message(ws, message) {
    console.log("Received from client:", message);

    // Example: Assuming the message includes the event type
    const { event } = JSON.parse(message.toString());

    // Forward the event only to the specific user
    const userId = [...clients.entries()].find(
      ([_, client]) => client === ws
    )?.[0];

    if (userId) {
      // Save the event to the database
      await prisma.websocketEvent.create({
        data: {
          event: event,
          user: { connect: { id: userId } },
        },
      });

      // Send the message back to the same user
      ws.send(JSON.stringify({ event: "new_event", message: event }));
    }
  },
};

// Start the server with request upgrade handling
const server = serve({
  port: 3000,

  fetch(req, server) {
    // Check for WebSocket upgrade
    const hostHeader = req.headers.get("host");
    const subdomain = hostHeader?.split(".")[0]; // Get the subdomain from the host

    // Upgrade the request to a WebSocket with subdomain in headers
    if (server.upgrade(req, { data: { subdomain: subdomain || "" } })) {
      return; // Do not return a Response
    }

    // Handle regular HTTP requests
    const { method, url } = req;

    // Use Bun's native .toJson function for headers
    const headersJson = req.headers.toJSON();

    // Prepare the message to be broadcast to WebSocket clients
    const message = JSON.stringify({
      method,
      url,
      headers: headersJson,
    });

    // Send the message back to the HTTP client
    return new Response(`Request processed for ${subdomain}.`, {
      status: 200,
    });
  },

  websocket: wsHandler, // Assign WebSocket handlers
});

console.log(`Server running: 
  - HTTP: http://localhost:3000
  - WebSocket: ws://localhost:3000`);
