//! The Probes that are on, as the rest of the app reads them: the one set of their text rules the terminals'
//! rows and links use, the status items their hooks set and the blocks their hooks marked.
//!
//! State only, and low in the layers (beside `automation`): the terminal view and the windows read it, and
//! `app::probes` is what loads the Probes, listens to the event bus and writes it. Every change says
//! so (`cx.refresh_windows()`), because the views that draw it are cached. What the rules and effects are is
//! `wormhole_core::probe::runtime`, which the Swift app reads as well; this holds them in a global.
//!
//! A Probe's rule is decoration and links, never an input: nothing here changes a terminal's text, and
//! with the `probes` flag off, or no Probe on, there is no rule set and a frame pays nothing for it.

mod state;

pub(crate) use state::{
    approvals_changed, block_tone, generation, install, mark_block, on_approvals_changed, probe_name, rules,
    set_status, status_items,
};
pub(crate) use wormhole_core::probe::runtime::{RuleSet, StatusItem};
