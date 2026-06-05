# courierx-worker

Queue consumer that delivers emails via pluggable providers. Shares the Postgres
`emails` table with `courierx-api` (schema is owned by the api repo; this repo
holds no migrations).

Currently supports:

- **stdout** — debug provider, logs the email body and returns a fake message id.

Planned:

- AWS SES
- Resend
- SMTP

## Architecture

```
[Postgres queue]  <--- poll ---  [worker]  --- send --->  [EmailProvider]
                                                            ├─ StdoutProvider (now)
                                                            ├─ ResendProvider (next)
                                                            └─ SesProvider (later)
```

The worker claims a batch of `status='queued'` rows with `FOR UPDATE SKIP
LOCKED`, flips them to `sending` (bumping `attempts`) in a single short
transaction, then calls the provider per row. Result writes back as `sent`,
re-`queued` (if transient and attempts remain), or `failed`.

Migrations are owned by `courierx-api`. Worker waits until the expected schema
version is present in the `_sqlx_migrations` table before starting.

## Run

```bash
cp .env.example .env   # adjust DATABASE_URL
cargo run
```

## Env vars

| Var                  | Default                                                | Notes                              |
| -------------------- | ------------------------------------------------------ | ---------------------------------- |
| `DATABASE_URL`       | —                                                      | required                           |
| `POLL_INTERVAL_SECS` | `2`                                                    | sleep between polls when empty     |
| `BATCH_SIZE`         | `10`                                                   | rows claimed per poll              |
| `MAX_ATTEMPTS`       | `3`                                                    | transient retries before `failed`  |
| `MIN_SCHEMA_VERSION` | `2`                                                    | wait until api has applied this    |
| `RUST_LOG`           | `info`                                                 | e.g. `courierx_worker=debug`       |

## Shutdown

`SIGINT` (Ctrl+C) or `SIGTERM`: the in-flight batch finishes, then the loop
exits cleanly.
