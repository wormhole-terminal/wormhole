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
