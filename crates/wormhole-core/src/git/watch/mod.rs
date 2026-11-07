//! Knowing that a repository changed without asking git: which file events matter to `git status`
//! ([`Filter`]), when to read after them ([`Debounce`]), and where a repository's files are ([`Layout`]).
//!
//! The events come from `platform::watch` and the reading is the app's; this module is the rules
//! between, so they are tested without a watcher or a clock. A path that is not named by a rule here
//! does not trigger a read, and the polling that stays as a backstop (a slow one while a watch lives)
//! catches anything these rules were too strict about.
//!
//! Repository content is hostile input: the listing of ignored paths is capped, and a path that
//! cannot be placed under the work tree or a git dir matters to no one.

mod debounce;
mod filter;
mod ignored;
mod layout;
mod marks;

pub use debounce::{Debounce, MAX_WAIT, MIN_GAP, QUIET};
pub use filter::{Filter, Relevance};
pub use ignored::Ignored;
pub use layout::Layout;
pub use marks::{Marks, on_batch};
