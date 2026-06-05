use std::time::Duration;

use tokio::time::sleep;
use uuid::Uuid;

use super::{EmailProvider, ProviderError, SentInfo};
use crate::models::Email;

/// Debug provider: logs the email to stdout via `tracing` and returns a fake message id.
pub struct StdoutProvider;

#[async_trait::async_trait]
impl EmailProvider for StdoutProvider {
    async fn send(&self, email: &Email) -> Result<SentInfo, ProviderError> {
        sleep(Duration::from_millis(100)).await;

        tracing::info!(
            id = %email.id,
            from = %email.from_addr,
            to = ?email.to_addrs,
            subject = %email.subject,
            text = email.text_body.as_deref().unwrap_or(""),
            html = email.html_body.as_deref().unwrap_or(""),
            "[stdout-provider] delivered"
        );

        Ok(SentInfo {
            provider_message_id: format!("stdout_{}", Uuid::new_v4()),
        })
    }

    fn name(&self) -> &'static str {
        "stdout"
    }
}
