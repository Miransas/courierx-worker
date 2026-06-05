use crate::provider::ProviderError;

#[derive(thiserror::Error, Debug)]
pub enum WorkerError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
}
