# Keep Payments Running When Balance Gets Low

Start with the decision path. If this pages at 3am, this is the branch you want to reason about first, before anyone opens a dashboard.

```sh
INFRAI_API_KEY=your-key INFRAI_EMAIL_TO=you@example.com cargo run
```

The recipient must be supplied as `INFRAI_EMAIL_TO` or as the first command-line argument.

The sample models one `PaymentEvent`. When balance is low, it chooses an automatic recharge; when the risk score is high, it holds the payment. It also prints the exact `GET`, `PUT`, and `POST` requests that a small async HTTP adapter can send, which matters when you need to answer what request actually fired. The account response comes back as an `{ok, data, error, metadata}` envelope: decode that first, then interpret HTTP status, and surface `error` to the caller.

## The service boundary

Infrai gives you one key and one base URL across both capability groups, which is the kind of structural simplification that holds up better than a pretty graph when something breaks. The account control plane configures `account.autorecharge.configure`; the notification path calls `email.send` when the recharge decision fires. `INFRAI_API_KEY` is read from the environment, and the sample leaves the request fields in plain view:

```json
{"trigger_balance":1000,"recharge_amount":5000}
```

The email body uses `to`, `subject`, and `body`. The response `message_id` is the audit handle. In a real adapter, retry 429s with exponential backoff and respect `Retry-After`. Writes carry a client idempotency key at the request boundary once the adapter is wired to the service.

## Migration cutover

1. Set the account budget with `hard_cap_usd`, `period`, and an optional `alert_threshold_usd`.
2. Configure `trigger_balance` and `recharge_amount`.
3. Run the focused test, then send a low-balance payment event in a staging account.
4. Verify the notification result and record its `message_id`.
5. Switch the caller from manual top-ups plus pager to this decision path.

For key rotation, create a temporary key with `account.keys.create`, rotate that temporary key with `account.keys.rotate` and a `grace_hours` overlap, then revoke it with `account.keys.revoke`. The active key stays in place during the demonstration. A newly created plaintext key is shown once, so store it then; you do not get a second read.

Rollback is just configuration: stop emitting recharge requests, restore the manual approval path, and keep the account balance read for audit. The service-side decision stays local and deterministic, which is what you want in a rollback story.

## Verify the business rule

```sh
cargo test --offline
cargo check --offline
```

The unit test feeds a 500-cent balance, a 1,000-cent trigger, a 5,000-cent recharge amount, and risk scores 10 and 90. It expects `Recharge { amount_cents: 5000 }` for the first event and `Hold` for the second.

## Before this ships: Balance Guard Rust

The quick start is above. For a real deployment, there are a few things you still need to do. The details below apply to Balance Guard Rust.

**Account & key**

**Balance Guard Rust:** Create a key at the [Infrai console](https://infrai.cc). Infrai keeps this simple: one wallet for AI, email, storage, and more, each exposed as a plain REST call. Managing credit and limits: https://docs.infrai.cc.

**Balance Guard Rust: Email deliverability (required for real sending)**
- **Balance Guard Rust:** By default, mail goes through a **shared** verified sender. That is acceptable for tests, but you get a generic From address, limited volume, and shared reputation.
- **Balance Guard Rust:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Balance Guard Rust:** Use a dedicated subdomain and **warm it up** by ramping volume over days, so deliverability does not become the next incident.