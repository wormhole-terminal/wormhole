//! Limits and hostile input: pictures come from programs that may be broken or hostile.

use super::{apc, apc_bare, freed, png, rgb, rig, spots, uploads};
use crate::grid::MAX_PLACEMENTS;
use crate::terminal::Event;
use crate::terminal::{IMAGE_BUDGET_BYTES, MAX_IMAGES};

/// A deterministic byte generator.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u8 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u8
    }
}

#[test]
fn random_bytes_in_every_protocol_never_panic() {
    let mut rng = Lcg(7);
    let openers: [&[u8]; 5] =
        [b"\x1b_G", b"\x1bPq", b"\x1bP9;1;2q", b"\x1b]1337;File=inline=1:", b"\x1b]1337;MultipartFile="];
    for round in 0..300 {
        let mut r = rig(20, 6);
        let mut bytes = Vec::new();
        for _ in 0..10 {
            bytes.extend_from_slice(openers[rng.next() as usize % openers.len()]);
            let len = rng.next() as usize * 4;
            // Mostly plausible characters, some anything.
            bytes.extend((0..len).map(|_| match rng.next() % 8 {
                0 => rng.next(),
                1 => b",;=".get(rng.next() as usize % 3).copied().unwrap_or(b','),
                _ => {
                    b"abcdefgiIpqtmzxyCUDA0123456789+/".get(rng.next() as usize % 32).copied().unwrap_or(b'A')
                }
            }));
            bytes.extend_from_slice(if rng.next().is_multiple_of(2) { b"\x1b\\" } else { b"\x07" });
        }
        for piece in bytes.chunks(1 + round % 97) {
            r.feed_bytes(piece);
        }
        for up in uploads(&mut r) {
            r.term.image_decoded(up.image, Ok(()));
        }
        r.feed("\x1b[5n");
        r.replies();
    }
}

#[test]
fn commands_with_extreme_numbers_never_panic() {
    let mut r = rig(20, 6);
    r.feed(&apc("a=t,f=24,s=2,v=2,i=1", &rgb(2, 2)));
    for control in [
        "a=p,i=1,c=4294967295,r=4294967295",
        "a=p,i=1,x=4294967295,y=4294967295,w=4294967295,h=4294967295",
        "a=p,i=1,X=4294967295,Y=4294967295,z=-2147483648",
        "a=p,i=1,c=1,r=4294967295",
        "a=p,i=1,r=4294967295",
        "a=d,d=p,x=0,y=0",
        "a=d,d=q,x=4294967295,y=4294967295,z=2147483647",
        "a=d,d=r,x=4294967295,y=0",
        "a=d,d=x,x=0",
        "a=t,f=24,s=4294967295,v=4294967295,i=2",
        "a=t,f=32,s=65535,v=65535,i=2",
    ] {
        r.feed(&apc_bare(control));
        r.feed("\x1b[1;1H\n\n\n");
    }
    r.term.resize(3, 2);
    r.term.resize(200, 50);
}

#[test]
fn no_more_pictures_than_the_count_cap_and_the_least_recently_used_goes_first() {
    let mut r = rig(40, 6);
    for id in 1..=MAX_IMAGES as u32 + 20 {
        r.feed(&apc(&format!("a=t,f=24,s=1,v=1,i={id},q=2"), &rgb(1, 1)));
    }
    assert_eq!(freed(&mut r).len(), 20);
    r.feed(&apc_bare("a=p,i=1"));
    assert!(r.replies().contains("ENOENT"), "the oldest went");
    r.feed(&apc_bare("a=p,i=1030,q=0"));
    assert!(r.replies().contains("OK"));
}

