use median_core::{CatalogEvent, Event, Version};

use crate::catalog;
use crate::document;
use crate::error::IngestError;
use crate::input::Raw;

/// Maps raw data to a domain event, or the typed reason it can't become one.
pub(crate) fn ingest(raw: &Raw) -> Result<Event, IngestError> {
    match raw {
        Raw::Worldstate(body) => document::parse(body)
            .map(|(worldstate, shelves)| Event::WorldReceived {
                worldstate,
                shelves,
            })
            .map_err(IngestError::MalformedWorldstate),
        Raw::CatalogManifest(body) => catalog::parse(body)
            .map(|release| Event::Catalog(CatalogEvent::Checked(release)))
            .map_err(IngestError::MalformedManifest),
        Raw::CatalogVersion(version) => Version::parse(version)
            .map(|version| Event::Catalog(CatalogEvent::Ready(version)))
            .ok_or_else(|| IngestError::MalformedCatalogVersion(version.clone())),
        Raw::PackVersion(version) => Version::parse(version)
            .map(|version| Event::Catalog(CatalogEvent::PackReady(version)))
            .ok_or_else(|| IngestError::MalformedCatalogVersion(version.clone())),
    }
}
