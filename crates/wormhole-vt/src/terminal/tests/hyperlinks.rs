//! OSC 8: what makes a hyperlink another one, and how long its target may be.

use super::Rig;
use crate::terminal::{Limits, Terminal};
use crate::types::{Column, Line, Point};

/// The id and the target of the link under column `x` of the first row.
fn link_at(rig: &Rig, x: usize) -> Option<(String, String)> {
    let link = rig.term.grid().link_at(Point::new(Line(0), Column(x)))?;
    Some(((*link.id).to_string(), link.uri.to_string()))
}

#[test]
fn a_new_id_or_a_new_target_makes_a_new_link_and_the_same_pair_goes_on() {
    let mut rig = Rig::new(20, 3);
    rig.feed("\x1b]8;id=a;https://one.example\x07w");
    // The same id and another target.
    rig.feed("\x1b]8;id=a;https://two.example\x07x");
    // Another id and the same target.
    rig.feed("\x1b]8;id=b;https://two.example\x07y");
    // The same id and the same target again.
    rig.feed("\x1b]8;id=b;https://two.example\x07z");
    let link = |x: usize| link_at(&rig, x).expect("a link");
    assert_eq!(link(0), ("a".to_string(), "https://one.example".to_string()));
    assert_eq!(link(1), ("a".to_string(), "https://two.example".to_string()));
    assert_eq!(link(2), ("b".to_string(), "https://two.example".to_string()));
    assert_eq!(link(3), link(2));
}

#[test]
fn a_target_as_long_as_the_hosts_cap_is_a_link_and_one_byte_more_is_not() {
    let mut rig = Rig {
        term: Terminal::builder().size(40, 3).limits(Limits { link_uri: 20, ..Limits::default() }).build(),
    };
    // 20 bytes, then 21.
    rig.feed("\x1b]8;;https://a.example/ok\x07a\x1b]8;;\x07");
    rig.feed("\x1b]8;;https://a.example/ok1\x07b\x1b]8;;\x07");
    assert_eq!(link_at(&rig, 0).map(|(_, uri)| uri).as_deref(), Some("https://a.example/ok"));
    assert_eq!(link_at(&rig, 1), None);
}
