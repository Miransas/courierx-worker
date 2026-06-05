use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::time::sleep;

/// Postgres error code for "undefined table".
const PG_UNDEFINED_TABLE: &str = "42P01";

/// How long to back off between schema-readiness probes.
const SCHEMA_RETRY_DELAY: Duration = Duration::from_secs(5);

/// Build a Postgres connection pool with the worker's standard settings.
pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    PgPoolOptions::new().max_connections(5).connect(url).await
}

/// Block until the shared `_sqlx_migrations` table reports a version at least
/// `min_version`. The worker never runs migrations itself — that's owned by
/// `courierx-api`. We just wait (and log) until the api has caught the DB up.
pub async fn check_schema_version(pool: &PgPool, min_version: i64) -> anyhow::Result<()> {
    loop {
        match sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(pool)
            .await
        {
            Ok(Some(v)) if v >= min_version => {
                tracing::info!(version = v, min_version, "schema version OK");
                return Ok(());
            }
            Ok(Some(v)) => {
                tracing::warn!(
                    version = v,
                    min_version,
                    "schema version too old, waiting for courierx-api to migrate…"
                );
            }
            Ok(None) => {
                tracing::warn!(
                    min_version,
                    "_sqlx_migrations is empty, waiting for courierx-api to migrate…"
                );
            }
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some(PG_UNDEFINED_TABLE) => {
                tracing::error!("schema not initialized — start courierx-api first");
            }
            Err(e) => {
                tracing::error!(error = %e, "schema check query failed, retrying");
            }
        }
        sleep(SCHEMA_RETRY_DELAY).await;
    }
}
