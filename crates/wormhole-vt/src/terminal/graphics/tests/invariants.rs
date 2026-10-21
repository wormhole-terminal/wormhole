//! The terminal's check of its own picture tables, which the tests of every other area lean on:
//! it has to find a table the engine would never make.

use super::{apc, rgb, rig};
use crate::terminal::graphics::images::Image;
use crate::types::ImageHandle;

fn picture(handle: u64, id: u32) -> Image {
    Image { handle: ImageHandle(handle), id, number: 0, width: 1, height: 1, used: 0, virtuals: Vec::new() }
}

#[test]
fn the_check_finds_a_wrong_table_and_says_which_screen_it_is_on() {
    let mut r = rig(40, 6);
    r.feed(&apc("a=t,f=24,s=1,v=1,i=1,q=2", &rgb(1, 1)));
    assert_eq!(r.term.check_invariants(), Ok(()));
    // Two pictures with one id, on each screen in turn.
    for slot in 0..2 {
        r.term.graphics.tables[slot].insert(picture(90, 9));
        r.term.graphics.tables[slot].insert(picture(91, 9));
        let error = r.term.check_invariants().expect_err("a picture id used twice");
        assert!(error.contains(&format!("pictures of screen {slot}: picture id 9")), "{error}");
        let table = &mut r.term.graphics.tables[slot];
        table.remove(table.find(ImageHandle(90)).expect("it was inserted"));
        table.remove(table.find(ImageHandle(91)).expect("it was inserted"));
        assert_eq!(r.term.check_invariants(), Ok(()), "and sound again without them");
    }
}
