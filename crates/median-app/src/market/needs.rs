use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock};

use median_core::{Holder, Need};

/// Pairs each holder needs the scheduler to keep fresh, keyed by holder. A holder's needs stay
/// until that holder declares new ones.
#[derive(Clone, Default)]
pub struct Needs(Arc<RwLock<BTreeMap<Holder, Vec<Need>>>>);

impl Needs {
    /// Replaces the needs of one holder; other holders' needs are unchanged.
    pub fn declare(&self, holder: Holder, needs: Vec<Need>) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(holder, needs);
    }

    /// The current needs of all holders, as one list.
    pub fn all(&self) -> Vec<Need> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .flatten()
            .cloned()
            .collect()
    }
}
