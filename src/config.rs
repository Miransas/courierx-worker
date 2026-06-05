use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub poll_interval_secs: u64,
    pub batch_size: i64,
    pub max_attempts: i32,
    pub min_schema_version: i64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;

        Ok(Self {
            database_url,
            poll_interval_secs: parse_env("POLL_INTERVAL_SECS", 2),
            batch_size: parse_env("BATCH_SIZE", 10),
            max_attempts: parse_env("MAX_ATTEMPTS", 3),
            min_schema_version: parse_env("MIN_SCHEMA_VERSION", 2),
        })
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
