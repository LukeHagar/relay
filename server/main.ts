import { Server } from "socket.io";
import { PrismaClient } from "@prisma/client";
import * as http from "http";

// Initialize Prisma client
const prisma = new PrismaClient();

// User tokens (for example purposes)
const userTokens: Record<string, string> = {
  token1: "user1",
  token2: "user2",
};

// WebSocket clients map
const clients: Record<string, Set<string>> = {};

// Initialize HTTP server
const httpServer = http.createServer((req, res) => {
  if (req.url === "/user/events" && req.method === "GET") {
    getUserEvents(req, res);
  } else if (req.url === "/user/clear" && req.method === "POST") {
    clearUserEvents(req, res);
  } else {
    handleWebhook(req, res);
  }
});

// Initialize Socket.IO server
const io = new Server(httpServer, { path: "/ws" });

// Middleware for Socket.IO authentication
io.use((socket, next) => {
  const token = socket.handshake.auth.token;
  const userID = userTokens[token];
  if (!userID) {
    return next(new Error("Unauthorized"));
  }
  socket.data.userID = userID;
  next();
});

// WebSocket connection handler
io.on("connection", (socket) => {
  const userID = socket.data.userID as string;
  console.log(`User ${userID} connected`);

  if (!clients[userID]) {
    clients[userID] = new Set();
  }
  clients[userID].add(socket.id);

  socket.on("disconnect", () => {
    console.log(`User ${userID} disconnected`);
    clients[userID].delete(socket.id);
    if (clients[userID].size === 0) {
      delete clients[userID];
    }
  });
});

// Broadcast message to a specific user's clients
function broadcastMessage(userID: string, message: any) {
  if (clients[userID]) {
    clients[userID].forEach((socketID) => {
      io.to(socketID).emit("message", message);
    });
  }
}

// Handle incoming webhook requests and store events
async function handleWebhook(req: http.IncomingMessage, res: http.ServerResponse) {
  let body = "";
  req.on("data", (chunk) => (body += chunk));
  req.on("end", async () => {
    const host = req.headers.host || "";
    const userID = extractUserIDFromHost(host);
    if (!userID || !Object.values(userTokens).includes(userID)) {
      res.writeHead(400);
      return res.end("Invalid user ID");
    }

    const headers = req.headers;

    try {
      // Save the event to the database
      await prisma.webhookEvent.create({
        data: {
          userId: userID,
          method: req.method || "",
          url: req.url || "",
          headers,
          body,
        },
      });

      broadcastMessage(userID, { method: req.method, url: req.url, body });
      res.writeHead(200);
      res.end("Request logged and forwarded");
    } catch (error) {
      console.error("Error saving event:", error);
      res.writeHead(500);
      res.end("Failed to save to database");
    }
  });
}

// Retrieve events for a user
async function getUserEvents(req: http.IncomingMessage, res: http.ServerResponse) {
  const token = req.headers.authorization || "";
  const userID = userTokens[token];
  if (!userID) {
    res.writeHead(401);
    return res.end("Unauthorized");
  }

  try {
    const events = await prisma.webhookEvent.findMany({
      where: { userId: userID },
    });
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify(events));
  } catch (error) {
    console.error("Error retrieving events:", error);
    res.writeHead(500);
    res.end("Failed to retrieve events");
  }
}

// Clear events for a user
async function clearUserEvents(req: http.IncomingMessage, res: http.ServerResponse) {
  const token = req.headers.authorization || "";
  const userID = userTokens[token];
  if (!userID) {
    res.writeHead(401);
    return res.end("Unauthorized");
  }

  try {
    await prisma.webhookEvent.deleteMany({
      where: { userId: userID },
    });
    res.writeHead(200);
    res.end("Events cleared");
  } catch (error) {
    console.error("Error clearing events:", error);
    res.writeHead(500);
    res.end("Failed to clear events");
  }
}

// Extract user ID from subdomain
function extractUserIDFromHost(host: string): string {
  const parts = host.split(".");
  return parts.length > 1 ? parts[0] : "";
}

// Start the HTTP and WebSocket server
const PORT = 8000;
httpServer.listen(PORT, () => {
  console.log(`HTTP server listening on http://localhost:${PORT}`);
});
