// Data layer: the TypeScript SDK is the single source of API access.
// This module adapts it to Svelte 5 runes (.svelte.ts) and owns connection state.

import { RelayClient, type ChannelConfig, type Envelope } from "../../../sdks/typescript/src/index";

export type { Envelope, ChannelConfig };

export interface Settings {
	controlUrl: string;
	captureUrl: string;
	token: string;
}

const SETTINGS_KEY = "relay.settings.v1";

function defaultSettings(): Settings {
	const origin = globalThis.location?.origin ?? "http://127.0.0.1:8000";
	// When served from the engine (same origin) we reuse it; standalone (file://,
	// vite dev on :5173, tauri://) fall back to the engine defaults.
	const sameOrigin = origin.startsWith("http") && /:\d+$/.test(origin) && !origin.includes("5173");
	const control = sameOrigin ? origin : "http://127.0.0.1:8000";
	return {
		controlUrl: control,
		captureUrl: control.replace(/:(\d+)$/, ":8001"),
		token: ""
	};
}

export const settings = $state<Settings>(loadSettings());

function loadSettings(): Settings {
	try {
		const raw = localStorage.getItem(SETTINGS_KEY);
		if (raw) return { ...defaultSettings(), ...JSON.parse(raw) };
	} catch {
		/* storage unavailable */
	}
	return defaultSettings();
}

export function persistSettings(next: Partial<Settings>) {
	Object.assign(settings, next);
	localStorage.setItem(SETTINGS_KEY, JSON.stringify($state.snapshot(settings)));
}

export function client(): RelayClient {
	return new RelayClient(settings.controlUrl, { token: settings.token || undefined });
}

export async function capture(channel: string, path: string, body: string): Promise<Response> {
	return fetch(`${settings.captureUrl}/c/${channel}${path}`, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body
	});
}

// ---- live subscription with reconnect

export type ConnStatus = "live" | "connecting" | "down";

export const conn = $state({ status: "connecting" as ConnStatus, detail: "" });

export interface SubscriptionHandlers {
	channel: () => string;
	from?: () => "newest" | "oldest" | number;
	onOk?: (lastSeq: number) => void;
	onEvent: (env: Envelope) => void;
	onError?: (e: { code: string; message: string }) => void;
	onClosed?: () => void;
}

/**
 * Connects with exponential-backoff reconnect and id-based dedup so a reconnect
 * that replays overlapping backlog never duplicates rows. Returns a stop function
 * whose flag is checked on every message, killing cross-channel races instantly.
 */
export function subscribeLive(h: SubscriptionHandlers): () => void {
	let attempt = 0;
	let stopped = false;
	let ws: WebSocket | null = null;
	let generation = 0;

	const connect = () => {
		if (stopped) return;
		conn.status = attempt === 0 ? "connecting" : "down";
		const gen = ++generation;
		const seen = new Set<string>();
		const wsBase = settings.controlUrl.replace(/^http/, "ws");
		ws = new WebSocket(wsBase + "/ws");

		ws.onopen = () => {
			ws.send(
				JSON.stringify({
					type: "subscribe",
					id: `ui-${gen}`,
					channel: h.channel(),
					from: h.from?.() ?? "oldest"
				})
			);
		};
		ws.onmessage = (m) => {
			if (stopped || gen !== generation) return; // stale socket
			const frame = JSON.parse(String(m.data));
			switch (frame.type) {
				case "ok":
					attempt = 0;
					conn.status = "live";
					h.onOk?.(frame.last_seq);
					break;
				case "event": {
					if (seen.has(frame.event.id)) return;
					seen.add(frame.event.id);
					if (seen.size > 2000) {
						// bounded: drop the oldest half of remembered ids
						for (const id of [...seen].slice(0, 1000)) seen.delete(id);
					}
					h.onEvent(frame.event);
					break;
				}
				case "error":
					h.onError?.(frame.error);
					break;
				case "channel_closed":
					h.onClosed?.();
					break;
			}
		};
		ws.onclose = () => {
			if (stopped || gen !== generation) return;
			conn.status = "down";
			const delay = Math.min(1000 * 2 ** attempt++, 8000);
			setTimeout(connect, delay);
		};
	};

	connect();
	return () => {
		stopped = true;
		generation++;
		ws?.close();
	};
}

export function prettyBody(env: Envelope): string {
	if (env.body_encoding === "base64") return `${env.body}\n(base64)`;
	try {
		return JSON.stringify(JSON.parse(env.body), null, 2);
	} catch {
		return env.body;
	}
}