#[test]
fn a_picture_with_a_placement_outlives_one_without() {
    let mut r = rig(40, 6);
    r.feed(&apc("a=T,f=24,s=8,v=16,i=1,q=2", &rgb(8, 16)));
    for id in 2..=MAX_IMAGES as u32 {
        r.feed(&apc(&format!("a=t,f=24,s=1,v=1,i={id},q=2"), &rgb(1, 1)));
    }
    r.events();
    r.feed(&apc("a=t,f=24,s=1,v=1,i=5000,q=2", &rgb(1, 1)));
    let gone = freed(&mut r);
    assert_eq!(gone.len(), 1);
    assert_eq!(spots(&r).len(), 1, "the placed one stayed; picture 2 went instead");
    r.feed(&apc_bare("a=p,i=2"));
    assert!(r.replies().contains("ENOENT"));
}

#[test]
fn pictures_over_the_byte_budget_evict_the_oldest() {
    let mut r = rig(40, 6);
    // 9 pictures of 4,000 x 4,000 pixels would be 576 MiB decoded; the screen keeps 128 MiB.
    let pixels = 3_000u32 * 1_000;
    let each = u64::from(pixels) * 4;
    let fit = (IMAGE_BUDGET_BYTES / each) as u32;
    let mut gone = 0;
    for id in 1..=fit + 3 {
        r.feed(&apc(&format!("a=t,f=100,i={id},q=2"), &png(3_000, 1_000)));
        for event in r.events() {
            match event {
                crate::terminal::Event::ImageData(up) => r.term.image_decoded(up.image, Ok(())),
                crate::terminal::Event::ImageFree(_) => gone += 1,
                _ => {}
            }
        }
    }
    assert_eq!(gone, 3);
    // One that cannot fit at all is refused, and nothing else is lost for it.
    r.feed(&apc("a=t,f=100,i=99,q=0", &png(10_000, 3_000)));
    assert!(r.replies().contains("EFBIG"));
    assert!(freed(&mut r).is_empty());
}

#[test]
fn a_host_that_never_drains_cannot_make_the_terminal_hold_more_than_its_cap() {
    let mut r = rig(40, 6);
    let block = vec![0u8; 22 * 1024 * 1024];
    let mut accepted = 0;
    for id in 1..=30 {
        r.feed(&apc(&format!("a=t,f=24,s=4000,v=1750,i={id},q=0"), &block[..4000 * 1750 * 3]));
        accepted += usize::from(r.replies().contains("OK"));
    }
    assert!(accepted > 0 && accepted < 30, "{accepted}");
}

#[test]
fn the_oldest_placements_go_past_the_cap() {
    let mut r = rig(40, 6);
    r.feed(&apc("a=t,f=24,s=1,v=1,i=1,q=2", &rgb(1, 1)));
    for _ in 0..MAX_PLACEMENTS + 10 {
        r.feed(&apc_bare("a=p,i=1,C=1,q=2"));
    }
    assert!(r.term.grid().placement_count() <= MAX_PLACEMENTS);
}

#[test]
fn an_unfinished_command_is_dropped_by_cancel_and_the_next_is_read() {
    let mut r = rig(40, 6);
    r.feed("\x1b_Ga=t,f=24,s=1,v=1,i=1;AAAA\x18");
    r.feed(&apc("a=t,f=24,s=1,v=1,i=2", &rgb(1, 1)));
    let up = uploads(&mut r);
    assert_eq!(up.len(), 1);
    assert!(r.replies().contains("Gi=2;OK"));
}

#[test]
fn a_transmission_left_open_does_not_swallow_the_next_command_for_ever() {
    // The first chunk says more are coming; they never do. A later command that is not a chunk
    // still has to be read as the next chunk by the protocol, so it ends the transfer.
    let mut r = rig(40, 6);
    r.feed(&apc("a=t,f=24,s=2,v=2,i=1,m=1", &rgb(1, 1)));
    r.feed(&apc("m=0", &rgb(1, 3)));
    assert_eq!(uploads(&mut r).len(), 1);
    assert!(r.replies().contains("Gi=1;OK"));
}

