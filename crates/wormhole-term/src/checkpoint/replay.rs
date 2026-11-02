//! Drawing a checkpoint's screen before a shell starts: VT with colors and attributes that ends with its own
//! divider. Only text, line ends and SGR get in (`vt::safe_replay`), and what the engine queued is dropped after,
//! as for the text of `restore::replay`.

use crate::screen::Screen;

pub(crate) fn replay_styled(screen: &mut Screen, vt: &str) {
    if vt.trim().is_empty() {
        return;
    }
    screen.feed(&super::vt::safe_replay(vt.as_bytes()));
    screen.discard_output();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GridSize;

    fn screen() -> Screen {
        Screen::new(GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 }, 100)
    }

    #[test]
    fn a_styled_seed_keeps_its_colors_and_acts_on_nothing() {
        let mut screen = screen();
        replay_styled(
            &mut screen,
            "\x1b[1;31mred\x1b[0m\r\n\x1b[6n\x1b]52;c;?\x07\x1b]0;t\x07\x1b[?1000hend\r\n",
        );
        assert!(screen.queued_replies().is_empty() && screen.drain_events().is_empty());
        let drawn = crate::contents::screen_text(&screen);
        assert!(drawn.contains("red") && drawn.contains("end"), "{drawn}");
        let mut cells = Vec::new();
        screen.copy_row(wormhole_vt::Line(0), &mut cells, None);
        assert!(cells[0].flags.contains(wormhole_vt::Flags::BOLD), "the colors and attributes are drawn");
        assert!(!screen.mode().contains(wormhole_vt::Modes::MOUSE_CLICK), "no mode was set");
    }

    #[test]
    fn the_divider_names_the_time_on_one_line() {
        assert_eq!(super::super::divider("a\nb"), "──── restored ab ────");
        assert!(crate::restore::is_marker(&super::super::divider("2026-11-02 14:03")));
    }
}
