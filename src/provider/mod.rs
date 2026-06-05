use crate::models::Email;

pub mod stdout;

/// Pluggable backend that actually delivers an email.
#[async_trait::async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, email: &Email) -> Result<SentInfo, ProviderError>;
    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct SentInfo {
    pub provider_message_id: String,
}

#[allow(dead_code)] // variants are constructed by real providers (SES, Resend, SMTP).
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    /// Recoverable failure (network blip, 5xx). Worker will requeue if attempts remain.
    #[error("transient: {0}")]
    Transient(String),

    /// Unrecoverable failure (bad address, auth error). Worker fails the email immediately.
    #[error("permanent: {0}")]
    Permanent(String),
}
