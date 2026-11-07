use serde_json::Value;

use crate::error::IngestError;
use crate::input::Raw;

/// Maximum number of field names a description lists.
const NAMED_FIELDS: usize = 24;

/// A dead-lettered input with the typed reason it could not become an event.
#[derive(Debug, Clone, PartialEq)]
pub struct QuarantineRecord {
    /// Where the input came from.
    pub source: &'static str,
    /// Why the input was rejected.
    pub error: IngestError,
    /// The input described by its shape: size and field names, never values.
    pub shape: String,
}

/// Describes rejected data by its shape, never by its contents.
pub(crate) fn describe(raw: &Raw) -> String {
    match raw {
        Raw::Worldstate(body) => document("world-state document", body),
        Raw::CatalogManifest(body) => document("catalog manifest", body),
        Raw::CatalogVersion(text) => format!("catalog version, {} bytes", text.len()),
        Raw::PackVersion(text) => format!("pack version, {} bytes", text.len()),
    }
}

/// A document's size and, when it parses as a JSON object, its field names in order.
fn document(what: &str, body: &str) -> String {
    let size = format!("{what}, {} bytes", body.len());
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(body) else {
        return size;
    };

    let names = fields
        .keys()
        .take(NAMED_FIELDS)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    match fields.len() > NAMED_FIELDS {
        true => format!(
            "{size}, fields: {names}, and {} more",
            fields.len() - NAMED_FIELDS
        ),
        false => format!("{size}, fields: {names}"),
    }
}

#[cfg(test)]
mod tests {
    use super::describe;
    use crate::input::Raw;

    #[test]
    fn a_document_is_described_by_its_fields() {
        let raw = Raw::Worldstate(r#"{"Time": 1785240441, "ActiveMissions": []}"#.into());

        assert_eq!(
            describe(&raw),
            "world-state document, 42 bytes, fields: ActiveMissions, Time"
        );
    }

    #[test]
    fn a_document_that_is_not_json_keeps_its_size_alone() {
        let raw = Raw::Worldstate("not json".into());

        assert_eq!(describe(&raw), "world-state document, 8 bytes");
    }

    #[test]
    fn a_description_never_carries_a_value() {
        let raw = Raw::CatalogManifest(r#"{"account": "Sneaky", "riven": 170}"#.into());

        let shape = describe(&raw);

        assert!(!shape.contains("Sneaky"), "{shape}");
        assert!(!shape.contains("170"), "{shape}");
    }
}
