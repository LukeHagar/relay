/**
 * Relay TypeScript SDK.
 *
 * A thin, typed client over the wire contract in docs/api.md. Zero runtime
 * dependencies: requires global `fetch` and `WebSocket` (Node >= 22, Bun, Deno,
 * browsers).
 *
 * ```ts
 * import { RelayClient } from "./index";
 * const relay = new RelayClient("http://127.0.0.1:8000");
 * await relay.upsertChannel("stripe-dev", { response_mode: "echo" });
 * const stop = relay.subscribe("stripe-dev", "newest", console.log);
 * ```
 */

export interface Envelope {
  v: 1;
  id: string;
  channel: string;
  host?: string | null;
  seq: number;
  received_at: string;
  method: string;
  path: string;
  query?: string | null;
  headers: [string, string][];
  body: string;
  body_encoding: "utf8" | "base64";
  truncated: boolean;
  status_sent?: number | null;
  remote_addr?: string | null;
}

export type ResponseMode = "ack" | "echo" | "static";

export interface StaticResponse {
  status: number;
  headers?: [string, string][];
  body?: string;
}

export interface ChannelConfig {
  response_mode?: ResponseMode;
  static_response?: StaticResponse;
  auth?: { bearer_env: string } | null;
  buffer?: { max_events?: number; max_age_s?: number };
  rate_limit?: { burst: number; refill_per_s: number };
  forward?: { url: string; profile?: string; headers?: [string, string][] }[];
}

export type FromSpec = "newest" | "oldest" | number;

export interface ForwardRule {
  url: string;
  profile?: string;
  headers?: [string, string][];
}

export interface ChannelDescriptor {
  name: string;
  created_at: string;
  response_mode?: ResponseMode;
  buffer?: { max_events?: number; max_age_s?: number };
  auth?: { bearer_env: string } | null;
  rate_limit?: { burst: number; refill_per_s: number } | null;
  forward?: ForwardRule[] | null;
  capture_url_local?: string;
  capture_url_hosted?: string | null;
}

export interface Plan {
  id: string;
  name: string;
  max_channels: number;
  price_monthly_usd: number;
}

export interface SubscriptionState {
  plan_id: string;
  status: "active" | "trialing" | "past_due" | "canceled";
  stripe_customer_id?: string;
  stripe_subscription_id?: string;
  current_period_end_unix?: number;
}

/** Server → client WS frames, discriminated on `type` (docs/api.md §5.1). */
export type ServerFrame =
  | { v: 1; type: "ok"; id: string; last_seq: number }
  | { v: 1; type: "error"; id?: string; error: { code: string; message: string } }
  | { v: 1; type: "event"; subscription: string; event: Envelope }
  | { v: 1; type: "pong"; id: string }
  | { v: 1; type: "channel_closed"; channel: string };

export function headersToMap(env: Envelope): Record<string, string> {
  return Object.fromEntries(env.headers);
}

export interface ReplayRequest {
  target_url: string;
  timeout_ms?: number;
  source: { event_id: string } | { template: string } | { inline: Record<string, unknown> };
  overrides?: {
    method?: string;
    path?: string;
    query?: string;
    headers?: [string, string][];
    body?: string;
  };
  vars?: Record<string, unknown>;
  signing?: { profile?: string; scheme?: string; secret?: string };
}

export interface Delivery {
  status: number | null;
  duration_ms: number;
  error: string | null;
}

export interface ReplayResult {
  sent_event: Envelope;
  delivery: Delivery;
}

export class RelayError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(`${code}: ${message}`);
  }
}

export interface RelayClientOptions {
  /** Bearer token for the control plane (`--token` on the engine). */
  token?: string;
  /** Capture-plane origin when it differs from the control plane. */
  captureBase?: string;
  /** fetch implementation override; defaults to globalThis.fetch. */
  fetch?: typeof fetch;
  /** WebSocket implementation override; defaults to globalThis.WebSocket. */
  WebSocket?: typeof WebSocket;
}

export class RelayClient {
  private readonly base: string;
  private readonly token?: string;
  private readonly fetchImpl: typeof fetch;
  private readonly wsImpl: typeof WebSocket;

  constructor(baseUrl = "http://127.0.0.1:8000", options: RelayClientOptions = {}) {
    this.base = baseUrl.replace(/\/+$/, "");
    this.token = options.token;
    this.fetchImpl = options.fetch ?? globalThis.fetch;
    this.wsImpl = options.WebSocket ?? globalThis.WebSocket;
    // Default: capture plane = control port + 1 (8000 → 8001).
    if (options.captureBase) {
      this.captureBase = options.captureBase.replace(/\/+$/, "");
    }
  }

  private async call<T>(path: string, init?: RequestInit): Promise<T> {
    const headers = new Headers(init?.headers);
    if (this.token) headers.set("authorization", `Bearer ${this.token}`);
    const res = await this.fetchImpl(this.base + path, { ...init, headers });
    if (!res.ok && res.status !== 201) {
      let code = String(res.status);
      let message = res.statusText;
      try {
        const body = await res.json();
        code = body.error?.code ?? code;
        message = body.error?.message ?? message;
      } catch {
        /* non-JSON error body */
      }
      throw new RelayError(res.status, code, message);
    }
    return res.json() as Promise<T>;
  }

