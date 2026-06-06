# CourierX Worker

The queue consumer for [CourierX](https://courierx.io) — an open-source, self-hosted email service.

This worker polls the `emails` table in Postgres, picks up `queued` jobs using `FOR UPDATE SKIP LOCKED`, and delivers them via a configurable provider (stdout for local dev; SES, Resend, or SMTP for production).

Designed to run alongside one or more API instances — the API writes, the worker delivers.

## Stack

- **Language:** Rust (edition 2021)
- **Runtime:** tokio
- **Database:** Postgres 16+ via SQLx 0.8
- **Queue pattern:** `FOR UPDATE SKIP LOCKED` (no Redis, no extra services)
- **Provider abstraction:** trait-based — swap `StdoutProvider` for `SesProvider` or `ResendProvider` at runtime

## How it works

The worker runs an infinite polling loop:

1. Wake up every `POLL_INTERVAL_SECS` (default: 2s)
2. Open a transaction
3. `SELECT ... FROM emails WHERE status = 'queued' FOR UPDATE SKIP LOCKED LIMIT $BATCH_SIZE`
4. Update each row's status to `sending`
5. Commit the transaction (releases the lock)
6. For each row, call `provider.send()` and update the row to `sent` or `failed`
7. On transient failures, requeue with incremented `attempts` (up to `MAX_ATTEMPTS`)

Multiple worker instances can run in parallel safely — `SKIP LOCKED` ensures no two workers pick up the same email.

## Provider abstraction

Providers implement a single trait:

```rust
#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, email: &Email) -> Result<String, ProviderError>;
}
```

`Ok(message_id)` marks the email as `sent`. `Err(ProviderError::Transient(_))` triggers a retry. `Err(ProviderError::Permanent(_))` marks it as `failed` immediately.

Built-in providers (Phase 1):

- **StdoutProvider** — prints the email body to the terminal. For local dev and CI. Returns a fake message ID.

Planned providers (Phase 2):

- **ResendProvider** — delegates delivery to Resend's API
- **SesProvider** — direct AWS SES integration
- **SmtpProvider** — plain SMTP via `lettre`

## Local development

### Prerequisites

- Rust 1.85+
- Postgres 16+ running locally (same database as `courierx-api`)
- `courierx-api` must be set up first — it owns the schema and runs migrations

### Setup

```bash
git clone https://github.com/Miransas/courierx-worker.git
cd courierx-worker

# Copy the example env, then edit as needed
cp .env.example .env

# Run the worker
cargo run
```

The worker waits for the `emails` table to exist before starting. If you launch it before the API has run migrations, it logs a warning and polls every 5 seconds until the schema is ready.

### Configuration

`.env`:

```env
DATABASE_URL=postgres://courierx:courierx@localhost:5432/courierx
POLL_INTERVAL_SECS=2
BATCH_SIZE=10
MAX_ATTEMPTS=3
PROVIDER=stdout
RUST_LOG=courierx_worker=debug
```

## Schema ownership

The worker does **not** run migrations. The schema is owned by [courierx-api](https://github.com/Miransas/courierx-api). The worker reads and writes to existing tables but does not modify the schema. This makes worker deploys safe even mid-migration.

A minimum schema version (`MIN_SCHEMA_VERSION`) is checked on startup. If the schema is behind, the worker waits without exiting.

## Other repos in the CourierX project

- [courierx-web](https://github.com/sardorazimov/courierx-web) — the marketing site at courierx.io
- [courierx-api](https://github.com/Miransas/courierx-api) — the HTTP API
- `courierx-console` — the dashboard (private until launch)

## Brand

CourierX is built under the [Miransas](https://miransas.com) brand by [@sardorazimov](https://github.com/sardorazimov).

## License

MIT — see [LICENSE](./LICENSE)
