# Using Vercel's emulate as an offline event source

Relay captures and replays **real-shaped** webhook traffic.
[emulate](https://github.com/vercel-labs/emulate) — Vercel Labs' local, stateful
emulation of provider APIs (GitHub, Stripe, Slack, …) — is the cleanest way to produce
that traffic offline: instead of hand-crafting payloads, you drive a faithful provider
stand-in and it delivers *actual provider-signed webhooks* over HTTP. The two are
complementary layers: emulate manufactures authentic events; relay captures, inspects,
templates, and re-fires them.

## Why

- **Provider-faithful and signed.** emulate emits the real header set —
  `X-Hub-Signature-256` for GitHub, `Stripe-Signature: t=<unix>,v1=<hex>` for Stripe —
  computed over the exact bytes it transmits. Signature-verification code sees
  production-shaped input.
- **Offline and repeatable.** No network, tunnels, or rate limits; state is in-memory
  and resets cleanly between runs.
- **Ideal capture/replay input.** Every delivery lands in a relay channel as a
  full-fidelity envelope (docs/api.md §4), ready to inspect, save as a template, and
  replay anywhere — correctly re-signed.

## Quickstart

Start emulate with the services you need and aim their webhooks at a relay capture URL.

1. Create matching channels on the relay control plane (defaults: control `:8000`,
   capture `:8001`):

   ```bash
   curl -X PUT localhost:8000/api/channels/github
   curl -X PUT localhost:8000/api/channels/stripe
   ```

2. Seed emulate with webhook routes pointed at the capture plane
   (`emulate.config.yaml`; key reference in the
   [configuration docs](https://emulate.dev/docs/configuration)):

   ```yaml
   github:
     apps:
       - app_id: 12345
         slug: relay-dev
         name: Relay Dev App
         webhook_url: http://127.0.0.1:8001/c/github   # deliveries land in relay
         webhook_secret: dev-github-secret             # same secret goes into your relay signing profile
         events: [push, pull_request]

   stripe:
     webhooks:
       - url: http://127.0.0.1:8001/c/stripe
         events: [checkout.session.completed]
         secret: dev-stripe-secret
   ```

3. Run it (`--generated-secrets-file` lets emulate mint the GitHub App RSA key that
   CLI seed files require when `private_key` is omitted):

   ```bash
   npx emulate --service github,stripe --seed emulate.config.yaml \
     --generated-secrets-file .emulate-secrets.json
   ```

4. Drive the emulated APIs (GitHub service defaults to `:4001`, Stripe to `:4009`) —
   push a commit, open a PR, complete a checkout session — and watch each signed
   delivery arrive in relay:

   ```bash
   curl "localhost:8000/api/channels/stripe/events"
   ```

### Signatures line up with relay §6.1

| Provider | emulate sends | relay scheme |
|---|---|---|
| GitHub   | `X-Hub-Signature-256: sha256=<hex>` over the raw body | `github` |
| Stripe   | `Stripe-Signature: t=<unix>,v1=<hex>`, HMAC over `<t>.<body>` | `stripe` |

emulate's Stripe scheme matches relay's ([§6.1](api.md)) byte-for-byte:
`HMAC-SHA256(secret, "<t>.<body>")`. A captured delivery therefore replays correctly
signed with a `stripe`-scheme profile holding the same secret. Put those secrets in the
engine config — never on the wire:

```json
{ "signing_profiles": {
    "gh-dev":     { "scheme": "github", "secret_env": "DEV_GITHUB_SECRET" },
    "stripe-dev": { "scheme": "stripe", "secret_env": "DEV_STRIPE_SECRET" }
} }
```

```bash
./target/debug/relay serve --config engine-config.json
./target/debug/relay replay --target http://localhost:3000/hooks --event <EVENT_ID> --profile stripe-dev
```

## Regenerating templates from captured traffic

Bundled fixtures stay honest when they are regenerated from what providers really send,
rather than written from memory:

1. Point emulate's webhook at a scratch channel (`curl -X PUT localhost:8000/api/channels/emul-stripe`).
2. Trigger the provider flow you care about through the emulated API.
3. `GET /api/channels/emul-stripe/events` and pick the cleanest delivery.
4. Save its shape as `templates/{provider}/{name}.json`: copy method/path/headers/body,
   then replace volatile values (ids, timestamps, amounts) with `{{placeholders}}` and
   give them sensible `defaults.vars` (template format in [api.md §7](api.md)).
5. Reference it as `{provider}/{name}@{v}` in replay calls; bump `@v` when the shape changes.

## License & positioning

emulate is [Apache-2.0](https://github.com/vercel-labs/emulate/blob/main/LICENSE),
maintained by Vercel Labs (`vercel-labs/emulate`). Per [PLAN.md](../PLAN.md) decision
**D9**, emulate is a complementary *source*, not a dependency: this page is the entire
integration — a docs recipe with no engine coupling, wrapping, or version pinning on
relay's side. Revisit trigger: if emulate matures and there is demand for a one-command
offline mode, add opt-in application-level integration (M3+).
