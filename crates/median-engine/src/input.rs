use median_core::Event;

/// An input to the engine: raw data to parse, or a ready event.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// Data from outside that has yet to be parsed.
    Raw(Raw),
    /// A command already in the shape the core folds.
    Command(Event),
}

/// Unparsed data, as received from its source.
#[derive(Debug, Clone, PartialEq)]
pub enum Raw {
    /// A world-state document fetched from the source.
    Worldstate(String),
    /// A catalog manifest fetched from the release.
    CatalogManifest(String),
    /// The version an opened catalog file declares.
    CatalogVersion(String),
    /// Release version of the pictures on disk.
    PackVersion(String),
}

impl Raw {
    /// A short label of where this input came from.
    pub(crate) fn source(&self) -> &'static str {
        match self {
            Raw::Worldstate(_) => "worldstate",
            Raw::CatalogManifest(_) | Raw::CatalogVersion(_) | Raw::PackVersion(_) => "catalog",
        }
    }
}
