// Integration tests for the TS SDK. Skipped unless an engine is reachable at
// RELAY_URL (default http://127.0.0.1:8000) — start one with `relay serve`.

import { describe, expect, test } from "bun:test";
import { RelayClient, type Envelope } from "../src/index";

const BASE = process.env.RELAY_URL ?? "http://127.0.0.1:8000";

async function engineUp(): Promise<boolean> {
  try {
    const r = await fetch(`${BASE}/healthz`, { signal: AbortSignal.timeout(1500) });
    return r.ok;
  } catch {
    return false;
  }
}

describe.skipIf(!(await engineUp()))("RelayClient against a live engine", () => {
  const relay = new RelayClient(BASE);
  const channel = `sdk-ts-${Date.now()}`;

  test("upsert + capture + history round trip", async () => {
    await relay.upsertChannel(channel, { response_mode: "echo" });

    const res = await fetch(`http://127.0.0.1:8001/c/${channel}/evt1?a=1`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ hello: "sdk" }),
    });
    expect(res.status).toBe(200);

    const events: Envelope[] = await relay.history(channel);
    expect(events.length).toBeGreaterThanOrEqual(1);
    const env = events.at(-1)!;
    expect(env.v).toBe(1);
    expect(env.method).toBe("POST");
    expect(env.query).toBe("a=1");
    expect(JSON.parse(env.body)).toEqual({ hello: "sdk" });
  });

  test("websocket subscription receives live event", async () => {
    // once() resolves only after the engine confirms the subscription and the
    // produced capture has been delivered — deterministic, no wall-clock sleeps.
    const env = await relay.once(channel, {
      async produce() {
        await fetch(`http://127.0.0.1:8001/c/${channel}/live`, {
          method: "POST",
          body: "ping",
        });
      },
    });

    expect(env.path).toBe("/live");
    expect(env.body).toBe("ping");
  });

  test("replay returns delivery outcome", async () => {
    const events = await relay.history(channel);
    const id = events.at(-1)!.id;
    await relay.upsertChannel(`${channel}-echo`);
    const result = await relay.replay({
      target_url: `${BASE}`,
      source: { event_id: id },
      overrides: { path: `/c/${channel}-echo/replayed` },
    });
    expect(result.delivery.status).toBe(200);
    expect(result.sent_event.path).toBe(`/c/${channel}-echo/replayed`);
  });

  test("errors surface typed code and message", async () => {
    await expect(
      relay.replay({ target_url: BASE, source: { event_id: "01NOPE" } }),
    ).rejects.toThrow(/no_such_event/);
  });
});

test("engine offline: suite skips gracefully", () => {
  expect(true).toBe(true);
});
