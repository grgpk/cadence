# Plan 29: webhooks (Business)

Status: planned. Gated by `webhooks` from Plan 27.

## Approach

- Tables:
  - `webhook_endpoints`: id, `host_unid`, url, secret, events[], `is_active`
  - `webhook_deliveries`: a queue with attempts, `next_attempt_at`, `last_status` and `last_error`
- Events are `booking.created`, `booking.rescheduled` and `booking.cancelled`. They are written in the same transaction that writes the booking emails.
- A worker next to the email worker POSTs the JSON payload. The `Cadence-Signature: t=..,v1=HMAC` header is HMAC-SHA256 with the endpoint secret, the same scheme we verify for Stripe. Failed deliveries retry with exponential backoff, up to 8 times.
- UI: Apps → "Webhooks" app page, with endpoints, a "send test event" button and recent deliveries.

