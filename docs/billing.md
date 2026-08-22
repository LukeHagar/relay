# Billing (Stripe) — operator guide

Relay's billing layer implements the M4 open-core model: the engine is fully
functional on the **Free** plan; an operator upgrades *their instance* to **Pro**
through Stripe Checkout. Everything runs locally — Stripe is only contacted to create
checkout sessions and to verify signed webhooks.

## Plans

| | Free | Pro |
|---|---|---|
| Channels | 5 | unlimited |
| Forwarding rules per channel | — | unlimited |
| Price | $0 | $12/mo |

Plans are defined in `crates/engine/src/billing.rs` (`PLANS`) — the catalog is served
at `GET /api/billing/plans`.

## Enabling billing

```bash
export STRIPE_SECRET_KEY=sk_live_…            # or sk_test_… for test mode
export STRIPE_WEBHOOK_SECRET=whsec_…          # from `stripe listen` or the dashboard

relay serve \
  --billing-enabled \
  --stripe-price-pro price_123ABC \           # recurring monthly Price ID
  --stripe-webhook-secret whsec_… \           # optional here (env fallback shown above)
  --db /var/lib/relay/relay.db                # required: subscription state persists here
```

Without these flags the billing routes report `billing_disabled` and no quotas apply.

## HTTP surface

| Route | Auth | Behavior |
|---|---|---|
| `GET /api/billing/plans` | control token | catalog + current subscription |
| `GET /api/billing/subscription` | control token | current state |
| `POST /api/billing/checkout {"success_url","cancel_url"}` | control token | creates a Stripe Checkout Session, returns `{"url": …}` |
| `POST /api/billing/webhook` | **Stripe signature** | verifies + applies subscription events |

Quota enforcement (`402 Payment Required`, code `quota_exceeded`) applies when billing
is enabled and the current plan disallows the operation:

- creating a channel beyond `max_channels` (updating existing channels always works)
- configuring forwarding rules on Free

## Webhook handling

Point a Stripe webhook endpoint at `POST /api/billing/webhook`, subscribed to:

- `checkout.session.completed`
- `customer.subscription.updated`
- `customer.subscription.deleted`

The handler:

1. Verifies `Stripe-Signature` with the exact §6.1 canonical scheme
   (`t=<ts>,v1=hex(HMAC(secret,"<t>.<body>"))`) using constant-time comparison.
2. Rejects timestamps older than ±300 s (`stale_timestamp`) — replay protection.
3. Applies the event to subscription state and persists it to SQLite, so entitlements
   survive restarts.

Handled transitions:

| Event | Effect |
|---|---|
| `checkout.session.completed` | plan → Pro (from session metadata), status active, customer/subscription ids stored |
| `customer.subscription.updated` | status mapped (`active/trialing/past_due/canceled`), period end updated; `past_due`/`unpaid` lose entitlements |
| `customer.subscription.deleted` | downgraded to Free |

## Testing locally without Stripe network calls

```bash
cargo test -p relay-engine --test billing
```

Covers: signature accept/tamper/wrong-secret/stale/tolerance, all state transitions,
quota gating (402s), upgrade-via-signed-webhook unlocking quota, persistence, and the
plans endpoint. The checkout form builder is unit-tested; the live REST call requires a
real key by design.

For an end-to-end dev flow use the Stripe CLI:

```bash
stripe listen --forward-to localhost:8000/api/billing/webhook
stripe trigger checkout.session.completed
```

## Multi-tenancy note

This layer bills the **instance operator**. True per-tenant accounts/device pairing are
a deployment-layer concern (see docs/cloud.md); the engine stays single-tenant so the
self-hosted exit criterion holds unchanged.
