//! The right panel's tab switch as the window plays it (core `view::motion::tabs`): the new tab's content
//! slides in from the side of the tab it came from, and the selection's spring is the tab bar's own
//! (`style::selection_bar`). The panel is a cached region, so the switch is remembered here until its time
//! is up, as the status bar remembers a roll: a build in the middle of one (a tick, a count that changed)
//! carries it on instead of cutting it, and it is drawn as an animation inside the region, which draws
//! that region and nothing else. At rest it holds nothing: no wrapper, no work.

use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::view::motion::tabs::{CONTENT_IN, Side};

use crate::settings::settings_in;
use crate::window::Workspace;

/// How long after it should have finished a switch is kept, so its last frame is drawn.
const GRACE: Duration = Duration::from_millis(40);

/// The tab switch under way.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Switch {
    side: Side,
    /// Which switch it was: tells its animation from the next one's, which runs from its start.
    serial: u64,
    began: Instant,
}

/// What the panel is playing of a tab switch.
#[derive(Debug, Default)]
pub(super) struct PanelSwitch {
    current: Option<Switch>,
    switches: u64,
}

impl PanelSwitch {
    /// A switch from the tab at `from` to the one at `to` (their places in the bar) begins.
    fn begin(&mut self, from: usize, to: usize, now: Instant) {
        self.switches += 1;
        self.current = Some(Switch { side: Side::of_switch(from, to), serial: self.switches, began: now });
    }

    /// Whatever is playing is cut.
    fn cut(&mut self) {
        self.current = None;
    }

    /// The side the content comes from and which switch it is, while one is on its way at `now`.
    pub(super) fn playing(&self, now: Instant) -> Option<(Side, u64)> {
        let switch = self.current.filter(|switch| now.duration_since(switch.began) < CONTENT_IN + GRACE)?;
        Some((switch.side, switch.serial))
    }

    /// How many switches have begun: a fact the panel is built from, which only a switch changes.
    pub(super) fn count(&self) -> u64 {
        self.switches
    }
}

impl Workspace {
    /// The panel shows `to` where it showed `from`: the new content slides in, unless motion is off or
    /// reduced, the panel was not on screen (it slides in as a whole), or one of the tabs is no tab of the
    /// bar (one that vanished with its feature).
    pub(super) fn start_tab_switch(
        &mut self,
        from: TabId,
        to: TabId,
        was_visible: bool,
        cx: &mut Context<Self>,
    ) {
        let tabs = self.panel_tab_list(cx);
        let places = tabs.iter().position(|&tab| tab == from).zip(tabs.iter().position(|&tab| tab == to));
        let on = was_visible && settings_in(&self.profile, cx).motion_on() && !cx.reduce_motion();
        match places.filter(|_| on) {
            Some((from, to)) => {
                self.regions.drawn.panel_switch.begin(from, to, cx.background_executor().now())
            }
            None => self.regions.drawn.panel_switch.cut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{CONTENT_IN, GRACE, PanelSwitch, Side};

    #[test]
    fn a_switch_plays_for_its_duration_and_is_kept_a_moment_longer_for_its_last_frame() {
        let (mut switch, now) = (PanelSwitch::default(), Instant::now());
        assert_eq!(switch.playing(now), None, "at rest it holds nothing");
        switch.begin(0, 2, now);
        assert_eq!(switch.playing(now), Some((Side::Right, 1)));
        assert_eq!(switch.playing(now + CONTENT_IN), Some((Side::Right, 1)), "the last frame is drawn");
        assert_eq!(switch.playing(now + CONTENT_IN + GRACE), None, "and then it is gone");
    }

    #[test]
    fn a_second_switch_is_another_animation_from_the_side_of_its_own_tab() {
        let (mut switch, now) = (PanelSwitch::default(), Instant::now());
        switch.begin(0, 2, now);
        switch.begin(2, 1, now + Duration::from_millis(30));
        assert_eq!(switch.playing(now + Duration::from_millis(40)), Some((Side::Left, 2)));
        assert_eq!(switch.count(), 2);
        switch.cut();
        assert_eq!(switch.playing(now), None, "a cut ends it");
        assert_eq!(switch.count(), 2, "and the count is what began");
    }
}
