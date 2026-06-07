use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use super::{EmailProvider, ProviderError, SentInfo};
use crate::models::Email;

/// Resend HTTP API provider. Talks to `POST {base_url}/emails`.
pub struct ResendProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ResendProvider {
    pub fn new(api_key: String, base_url: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build reqwest client: {e}"))?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }
}

#[derive(Debug, Serialize)]
struct ResendSendRequest<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct ResendSendResponse {
    id: String,
}

#[derive(Debug, Deserialize, Default)]
struct ResendErrorResponse {
    #[serde(default)]
    message: String,
    #[serde(default)]
    name: String,
}

#[async_trait::async_trait]
impl EmailProvider for ResendProvider {
    async fn send(&self, email: &Email) -> Result<SentInfo, ProviderError> {
        let url = format!("{}/emails", self.base_url);

        let to: Vec<&str> = email.to_addrs.iter().map(String::as_str).collect();

        let body = ResendSendRequest {
            from: email.from_addr.as_str(),
            to,
            subject: email.subject.as_str(),
            html: email.html_body.as_deref(),
            text: email.text_body.as_deref(),
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(classify_send_error)?;

        let status = response.status();

        if status.is_success() {
            let parsed: ResendSendResponse = response.json().await.map_err(|e| {
                ProviderError::Permanent(format!("invalid response from resend: {e}"))
            })?;
            return Ok(SentInfo {
                provider_message_id: parsed.id,
            });
        }

        let error_body = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<ResendErrorResponse>(&error_body)
            .ok()
            .map(|e| {
                if !e.message.is_empty() {
                    e.message
                } else if !e.name.is_empty() {
                    e.name
                } else {
                    error_body.clone()
                }
            })
            .unwrap_or_else(|| error_body.clone());

        Err(classify_http_error(status, &message))
    }

    fn name(&self) -> &'static str {
        "resend"
    }
}

fn classify_send_error(e: reqwest::Error) -> ProviderError {
    if e.is_timeout() || e.is_connect() || e.is_request() {
        ProviderError::Transient(format!("network error: {e}"))
    } else {
        ProviderError::Permanent(format!("request error: {e}"))
    }
}

fn classify_http_error(status: StatusCode, message: &str) -> ProviderError {
    match status {
        StatusCode::UNAUTHORIZED
        | StatusCode::FORBIDDEN
        | StatusCode::BAD_REQUEST
        | StatusCode::NOT_FOUND
        | StatusCode::UNPROCESSABLE_ENTITY => {
            ProviderError::Permanent(format!("resend rejected request ({status}): {message}"))
        }
        StatusCode::TOO_MANY_REQUESTS
        | StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => {
            ProviderError::Transient(format!("resend transient error ({status}): {message}"))
        }
        _ if status.is_server_error() => {
            ProviderError::Transient(format!("resend transient error ({status}): {message}"))
        }
        _ => ProviderError::Permanent(format!(
            "resend returned unexpected status {status}: {message}"
        )),
    }
}