#[test]
fn a_full_event_queue_still_tells_the_host_which_pictures_go() {
    let mut r = rig(40, 6);
    for id in 1..=100 {
        r.feed(&apc(&format!("a=t,f=24,s=1,v=1,i={id},q=2"), &rgb(1, 1)));
    }
    assert_eq!(uploads(&mut r).len(), 100);
    // The host is not looking, and a program fills the queue with things it can lose.
    r.feed(&"\x1b]1337;SetUserVar=a=Yg==\x07".repeat(20_000));
    r.feed(&apc_bare("a=d,d=A,q=2"));
    r.feed("\x1bc\x1bc\x1bc");
    let events = r.events();
    assert!(events.len() >= 16_384, "the queue was full");
    let count = |want: fn(&Event) -> bool| events.iter().filter(|event| want(event)).count();
    assert_eq!(count(|event| matches!(event, Event::ImageFree(_))), 100);
    assert_eq!(count(|event| matches!(event, Event::FullReset)), 1, "resets in a row are one");
}
// ---- the host's limits (`Limits`) ----

/// Limits with one change from the ceilings.
fn lowered(change: impl FnOnce(&mut crate::terminal::Limits)) -> crate::terminal::Limits {
    let mut limits = crate::terminal::Limits::default();
    change(&mut limits);
    limits
}

/// Store a PNG header of this size under `id` and hand the host's answer back.
fn store(r: &mut crate::terminal::tests::Rig, id: u32, w: u32, h: u32) {
    r.feed(&apc(&format!("a=t,f=100,i={id},q=2"), &png(w, h)));
    for up in uploads(r) {
        r.term.image_decoded(up.image, Ok(()));
    }
}

#[test]
fn lowering_the_byte_budget_evicts_the_oldest_pictures_at_once() {
    let mut r = rig(40, 6);
    // 1,000 x 1,000 pixels: 4 MB each.
    for id in 1..=5 {
        store(&mut r, id, 1000, 1000);
    }
    assert!(freed(&mut r).is_empty());
    r.term.set_limits(lowered(|l| l.image_bytes = 9_000_000));
    assert_eq!(freed(&mut r).len(), 3, "two fit in 9 MB; the three oldest went");
    r.feed(&apc_bare("a=p,i=3"));
    assert!(r.replies().contains("ENOENT"));
    r.feed(&apc_bare("a=p,i=4"));
    assert!(r.replies().contains("OK"));
    // The budget is the host's from now on: a picture over it is refused, others make room.
    r.feed(&apc("a=t,f=100,i=50,q=0", &png(3000, 1000)));
    assert!(r.replies().contains("EFBIG"));
    assert!(freed(&mut r).is_empty(), "a refused picture evicts nothing");
    r.feed(&apc("a=t,f=100,i=51,q=2", &png(1000, 1000)));
    assert_eq!(freed(&mut r).len(), 1, "8 MB held and 4 MB more: one leaves");
}

#[test]
fn lowering_the_count_evicts_what_is_not_placed_first() {
    let mut r = rig(40, 6);
    r.feed(&apc("a=T,f=24,s=8,v=16,i=1,q=2", &rgb(8, 16)));
    for id in 2..=6 {
        r.feed(&apc(&format!("a=t,f=24,s=1,v=1,i={id},q=2"), &rgb(1, 1)));
    }
    r.events();
    r.term.set_limits(lowered(|l| l.image_count = 2));
    assert_eq!(freed(&mut r).len(), 4);
    assert_eq!(spots(&r).len(), 1, "the placed picture stayed");
    r.feed(&apc_bare("a=p,i=6,C=1"));
    assert!(r.replies().contains("OK"), "the newest of the others stayed");
    r.feed(&apc_bare("a=p,i=2"));
    assert!(r.replies().contains("ENOENT"));
    // The count holds for new pictures too.
    r.feed(&apc("a=t,f=24,s=1,v=1,i=9,q=2", &rgb(1, 1)));
    assert_eq!(freed(&mut r).len(), 1);
}

