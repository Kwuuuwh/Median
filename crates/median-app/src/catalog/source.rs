use std::env;

use median_core::CATALOG_SCHEMA;
use reqwest::header::RANGE;
use reqwest::{Client, StatusCode};

/// Base URL of the Median-Data release with the game data of `schema`.
fn releases(schema: u32) -> String {
    format!("https://github.com/Kwuuuwh/Median-Data/releases/download/schema-{schema}")
}

/// Environment variable overriding the base URL.
const BASE: &str = "MEDIAN_DATA_BASE_URL";

/// File names of the release assets.
pub const MANIFEST: &str = "manifest.json";
pub const INDEX: &str = "pack.index";

/// Largest buffer reserved up front for a download.
const RESERVE_LIMIT: u64 = 64 << 20;

/// HTTP access to the published game data: manifest, catalog and picture pack.
pub struct Source {
    client: Client,
    base: String,
}

impl Default for Source {
    /// Reads the base URL from the environment, falling back to the release of this build's
    /// schema.
    fn default() -> Self {
        let base = env::var(BASE).unwrap_or_else(|_| releases(CATALOG_SCHEMA));
        let client = crate::http::http_client().expect("failed to build http client");
        Self {
            client,
            base: base.trim_end_matches('/').to_string(),
        }
    }
}

impl Source {
    /// Downloads `file` as text; invalid UTF-8 is replaced, not rejected.
    pub async fn text(&self, file: &str) -> Result<String, String> {
        let mut body = String::new();
        self.stream(file, None, |chunk| {
            body.push_str(&String::from_utf8_lossy(chunk));
            Ok(())
        })
        .await?;
        Ok(body)
    }

    /// Downloads `file` whole, calling `progress` with the bytes read so far and `size`.
    pub async fn download(
        &self,
        file: &str,
        size: u64,
        mut progress: impl FnMut(u64, u64),
    ) -> Result<Vec<u8>, String> {
        let mut body = Vec::with_capacity(size.min(RESERVE_LIMIT) as usize);
        self.stream(file, None, |chunk| {
            body.extend_from_slice(chunk);
            progress(body.len() as u64, size);
            Ok(())
        })
        .await?;
        Ok(body)
    }

    /// Downloads `file`, or its half-open byte range `[start, end)`, passing each chunk to `sink`
    /// as it arrives; fails unless the status is 200 (206 for a range).
    pub async fn stream(
        &self,
        file: &str,
        range: Option<(u64, u64)>,
        mut sink: impl FnMut(&[u8]) -> Result<(), String>,
    ) -> Result<(), String> {
        let url = format!("{}/{file}", self.base);
        let mut request = self.client.get(&url);
        if let Some((start, end)) = range {
            request = request.header(RANGE, format!("bytes={start}-{}", end - 1));
        }

        let mut response = request.send().await.map_err(|e| e.to_string())?;
        let expected = match range {
            Some(_) => StatusCode::PARTIAL_CONTENT,
            None => StatusCode::OK,
        };
        let status = response.status();
        if status != expected {
            return Err(format!("{url} answered {status}, expected {expected}"));
        }

        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            sink(&chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_comes_from_the_release_of_the_schema() {
        assert_eq!(
            releases(19),
            "https://github.com/Kwuuuwh/Median-Data/releases/download/schema-19"
        );
    }
}
