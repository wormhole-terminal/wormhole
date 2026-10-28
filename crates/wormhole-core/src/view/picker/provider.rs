//! What a source of rows has to be. Two small traits, and the data they trade is in `types`.
//!
//! A trait and not a table of functions, because a provider keeps something (the recent folders it was given,
//! a cache) and is run off the main thread: the driver holds it as `Arc<dyn Provider>`. A `ProviderId` enum
//! and a table would name every provider in this crate, which is the one thing a provider from outside must
//! not need. The methods block: the driver calls them on a pool thread, and a provider that reads the disk
//! checks [`Cancel`] between its reads, so a query the text has moved past stops early.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::types::{Answer, Decoration, Query};

/// Whether the answer is still wanted. Cloned into the thread that runs the query; the driver sets it when the
/// text has changed or the picker is closed.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// A source of rows for what is typed.
pub trait Provider: Send + Sync {
    /// The rows for `query`. A cancelled query may return whatever it has: it is dropped.
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer;
}

/// Something that finds out more about rows already listed (a badge, whether there is more behind a row),
/// slower than listing them: asked only for the rows in view.
pub trait Decorator: Send + Sync {
    /// What is known about the rows with these keys; a key it knows nothing about is left out.
    fn decorate(&self, keys: &[String], cancel: &Cancel) -> Vec<Decoration>;
}
