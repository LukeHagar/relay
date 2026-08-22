// Minimal Relay client example — the wire contract is the SDK (docs/api.md).
// Native fetch + WebSocket only; each function maps onto one v1 endpoint/frame.

const CONTROL = process.env.RELAY_URL ?? "http://127.0.0.1:8000";
const CAPTURE = process.env.RELAY_CAPTURE_URL ?? "http://127.0.0.1:8001";

type Envelope = Record<string, unknown>;
type From = "newest" | "oldest" | number;

/** PUT /api/channels/{name} — idempotent create-or-update. */
export async function createChannel(name: string): Promise<Envelope> {
  const res = await fetch(`${CONTROL}/api/channels/${name}`, { method: "PUT" });
  if (!res.ok) throw new Error(`createChannel(${name}): HTTP ${res.status}`);
  return res.json();
}

/** Capture-plane URL for a channel — request anything here, relay records it. */
export const captureUrl = (name: string, suffix = ""): string =>
  `${CAPTURE}/c/${name}${suffix}`;

/** GET /ws — send a subscribe frame, receive event envelopes. Returns unsubscribe fn. */
export function subscribe(
  channel: string,
  from: From,
  onEvent: (event: Envelope) => void,
): () => void {
  const ws = new WebSocket(CONTROL.replace(/^http/, "ws") + "/ws");
  const id = crypto.randomUUID();

  ws.onopen = () =>
    ws.send(JSON.stringify({ v: 1, type: "subscribe", id, channel, from }));
  ws.onmessage = (m) => {
    const frame = JSON.parse(String(m.data));
    if (frame.type === "event") onEvent(frame.event);
    else if (frame.type === "error")
      console.error(`subscribe(${channel}) rejected:`, frame.error?.code);
  };
  return () => {
    try {
      ws.send(JSON.stringify({ v: 1, type: "unsubscribe", id, channel }));
      ws.close();
    } catch { /* already closed */ }
  };
}

/** POST /api/replay — send a stored event to a target origin, optionally re-signed. */
export async function replay(
  eventId: string,
  targetUrl: string,
  opts: { profile?: string; timeoutMs?: number } = {},
): Promise<{ sent_event: Envelope; delivery: Record<string, unknown> }> {
  const res = await fetch(`${CONTROL}/api/replay`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      target_url: targetUrl,
      source: { event_id: eventId },
      timeout_ms: opts.timeoutMs ?? 10_000,
      ...(opts.profile && { signing: { profile: opts.profile } }),
    }),
  });
  if (!res.ok) throw new Error(`replay(${eventId}): HTTP ${res.status}`);
  return res.json();
}

// Demo: create a channel, subscribe from the oldest buffered event, fire one
// webhook through the capture URL, print the envelope as it arrives.
async function main(): Promise<void> {
  await createChannel("demo");

  const unsubscribe = subscribe("demo", "oldest", (event) => {
    console.log("received:", JSON.stringify(event, null, 2));
    unsubscribe();
  });

  const ack = await fetch(captureUrl("demo"), {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ hello: "relay" }),
  }).then((r) => r.json());
  console.log("capture ack:", ack);

  // With a real app listening somewhere, re-fire the captured event, re-signed:
  // await replay(ack.event_id, "http://localhost:3000/hooks", { profile: "stripe-dev" });
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
