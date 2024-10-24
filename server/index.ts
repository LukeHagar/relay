// ws-server.ts
import { type ServerWebSocket } from "bun";
import { IngestHandler, ingestReady } from "./ingest";
import { RelayHandler, relayReady } from "./relay";
import type { WSContext } from "hono/ws";

// Store connected WebSocket clients with proper typing
export const clients: Map<string, WSContext<ServerWebSocket>[]> = new Map(); // Map user IDs to WebSocket clients

const ingestServer = Bun.serve(IngestHandler);
const relayServer = Bun.serve(RelayHandler);

ingestReady(ingestServer);
relayReady(relayServer);