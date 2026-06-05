# CourierX API — Claude Code Rules

## Hard rules
- NEVER run `git commit`, `git push`, or any git write operation. User does all commits manually.
- NEVER modify `.env` files. Only edit `.env.example`. If a new env var is needed, add it to `.env.example` and tell the user.
- NEVER write secrets, tokens, API keys, or passwords into any file (including comments, examples, READMEs). Use placeholders like `<YOUR_KEY>`.
- NEVER delete or rewrite `migrations/*.sql` files that already exist. Add a new numbered migration instead.
- NEVER run `cargo run` long-running. Use `cargo check` or `cargo build` to verify. User runs the server.
- - Schema source of truth: courierx-api/migrations/. Bu klasörde olmayan bir kolon yoksa, varsaymak yerine - -       kullanıcıyasor: "API repo'sunda migration ekleyelim mi?"

## Stack
- Rust stable, Axum 0.7, SQLx 0.8 (Postgres), tokio.
- Use `sqlx::query!` / `query_as!` macros where possible (compile-time checked). If offline, use `sqlx::query` with manual types and note it.
- Errors via `thiserror` + a single `AppError` enum that implements `IntoResponse`. Every error response is JSON `{"error": "<code>", "message": "<msg>"}`.
- Logging via `tracing`. No `println!` in library code.

## Code style
- Modules small and single-purpose. One route group per file under `src/routes/`.
- Public functions get doc comments. No comments that just restate the code.
- No `unwrap()` or `expect()` in request handlers. Use `?` with `AppError`.
- Prefer `&str` over `String` in function args where ownership isn't needed.

## Database
- All schema changes are new migrations in `migrations/NNNN_name.sql`. Never edit an existing migration.
- Use `TIMESTAMPTZ` not `TIMESTAMP`. UUIDs for all primary keys.
- Index any column used in a WHERE clause by the worker or by a hot endpoint.

## API conventions
- All endpoints under `/v1/`. Health and metrics outside `/v1/`.
- Request and response bodies: JSON only. snake_case field names.
- Resend-compatible request shapes where possible (so SDK users can migrate).
- 2xx success, 4xx client error, 5xx server. 202 for queued, 200 for synchronous.

## When unsure
- Stop and ask. Don't invent table columns, env vars, or external service behavior.
- Don't add new dependencies without asking. List what you'd add and why.

## Verification before declaring done
- `cargo check` passes with no warnings (or warnings explained).
- `cargo fmt` applied.
- `cargo clippy -- -D warnings` passes (or each remaining warning is justified).
- A short summary at the end: files created/changed, what works, what's not done, and exact commands user should run to test.

# Global rules for Sardor

- Never run git commit, git push, or any git write operation.
- Never modify .env files. Only .env.example.
- Never put secrets in any file. Use placeholders.
- On macOS: don't use nano with Ctrl+O for file writes — prefer `cat > file << 'EOF'` heredoc or direct file write tools.
- Be terse. Skip preamble. No "Great question!" or "I'll help you with that".
- After finishing a task: short summary of files changed and exact next commands. No essays.
- Default to small, focused PR-sized changes. Ask before large refactors.