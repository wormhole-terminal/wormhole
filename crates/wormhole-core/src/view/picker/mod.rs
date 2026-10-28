//! A picker: a field, a list of rows under it, a ghost completion after the caret and a breadcrumb above. It
//! knows nothing of what it lists. Folders are one thing it can list (`crate::folder_picker`); the rows,
//! their order, what ↩ hands over and what ⇥ completes all come from providers.
//!
//! ## How a provider plugs in
//!
//! 1. Implement [`Provider`]: `query(&Query, &Cancel) -> Answer`. A [`Query`] is the text in the field and a
//!    [`Context`] (the terminal's folder, home, the profile). An [`Answer`] is rows, section titles, notes,
//!    the text that goes one level up, and the breadcrumb. It blocks, and runs on a pool thread; check
//!    [`Cancel`] between slow steps.
//! 2. Make each [`Row`] say everything about itself as data: a `key` that is the same whenever the same thing
//!    is listed, the `label` and where the typed text matched in it (`marks`, from [`fit`] if the match is
//!    forgiving), an SF Symbol name, an optional badge and detail, `descend` (the field text that goes into
//!    the row, which is what ⇥, → and the ghost use) and `insert` (what ↩ or a click hands to whoever opened
//!    the picker). A row with no `insert` is listed but can't be picked; one with no `descend` can't be gone
//!    into. A row that is listed again is the same row: the selection, decorations and forgetting go by its key.
//! 3. Optionally implement [`Decorator`] for what is slower than listing (a badge, whether there is more behind
//!    a row). It is asked only for the rows in view, a few at a time.
//! 4. Several providers are one with [`Compose`]: rows that answer into the same section are ranked together
//!    by `score`, a section with a new title follows the ones before it, and a row two providers list is
//!    listed once.
//!
//! Every type a provider sees or returns is plain, owned and `serde`-shaped (no closures, no handles), so a
//! provider written outside this crate, a Probe, could answer in the same words over any channel.
//!
//! ## Who does what
//!
//! [`Model`] is the state: it asks for a query ([`Model::request`]) and takes the answer back by generation, so
//! text that has moved on drops a late answer; it holds the selection, the window over the list, the ghost
//! and what a key does ([`decide`], which a panel may also ask directly, because AppKit wants the answer to a
//! key before the key returns). It runs nothing. The app's `picker` module runs it: queries and decorations
//! off the main thread, and the native panel (`platform::picker`) that shows a [`Snapshot`] and reports keys,
//! text and clicks. The one module that knows the terminal is the one that opens the picker for it.

mod compose;
mod fuzzy;
mod ghost;
mod keys;
mod model;
mod place;
mod provider;
mod shown;
mod types;

#[cfg(test)]
mod tests;

pub use compose::{Compose, merge};
pub use fuzzy::{Fit, fit};
pub use ghost::ghost;
pub use keys::{Do, Facts, Field, Key, PAGE, decide};
pub use model::{DecorationRequest, Model, Outcome, Request, Snapshot};
pub use place::{Card, GAP, MARGIN, Placed, place};
pub use provider::{Cancel, Decorator, Provider};
pub use shown::{Item, Shown};
pub use types::{Answer, Context, Crumb, Decoration, Note, Query, Row};
