use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::watch;
use tokio::time::sleep;
use uuid::Uuid;

use crate::config::Config;
use crate::error::WorkerError;
use crate::models::Email;
use crate::provider::{EmailProvider, ProviderError};

/// Drive the polling loop until `shutdown` flips to `true`.
pub async fn run(
    pool: PgPool,
    provider: Arc<dyn EmailProvider>,
    cfg: Config,
    mut shutdown: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    tracing::info!(
        provider = provider.name(),
        batch_size = cfg.batch_size,
        poll_interval_secs = cfg.poll_interval_secs,
        max_attempts = cfg.max_attempts,
        "worker started"
    );

    let poll = Duration::from_secs(cfg.poll_interval_secs);

    loop {
        if *shutdown.borrow() {
            break;
        }

        let claimed = match claim_batch(&pool, cfg.batch_size).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "claim_batch failed");
                sleep_or_shutdown(&mut shutdown, poll).await;
                continue;
            }
        };

        if claimed.is_empty() {
            sleep_or_shutdown(&mut shutdown, poll).await;
            continue;
        }

        tracing::debug!(count = claimed.len(), "claimed batch");

        for email in claimed {
            if *shutdown.borrow() {
                break;
            }
            let id = email.id;
            if let Err(e) = process_one(&pool, provider.as_ref(), email, cfg.max_attempts).await {
                tracing::error!(%id, error = %e, "process_one failed");
            }
        }
    }

    tracing::info!("worker stopped");
    Ok(())
}

/// Atomically claim up to `batch_size` queued rows: flip them to `sending`,
/// bump `attempts`, and return the full rows in a single short transaction.
///
/// `FOR UPDATE SKIP LOCKED` in the CTE means concurrent workers never wait on
/// each other; the surrounding UPDATE releases the row locks as soon as the
/// statement commits, so the provider call below runs without holding any lock.
async fn claim_batch(pool: &PgPool, batch_size: i64) -> Result<Vec<Email>, WorkerError> {
    let rows = sqlx::query_as::<_, Email>(
        r#"
        WITH next AS (
            SELECT id
            FROM emails
            WHERE status = 'queued'
            ORDER BY created_at
            FOR UPDATE SKIP LOCKED
            LIMIT $1
        )
        UPDATE emails AS e
        SET status = 'sending',
            attempts = e.attempts + 1,
            updated_at = NOW()
        FROM next
        WHERE e.id = next.id
        RETURNING e.id, e.from_addr, e.to_addrs, e.subject,
                  e.html_body, e.text_body, e.status, e.attempts,
                  e.error, e.provider_message_id,
                  e.created_at, e.updated_at, e.sent_at
        "#,
    )
    .bind(batch_size)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn process_one(
    pool: &PgPool,
    provider: &dyn EmailProvider,
    email: Email,
    max_attempts: i32,
) -> Result<(), WorkerError> {
    let id = email.id;
    let attempts = email.attempts;

    match provider.send(&email).await {
        Ok(info) => {
            mark_sent(pool, id, &info.provider_message_id).await?;
            tracing::info!(%id, message_id = %info.provider_message_id, "sent");
        }
        Err(ProviderError::Transient(msg)) if attempts < max_attempts => {
            tracing::warn!(%id, attempts, max_attempts, error = %msg, "transient, requeueing");
            mark_requeue(pool, id, &msg).await?;
        }
        Err(ProviderError::Transient(msg)) => {
            tracing::error!(%id, attempts, "max attempts reached, failing");
            mark_failed(pool, id, &msg).await?;
        }
        Err(ProviderError::Permanent(msg)) => {
            tracing::error!(%id, error = %msg, "permanent error, failing");
            mark_failed(pool, id, &msg).await?;
        }
    }

    Ok(())
}

async fn mark_sent(pool: &PgPool, id: Uuid, provider_message_id: &str) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE emails
        SET status = 'sent',
            provider_message_id = $2,
            sent_at = NOW(),
            updated_at = NOW(),
            error = NULL
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(provider_message_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn mark_requeue(pool: &PgPool, id: Uuid, err: &str) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE emails
        SET status = 'queued',
            error = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(err)
    .execute(pool)
    .await?;
    Ok(())
}

async fn mark_failed(pool: &PgPool, id: Uuid, err: &str) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE emails
        SET status = 'failed',
            error = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(err)
    .execute(pool)
    .await?;
    Ok(())
}

async fn sleep_or_shutdown(shutdown: &mut watch::Receiver<bool>, dur: Duration) {
    tokio::select! {
        _ = sleep(dur) => {}
        _ = shutdown.changed() => {}
    }
}
