use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use median_core::Instant;
use median_market::Book;

/// Seconds a book is kept. No decision uses an older book, and keeping every book the
/// scheduler fetches would take hundreds of megabytes.
const KEEP: Instant = 600;

/// A book and the time it was fetched.
type Answered = (Instant, Arc<Book>);

/// Latest full book of each traded pair, in memory. The market cache keeps only a summary of
/// each book read; pricing a bid needs the rows, which are kept here.
#[derive(Clone, Default)]
pub struct Books(Arc<RwLock<HashMap<String, Answered>>>);

impl Books {
    /// Stores the book of one item and drops books older than `KEEP`.
    pub fn keep(&self, slug: &str, book: Book, now: Instant) {
        let mut kept = self.0.write().unwrap_or_else(PoisonError::into_inner);
        kept.retain(|_, (taken, _)| now.saturating_sub(*taken) < KEEP);
        kept.insert(slug.to_owned(), (now, Arc::new(book)));
    }

    /// The book of one item and its fetch time; `None` when none is kept.
    pub fn look(&self, slug: &str) -> Option<Answered> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(slug)
            .cloned()
    }
}