#[test]
fn the_picture_limits_reach_both_screens() {
    let mut r = rig(40, 6);
    for id in 1..=4 {
        r.feed(&apc(&format!("a=t,f=24,s=1,v=1,i={id},q=2"), &rgb(1, 1)));
    }
    r.feed("\x1b[?1049h");
    for id in 1..=4 {
        r.feed(&apc(&format!("a=t,f=24,s=1,v=1,i={id},q=2"), &rgb(1, 1)));
    }
    r.events();
    r.term.set_limits(lowered(|l| l.image_count = 1));
    assert_eq!(freed(&mut r).len(), 6);
}

#[test]
fn a_transmission_over_the_hosts_payload_cap_is_refused() {
    let mut r = rig(40, 6);
    r.term.set_limits(lowered(|l| l.image_payload = 200));
    r.feed(&apc("a=t,f=24,s=10,v=10,i=1", &rgb(10, 10)));
    assert!(r.replies().contains("EFBIG"), "300 bytes of pixels");
    assert!(uploads(&mut r).is_empty());
    r.feed(&apc("a=t,f=24,s=5,v=5,i=2", &rgb(5, 5)));
    assert!(r.replies().contains("Gi=2;OK"), "75 bytes");
    // Chunks add up.
    r.feed(&apc("a=t,f=24,s=9,v=9,i=3,m=1", &rgb(9, 4)));
    r.feed(&apc("m=1", &rgb(9, 4)));
    r.feed(&apc("m=0", &rgb(9, 1)));
    assert!(r.replies().contains("EFBIG"), "243 bytes in three chunks");
}

#[test]
fn chunks_that_add_up_to_less_than_the_payload_cap_are_accepted() {
    let mut r = rig(40, 6);
    r.term.set_limits(lowered(|l| l.image_payload = 200));
    // Three chunks of 30 bytes: 90 in all, under the cap (the chunks are not multiplied).
    r.feed(&apc("a=t,f=24,s=5,v=6,i=3,m=1", &rgb(5, 2)));
    r.feed(&apc("m=1", &rgb(5, 2)));
    r.feed(&apc("m=0", &rgb(5, 2)));
    assert!(r.replies().contains("Gi=3;OK"));
    assert_eq!(uploads(&mut r).len(), 1);
}

#[test]
fn an_inline_file_that_takes_exactly_the_encoding_of_the_payload_cap_is_shown() {
    let file =
        |bytes: &[u8]| format!("\x1b]1337;File=inline=1:{}\x07", crate::terminal::base64::encode(bytes));
    let mut r = rig(40, 6);
    // A cap of 30 bytes allows 44 characters of base64 (ten groups of four, and one more group for a
    // part): the 33 bytes of a PNG header are 44 characters, so they just fit; a cap of 29 allows 40.
    r.term.set_limits(lowered(|l| l.image_payload = 30));
    r.feed(&file(&png(10, 10)));
    assert_eq!(uploads(&mut r).len(), 1);
    r.term.set_limits(lowered(|l| l.image_payload = 29));
    r.feed(&file(&png(10, 10)));
    assert!(uploads(&mut r).is_empty(), "44 characters against 40");
}

#[test]
fn an_inline_file_over_the_hosts_caps_is_ignored() {
    let file =
        |bytes: &[u8]| format!("\x1b]1337;File=inline=1:{}\x07", crate::terminal::base64::encode(bytes));
    let mut r = rig(40, 6);
    r.term.set_limits(lowered(|l| l.image_payload = 20));
    r.feed(&file(&png(10, 10)));
    assert!(uploads(&mut r).is_empty(), "33 bytes of file against 20");
    let mut r = rig(40, 6);
    r.term.set_limits(lowered(|l| l.osc_image = 40));
    r.feed(&file(&png(10, 10)));
    assert!(uploads(&mut r).is_empty(), "the string is longer than the OSC cap for pictures");
    r.term.set_limits(crate::terminal::Limits::default());
    r.feed(&file(&png(10, 10)));
    assert_eq!(uploads(&mut r).len(), 1);
}

