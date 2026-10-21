//! Pictures that arrive in pieces: kitty's chunks (`m=1`) and iTerm2's multipart files. What is
//! half-read makes a repaint wait, and the pieces are held to the host's cap together.

use super::{apc, png, rgb, rig, uploads};
use crate::terminal::Limits;
use crate::terminal::base64;
use crate::terminal::repaint::NotReady;

/// A terminal whose host allows pictures of at most `cap` bytes.
fn capped(cap: usize) -> crate::terminal::tests::Rig {
    let mut r = rig(40, 6);
    r.term.set_limits(Limits { image_payload: cap, ..Limits::default() });
    r
}

#[test]
fn a_repaint_waits_while_a_kitty_transmission_is_in_chunks() {
    let mut r = rig(40, 6);
    assert_eq!(r.term.repaint_ready(), Ok(()));
    r.feed(&apc("a=t,f=24,s=1,v=1,i=1,m=1", &rgb(1, 1)));
    assert_eq!(r.term.repaint_ready(), Err(NotReady::Picture));
    r.feed(&apc("m=0", &[]));
    assert_eq!(r.term.repaint_ready(), Ok(()), "the last chunk ends the wait");
    assert_eq!(uploads(&mut r).len(), 1);
}

#[test]
fn a_repaint_waits_while_an_iterm_multipart_file_is_under_way() {
    let mut r = rig(40, 6);
    r.feed("\x1b]1337;MultipartFile=inline=1\x07");
    assert_eq!(r.term.repaint_ready(), Err(NotReady::Picture));
    r.feed(&format!("\x1b]1337;FilePart={}\x07", base64::encode(&png(8, 16))));
    assert_eq!(r.term.repaint_ready(), Err(NotReady::Picture));
    r.feed("\x1b]1337;FileEnd\x07");
    assert_eq!(r.term.repaint_ready(), Ok(()));
    assert_eq!(uploads(&mut r).len(), 1);
}

#[test]
fn chunks_that_add_up_to_exactly_the_payload_cap_are_accepted() {
    // 4 by 17 pixels of RGB are 204 bytes, in two chunks of 102.
    let send = |cap: usize| {
        let mut r = capped(cap);
        r.feed(&apc("a=t,f=24,s=4,v=17,i=3,m=1", &[0x7F; 102]));
        r.feed(&apc("m=0", &[0x7F; 102]));
        (uploads(&mut r).len(), r.replies())
    };
    assert_eq!(send(204), (1, "\x1b_Gi=3;OK\x1b\\".to_string()));
    let (sent, reply) = send(203);
    assert_eq!(sent, 0, "a byte over");
    assert!(reply.contains("EFBIG"), "{reply:?}");
}

#[test]
fn the_parts_of_a_multipart_file_are_added_up_against_the_cap_not_multiplied() {
    // A cap of 30 bytes allows 44 characters of base64, which is what a 33-byte file takes; the
    // two halves of 22 would come to 484 if they were multiplied.
    let text = base64::encode(&png(10, 10));
    let (first, second) = text.split_at(22);
    let send = |cap: usize| {
        let mut r = capped(cap);
        r.feed("\x1b]1337;MultipartFile=inline=1\x07");
        r.feed(&format!("\x1b]1337;FilePart={first}\x07\x1b]1337;FilePart={second}\x07"));
        r.feed("\x1b]1337;FileEnd\x07");
        uploads(&mut r).len()
    };
    assert_eq!(send(30), 1);
    assert_eq!(send(29), 0, "44 characters against 40: the file is dropped");
}
