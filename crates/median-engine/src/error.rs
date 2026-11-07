use thiserror::Error;

/// Why a raw input could not become a domain event.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum IngestError {
    /// The world-state document could not be parsed.
    #[error("malformed world-state: {0}")]
    MalformedWorldstate(String),
    /// The catalog manifest could not be parsed.
    #[error("malformed catalog manifest: {0}")]
    MalformedManifest(String),
    /// The installed catalog declares a version string that does not parse.
    #[error("unreadable catalog version: {0:?}")]
    MalformedCatalogVersion(String),
}
