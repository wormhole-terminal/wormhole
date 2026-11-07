//! The Probes that are on, as the rest of the app reads them: the one set of their text rules the terminals'
//! rows and links use, the status items their hooks set and the blocks their hooks marked.
//!
//! State only, and low in the layers (beside `automation`): the terminal view and the windows read it, and
//! `app::probes` is what loads the Probes, listens to the event bus and writes it. Every change says
//! so (`cx.refresh_windows()`), because the views that draw it are cached.
//!
//! A Probe's rule is decoration and links, never an input: nothing here changes a terminal's text, and
//! with the `probes` flag off, or no Probe on, there is no rule set and a frame pays nothing for it.

mod rules;
mod state;

pub(crate) use rules::{Decoration, RuleSet};
pub(crate) use state::{
    StatusItem, approvals_changed, block_tone, generation, install, mark_block, on_approvals_changed,
    probe_name, rules, set_status, status_items,
};
