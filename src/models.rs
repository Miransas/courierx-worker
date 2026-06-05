use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Mirrors a row in the `emails` table (schema owned by courierx-api).
///
/// Some fields aren't read by the worker today but are loaded so future
/// providers (and observability code) can access the full row.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Email {
    pub id: Uuid,
    pub from_addr: String,
    pub to_addrs: Vec<String>,
    pub subject: String,
    pub html_body: Option<String>,
    pub text_body: Option<String>,
    pub status: String,
    pub attempts: i32,
    pub error: Option<String>,
    pub provider_message_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}
