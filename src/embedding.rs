use qdrant_client::client::QdrantClient;

use crate::{errors::SourceCodeError, repository::RepositoryInfo};

pub async fn vectorize_repository(stats: Vec<RepositoryInfo>) -> Result<(), SourceCodeError> {
    let _stats_json = serde_json::to_string(&stats)
        .map_err(|err| SourceCodeError::SerializationError(err.into()))?;

    let _client = QdrantClient::from_url("http://localhost:6334").build()?;

    Ok(())
}
