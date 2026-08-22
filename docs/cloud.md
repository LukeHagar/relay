# Self-hosted public capture & cloud deployment (M4)

Relay's capture plane is designed to be exposed publicly so providers (Stripe,
GitHub, Shopify, …) can reach it. This guide covers the two supported shapes.
The engine is the same binary in both; only exposure differs.

## Topology recap (§1 of docs/api.md)

```
providers ──HTTPS──▶ capture plane  (untrusted, per-channel hostnames)
                          │ listener protocol (outbound WS)
you ────control plane :8000 (loopback / VPN; auth via --token)
```

- **Local mode (default):** capture on `127.0.0.1:8001`, channels addressed by path
  (`/c/{name}/…`). Nothing is exposed; providers can't reach loopback.
- **Hosted mode:** start with `--capture-zone <zone>`; the capture plane then routes by
  `Host` — one hostname per channel, `{slug}.{zone}`. The captured path is the full wire
  path (byte-faithful, nothing stripped).

## 1. Self-hosted public capture (free, fully local-first)

Requirements: a domain, a server reachable from the internet, wildcard DNS + TLS.

### DNS

One wildcard record covers every channel slug:

```
*.capture.example.com.   A     203.0.113.10
capture.example.com.     A     203.0.113.10
```

### TLS (wildcard, ACME DNS-01)

A wildcard certificate requires the DNS-01 challenge (HTTP-01 cannot cover `*`).
Caddy handles this with its DNS plugins; nginx pairs with certbot.

Caddyfile:

```caddy
{
    acme_dns cloudflare {env.CF_API_TOKEN}
}

*.capture.example.com {
    tls {dns cloudflare {env.CF_API_TOKEN}}
    reverse_proxy 127.0.0.1:8001
}
```

nginx equivalent (certbot `--dns-cloudflare`):

```nginx
server {
    listen 443 ssl;
    server_name *.capture.example.com;
    ssl_certificate     /etc/letsencrypt/live/capture.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/capture.example.com/privkey.pem;
    location / {
        proxy_pass http://127.0.0.1:8001;
        proxy_http_version 1.1;
        proxy_set_header Host $host;          # REQUIRED: channel routing reads Host
        proxy_set_header X-Forwarded-For $remote_addr;
    }
}
```

> `Host` must pass through untouched — channel resolution is host-based. Do **not**
> use `proxy_set_header Host …` rewrites other than `$host`.

### Engine

```bash
relay serve \
  --capture-zone capture.example.com \
  --capture-addr 127.0.0.1:8001 \
  --token "$(openssl rand -hex 32)" \
  --db /var/lib/relay/relay.db \
  --config /etc/relay/config.json
```

Create channels as usual (`PUT /api/channels/{slug}` on the control plane, loopback or
VPN). Each slug is immediately live at `https://{slug}.capture.example.com/…`.

### Abuse controls (§9.7)

- Per-slug token bucket (300 burst, 100 rps) and body-size caps are on by default.
- Add `auth.bearer_env` on any channel that only *your* systems may post to.
- Keep the control plane off the public internet: bind it to loopback/VPN and use
  `--token`.

## 2. Cloud capture (the SaaS shape, M4)

The hosted relay is the same engine behind the same wildcard ingress, plus:

- **Tenant mapping:** slugs are provisioned per account (`{tenant}-{channel}.{zone}`);
  the engine is unchanged — provisioning is a `PUT /api/channels/...` against the
  tenant's engine instance (or a multi-tenant router in front of a pool).
- **Device pairing:** the user's desktop app connects *out* to the cloud engine with the
  listener protocol (`GET /ws`, authenticated) — no inbound ports on the user's machine,
  identical to the tunnel agent path.
- **Faithful forwarding:** the cloud engine records the event (envelope, byte-faithful)
  and delivers it over the listener connection; optional re-injection re-sends the
  original method/path/headers/body unchanged.
- **Accounts/billing** live at the edge (auth proxy, metering), not in the engine. The
  standing exit criterion holds: every feature works self-hosted with no account.

## Hardening checklist

- [ ] Control plane behind VPN/loopback with `--token`
- [ ] `--capture-zone` set; only 443 exposed for the capture plane
- [ ] Wildcard cert via DNS-01; HTTP-01 disabled on the capture vhost
- [ ] `--max-body-bytes` tuned to your largest real provider payload
- [ ] `--db` on durable storage; backups = copy the SQLite file (WAL mode)
- [ ] Signing profiles in a root-owned config file; secrets from env/systemd creds
