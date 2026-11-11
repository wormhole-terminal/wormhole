//! The status machines of a window: one per terminal that has been told anything, what they asked of the
//! shell and have not been answered, and what the window tells all of them.

use std::collections::HashMap;

use super::super::facts::Millis;
use super::{Ask, StatusMachine};

#[derive(Debug)]
pub(in crate::workspace) struct StatusBook {
    pub(in crate::workspace) machines: HashMap<i64, StatusMachine>,
    pub(in crate::workspace) window_active: bool,
    pub(in crate::workspace) asks: Vec<(i64, Ask)>,
    /// The latest moment the shell named. A person's action (selecting a row) has no time of its
    /// own, and what it settles into (idle) says nothing a time would be read from.
    pub(in crate::workspace) clock: Millis,
}

impl Default for StatusBook {
    fn default() -> Self {
        Self { machines: HashMap::new(), window_active: true, asks: Vec::new(), clock: 0 }
    }
}
