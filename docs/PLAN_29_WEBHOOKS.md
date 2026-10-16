# Plan 29: webhooks (Business)

Status: implemented. Gated by `webhooks` from Plan 27. Backend tests
(webhooks 4 HTTP, 11 unit; booking 47; billing 13), frontend tests (196),
build, lint and knip pass. Checked against the dev servers: a test ping went
out over HTTPS to example.com, which answered 405, so it was recorded and
retried a minute later; a Pro host's page shows the Business notice.

## Goal

A Business host's own server hears about every booked, moved and cancelled
call, so it can fill a CRM, a sheet or their own app without polling.

## Data model

`migrations/0019_create_webhooks.sql`, new tables only:

```
webhook_endpoints   unid, host_unid, url (https, ≤ 500), secret, is_active
webhook_deliveries  unid, endpoint_unid, event, payload (the event's data),
                    attempts, next_attempt_at, delivered_at, failed_at,
                    last_status, last_error
```

Every endpoint hears every event; there is no per-endpoint event filter.
Up to five endpoints per host. Deliveries sent or given up on are swept after
30 days by the email worker's sweep.

## API

```
GET    /api/host/webhooks                     the host's endpoints, with secrets
POST   /api/host/webhooks                     403 without Business, 400 for a
                                              URL that is not public https, 409
                                              past five
PUT    /api/host/webhooks/{unid}              { is_active }; turning on needs Business
DELETE /api/host/webhooks/{unid}
POST   /api/host/webhooks/{unid}/test         queues a `ping`, 202
GET    /api/host/webhooks/{unid}/deliveries   the last 20
```

## Flow

```
booking created/moved/cancelled ──▶ same transaction as its emails:
   (Business only)                   one webhook_deliveries row per active endpoint
                                                     │
webhook worker, every 10 s ──▶ claim one due row (FOR UPDATE SKIP LOCKED)
   ──▶ POST {id, event, created_at, data}, Cadence-Signature, Cadence-Event
   ──▶ 2xx: delivered │ else: retry after 1, 5, 15, 60, 180, 360, 720 min,
                        given up after the 8th try (about a day)
```

## Signature

`Cadence-Signature: t=<unix seconds>,v1=<hex HMAC-SHA256 of "t.body">`,
keyed with the endpoint's `whsec_…` secret: the scheme Stripe uses, so one
module (`backend/src/signature.rs`) signs our webhooks and verifies Stripe's.
`id` is the delivery's and stays the same across retries.

## Where a delivery may go

The host picks the URL and our server calls it, so `webhooks/target.rs`:

- saves only `https://` URLs whose host is not `localhost`, `*.local`,
  `*.internal`, `*.lan`, or a private, loopback, link-local, shared,
  documentation, multicast or reserved address (numeric tricks such as
  `https://2130706433/` are normalised to `127.0.0.1` and refused);
- on every delivery resolves the name, refuses it if any address is not
  public, and pins the request to the address it checked, so DNS cannot be
  switched in between;
- follows no redirects; 10-second timeout.

## Plan limits

Events are only queued while the host has Business. After a downgrade the
endpoints stay saved; they can be switched off or deleted but not on, and
nothing new is sent until an upgrade.

## Frontend

- Apps: a new Automation category and a Webhooks app ("With Business"), on
  its app page and on Installed apps (`domains/integrations/webhooks-panel.tsx`).
- Add a URL, switch it on or off, reveal or copy the secret, send a test,
  delete, and read recent deliveries with their state.
- `isInstalled` and `appState` take the endpoint count.

## Tests

- `tests/webhooks_http.rs`: Business and URL checks and the limit of five; a
  booking delivered with a body and signature the receiver can verify, not
  sent twice; a failing receiver retried on schedule and given up on after
  eight tries; an endpoint switched off hears nothing; another host cannot
  see, change, test or delete it.
- Unit: signature vectors (`signature.rs`), URL and address checks
  (`target.rs`), retry schedule, body and secret (`service.rs`), and the
  real sender refusing private addresses (`worker.rs`).
- Frontend: `webhooks.test.ts` (delivery state, masked secret), and the
  webhooks case of `isInstalled`.

## Definition of done

- [x] Business hosts add, test and manage webhooks from Apps.
- [x] Every booked, moved and cancelled call is delivered, signed, with retries.
- [x] A webhook cannot reach this server or its network.
- [x] Pricing page drops "Soon" from webhooks.
- [ ] Receive a real booking on a test endpoint after deploy.