  // ---- channels

  upsertChannel(name: string, config: ChannelConfig = {}): Promise<ChannelDescriptor> {
    return this.call(`/api/channels/${encodeURIComponent(name)}`, {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(config),
    });
  }

  listChannels(): Promise<{ channels: ChannelDescriptor[] }> {
    return this.call("/api/channels");
  }

  deleteChannel(name: string): Promise<void> {
    return this.call<void>(`/api/channels/${encodeURIComponent(name)}`, { method: "DELETE" });
  }

  async history(
    name: string,
    opts: { cursor?: number; limit?: number } = {},
  ): Promise<Envelope[]> {
    const q = new URLSearchParams();
    if (opts.cursor !== undefined) q.set("cursor", String(opts.cursor));
    if (opts.limit !== undefined) q.set("limit", String(opts.limit));
    const data = await this.call<{ events: Envelope[] }>(
      `/api/channels/${encodeURIComponent(name)}/events?${q}`,
    );
    return data.events;
  }

  captureUrl(channel: string): string {
    // Capture plane shares the host by default; pass an explicit origin when the
    // planes are split across ports/hosts.
    return `${this.captureBase ?? this.base}/c/${channel}`;
  }

  /** Override when the capture plane lives on a different origin than the control plane. */
  captureBase?: string;

  // ---- subscriptions

  /**
   * Subscribe over WebSocket with backlog-then-live delivery (no gaps, no
   * duplicates at the seam — docs/api.md §5.1). Returns a stop function.
   */
  subscribe(
    channel: string,
    from: FromSpec,
    onEvent: (env: Envelope) => void,
    onError?: (err: { code: string; message: string }) => void,
  ): () => void {
    const wsBase = this.base.replace(/^http/, "ws");
    const ws = new this.wsImpl(wsBase + "/ws");
    const corrId = `sdk-${Math.random().toString(36).slice(2)}`;
    ws.onopen = () =>
      ws.send(JSON.stringify({ type: "subscribe", id: corrId, channel, from }));
    ws.onmessage = (m) => {
      const frame: ServerFrame = JSON.parse(String(m.data));
      switch (frame.type) {
        case "event":
          onEvent(frame.event);
          break;
        case "error":
          onError?.(frame.error);
          break;
        case "channel_closed":
          ws.close();
          break;
      }
    };
    return () => ws.close();
  }

  /**
   * Await the next live event on a channel as a promise. When `produce` is given,
   * it runs only after the engine confirms the subscription ("ok" frame), so the
   * produced delivery can never be missed.
   */
  once(
    channel: string,
    options: { from?: FromSpec; produce?: () => void | Promise<void> } = {},
  ): Promise<Envelope> {
    const { from = "newest", produce } = options;
    return new Promise((resolve, reject) => {
      const wsBase = this.base.replace(/^http/, "ws");
      const ws = new this.wsImpl(wsBase + "/ws");
      const corrId = `sdk-${Math.random().toString(36).slice(2)}`;
      ws.onerror = () => reject(new RelayError(0, "websocket_error", "socket failed before event"));
      ws.onmessage = (m) => {
        const frame = JSON.parse(String(m.data));
        if (frame.type === "ok") {
          produce?.();
        } else if (frame.type === "event") {
          ws.close();
          resolve(frame.event);
        } else if (frame.type === "error") {
          ws.close();
          reject(new RelayError(400, frame.error.code, frame.error.message));
        }
      };
      ws.onopen = () =>
        ws.send(JSON.stringify({ type: "subscribe", id: corrId, channel, from }));
    });
  }

  // ---- replay

  replay(req: ReplayRequest): Promise<ReplayResult> {
    return this.call("/api/replay", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(req),
    });
  }

  // ---- billing

  billingPlans(): Promise<{
    billing_enabled: boolean;
    plans: Plan[];
    subscription: SubscriptionState;
  }> {
    return this.call("/api/billing/plans");
  }

  billingCheckout(successUrl: string, cancelUrl: string): Promise<{ url: string }> {
    return this.call("/api/billing/checkout", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ success_url: successUrl, cancel_url: cancelUrl })
    });
  }

  health(): Promise<{ ok: boolean; version: string }> {
    return this.call("/healthz");
  }

  ready(): Promise<{ ready: boolean }> {
    return this.call("/readyz");
  }

  // ---- templates & signing (UI-facing metadata)

  listTemplates(): Promise<{
    templates: {
      ref: string;
      provider: string;
      name: string;
      description: string;
      signing_hint?: string;
      defaults: { vars: Record<string, unknown> };
    }[];
  }> {
    return this.call("/api/templates");
  }

  listSigningProfiles(): Promise<{ profiles: string[] }> {
    return this.call("/api/signing-profiles");
  }
}
