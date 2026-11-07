use median_core::{Affixes, riven_name};
use median_ipc::Riven;

use super::appraisal::to_core;
use crate::catalog::{self, Catalog, Pooled};

/// In-game name of each of `rivens`, in input order; `None` when the weapon has no riven class
/// or the catalog lacks a stat's name affixes.
pub async fn names(catalog: &Catalog, rivens: Vec<Riven>) -> Result<Vec<Option<String>>, String> {
    catalog
        .read(move |connection| {
            let mut named = Vec::with_capacity(rivens.len());
            for riven in &rivens {
                let Some(class) = catalog::riven_class(connection, &riven.weapon)? else {
                    named.push(None);
                    continue;
                };
                let pool = catalog::pool(connection, "en", &class)?;
                named.push(riven_name(&to_core(riven), &affixes(&pool)));
            }
            Ok(named)
        })
        .await
        .map_err(|e| e.to_string())
}

/// Name prefix and suffix that each stat of `pool` contributes to a riven's name.
pub(super) fn affixes(pool: &[Pooled]) -> Vec<Affixes<'_>> {
    pool.iter()
        .map(|stat| Affixes {
            tag: &stat.tag,
            prefix: &stat.prefix,
            suffix: &stat.suffix,
        })
        .collect()
}