#[test]
fn a_sixel_over_the_hosts_cap_is_dropped() {
    // Nothing but the data between the header and the terminator: 22 bytes, then 62.
    let sixel = |bands: usize| format!("\x1bP9;1q\"1;1\n#1;2;100;0;0#1{}\x1b\\", "~".repeat(bands));
    let mut r = rig(40, 6);
    r.term.set_limits(lowered(|l| l.sixel = 40));
    r.feed(&sixel(5));
    assert_eq!(uploads(&mut r).len(), 1);
    r.feed(&sixel(40));
    assert!(uploads(&mut r).is_empty(), "the data is longer than 40 bytes");
}

#[test]
fn pictures_waiting_for_the_host_are_held_to_the_hosts_size() {
    let mut r = rig(40, 6);
    // The header of a PNG is 33 bytes: one fits in 50, two do not.
    r.term.set_limits(lowered(|l| l.image_uploads = 50));
    r.feed(&apc("a=t,f=100,i=1", &png(10, 10)));
    r.feed(&apc("a=t,f=100,i=2", &png(10, 10)));
    let first = uploads(&mut r);
    assert_eq!(first.len(), 1, "the second was refused");
    // The answers wait for the first picture's decode, in order.
    assert_eq!(r.replies(), "");
    r.term.image_decoded(first[0].image, Ok(()));
    let replies = r.replies();
    assert!(replies.contains("Gi=1;OK") && replies.contains("Gi=2;EFBIG"), "{replies:?}");
    // Taking the events freed the room.
    r.feed(&apc("a=t,f=100,i=3", &png(10, 10)));
    assert_eq!(uploads(&mut r).len(), 1);
}

/// What the host is handed, and what the program is told at once, for a PNG header of this size
/// when the host has set these limits.
fn png_sent(limits: impl FnOnce(&mut crate::terminal::Limits), w: u32, h: u32) -> (usize, String) {
    let mut r = rig(40, 6);
    r.term.set_limits(lowered(limits));
    r.feed(&apc("a=t,f=100,i=1,q=0", &png(w, h)));
    (uploads(&mut r).len(), r.replies())
}

#[test]
fn a_picture_may_have_the_most_pixels_the_sides_and_the_total_allow_and_no_more() {
    // 10,000 on a side, and 16 Mi pixels in all (4,096 square).
    for (w, h) in [(10_000, 1), (1, 10_000), (4096, 4096)] {
        let (sent, reply) = png_sent(|_| {}, w, h);
        assert_eq!((sent, reply.as_str()), (1, ""), "{w} by {h} waits for the host's decode");
    }
    for (w, h) in [(10_001, 1), (1, 10_001), (4097, 4096)] {
        let (sent, reply) = png_sent(|_| {}, w, h);
        assert_eq!(sent, 0, "{w} by {h}");
        assert!(reply.contains("EFBIG"), "{w} by {h}: {reply:?}");
    }
    // A picture with no pixels is no picture, whichever side says so.
    for (w, h) in [(0, 10), (10, 0)] {
        let (sent, reply) = png_sent(|_| {}, w, h);
        assert_eq!(sent, 0, "{w} by {h}");
        assert!(reply.contains("ENODATA"), "{w} by {h}: {reply:?}");
    }
}

#[test]
fn a_picture_that_takes_exactly_the_hosts_byte_budget_or_queue_is_kept() {
    // 10 by 10 pixels are 400 bytes decoded; its header, 33 bytes, is what waits for the host.
    assert_eq!(png_sent(|l| l.image_bytes = 400, 10, 10), (1, String::new()));
    let (sent, reply) = png_sent(|l| l.image_bytes = 400, 10, 11);
    assert_eq!(sent, 0);
    assert!(reply.contains("EFBIG"), "{reply:?}");
    assert_eq!(png_sent(|l| l.image_uploads = 33, 10, 10), (1, String::new()));
}
