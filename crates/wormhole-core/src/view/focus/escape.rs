//! Esc: a ladder, the nearest open thing first. Esc reaches the shell only when the terminal
//! already had the keyboard and nothing covers it; from anywhere else it ends up bringing the
//! keyboard back there.

/// Where things stand when Esc is pressed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EscState {
    /// The terminal has the keyboard.
    pub terminal: bool,
    /// A menu, popover or rename field is open.
    pub popup: bool,
    /// A diff or a find bar is open over the terminal.
    pub detail: bool,
}

/// What Esc does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    /// The terminal has the keyboard, and every key goes to its program, Esc included.
    ToShell,
    /// Close the menu, popover or rename field; the keyboard goes back to its row.
    ClosePopup,
    /// Close the diff or find bar; the keyboard stays in its list.
    CloseDetail,
    /// Give the keyboard to the terminal.
    ToTerminal,
}

/// The rung of the ladder Esc takes. Nothing open and the keyboard outside the terminal goes
/// to the terminal, whether it was in a list, on a button or nowhere at all (its row went away, a
/// click on bare chrome): the terminal is home, and Esc is how one gets back there. A diff covers
/// the terminal, so it closes first even while the terminal under it has the keys (a click on a
/// file leaves them there): "close the diff first", as its header's "esc close" says.
pub fn escape(state: EscState) -> Escape {
    if state.terminal && !state.detail {
        Escape::ToShell
    } else if state.popup {
        Escape::ClosePopup
    } else if state.detail {
        Escape::CloseDetail
    } else {
        Escape::ToTerminal
    }
}

#[cfg(test)]
mod tests {
    use super::{EscState, Escape, escape};

    #[test]
    fn each_rung_of_the_ladder() {
        let popup = EscState { popup: true, ..EscState::default() };
        let detail = EscState { detail: true, ..EscState::default() };
        let terminal = EscState { terminal: true, ..EscState::default() };
        assert_eq!(escape(popup), Escape::ClosePopup);
        assert_eq!(escape(detail), Escape::CloseDetail);
        assert_eq!(escape(EscState::default()), Escape::ToTerminal, "from a list or a button");
        assert_eq!(escape(terminal), Escape::ToShell);
    }

    #[test]
    fn the_nearest_open_thing_closes_first() {
        // A row's menu over an open diff: four presses take it all down and then reach the shell.
        let mut state = EscState { popup: true, detail: true, ..EscState::default() };
        assert_eq!(escape(state), Escape::ClosePopup);
        state.popup = false;
        assert_eq!(escape(state), Escape::CloseDetail);
        state.detail = false;
        assert_eq!(escape(state), Escape::ToTerminal);
        state.terminal = true;
        assert_eq!(escape(state), Escape::ToShell);
    }

    #[test]
    fn only_a_focused_terminal_sends_esc_to_the_shell() {
        for popup in [false, true] {
            for detail in [false, true] {
                let state = EscState { terminal: false, popup, detail };
                assert_ne!(escape(state), Escape::ToShell, "{state:?}");
            }
            let state = EscState { terminal: true, popup, detail: false };
            assert_eq!(escape(state), Escape::ToShell, "{state:?}");
        }
    }

    #[test]
    fn a_diff_over_the_terminal_closes_before_the_shell_hears_esc() {
        let state = EscState { terminal: true, detail: true, ..EscState::default() };
        assert_eq!(escape(state), Escape::CloseDetail);
    }
}
