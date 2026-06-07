use std::env;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProviderKind {
    #[default]
    Stdout,
    Resend,
}

impl std::str::FromStr for ProviderKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "stdout" => Ok(ProviderKind::Stdout),
            "resend" => Ok(ProviderKind::Resend),
            other => Err(anyhow::anyhow!(
                "unknown PROVIDER value: {other} (expected: stdout, resend)"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub poll_interval_secs: u64,
    pub batch_size: i64,
    pub max_attempts: i32,
    pub min_schema_version: i64,
    pub provider: ProviderKind,
    pub resend_api_key: Option<String>,
    pub resend_base_url: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;

        let provider = match env::var("PROVIDER").ok() {
            Some(v) if !v.trim().is_empty() => v.parse::<ProviderKind>()?,
            _ => ProviderKind::default(),
        };

        let resend_api_key = env::var("RESEND_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty());

        let resend_base_url = env::var("RESEND_BASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "https://api.resend.com".to_string());

        if provider == ProviderKind::Resend && resend_api_key.is_none() {
            anyhow::bail!("PROVIDER=resend requires RESEND_API_KEY env var to be set");
        }

        Ok(Self {
            database_url,
            poll_interval_secs: parse_env("POLL_INTERVAL_SECS", 2),
            batch_size: parse_env("BATCH_SIZE", 10),
            max_attempts: parse_env("MAX_ATTEMPTS", 3),
            min_schema_version: parse_env("MIN_SCHEMA_VERSION", 2),
            provider,
            resend_api_key,
            resend_base_url,
        })
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
