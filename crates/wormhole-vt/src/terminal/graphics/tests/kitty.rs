//! Transmitting: kitty, "Transmitting images", "Querying support", "Terminal Responses".

use super::{apc, apc_bare, freed, png, rgb, rig, spots, uploads};
use crate::terminal::{Compression, ImageError, ImageFormat};
use crate::types::ImageHandle;

#[test]
fn a_direct_rgb_transmission_reaches_the_host_and_is_answered() {
    // kitty: "a=t … f=24 … s,v … i=" ; the answer is `OK` for an image with an id.
    let mut r = rig(40, 10);
    let pixels = rgb(4, 2);
    r.feed(&apc("a=t,f=24,s=4,v=2,i=7", &pixels));
    let up = uploads(&mut r);
    assert_eq!(up.len(), 1);
    assert_eq!(
        (up[0].format, up[0].compression, up[0].width, up[0].height),
        (ImageFormat::Rgb, Compression::None, 4, 2)
    );
    assert_eq!(up[0].data, pixels);
    assert_eq!(r.replies(), "\x1b_Gi=7;OK\x1b\\");
    assert!(spots(&r).is_empty(), "a=t stores, it does not place");
}

#[test]
fn a_command_without_an_id_or_number_gets_no_answer() {
    let mut r = rig(40, 10);
    r.feed(&apc("a=t,f=32,s=1,v=1", &[1, 2, 3, 4]));
    assert_eq!(uploads(&mut r).len(), 1);
    assert_eq!(r.replies(), "");
    r.feed(&apc("a=t,f=32,s=1,v=1,i=5,q=1", &[1, 2, 3, 4]));
    assert_eq!(r.replies(), "", "q=1 keeps the OK back");
    r.feed(&apc("a=t,f=32,s=1,v=1,i=5,q=1", &[1, 2, 3]));
    assert!(r.replies().starts_with("\x1b_Gi=5;ENODATA"), "q=1 still tells of errors");
    r.feed(&apc("a=t,f=32,s=1,v=1,i=5,q=2", &[1, 2, 3]));
    assert_eq!(r.replies(), "", "q=2 keeps back errors too");
}

#[test]
fn chunks_are_put_together_and_answered_once_at_the_end() {
    // kitty: "m=1 … the remaining chunks only need m (and optionally q)".
    let mut r = rig(40, 10);
    let pixels: Vec<u8> = (0..4 * 3 * 3).map(|n| n as u8).collect();
    let (a, rest) = pixels.split_at(10);
    let (b, c) = rest.split_at(11);
    r.feed(&apc("a=t,f=24,s=4,v=3,i=2,m=1", a));
    assert_eq!(r.replies(), "");
    r.feed(&apc("m=1", b));
    assert!(uploads(&mut r).is_empty(), "nothing before the last chunk");
    r.feed(&apc("m=0", c));
    let up = uploads(&mut r);
    assert_eq!(up.len(), 1);
    assert_eq!(up[0].data, pixels);
    assert_eq!(r.replies(), "\x1b_Gi=2;OK\x1b\\");
}

#[test]
fn a_chunked_transmission_that_runs_over_the_cap_is_refused_whole() {
    let mut r = rig(40, 10);
    let block = vec![0u8; 3 * 1024 * 1024];
    r.feed(&apc("a=t,f=32,s=100,v=100,i=3,m=1,q=0", &block));
    for _ in 0..12 {
        r.feed(&apc("m=1", &block));
    }
    r.feed(&apc("m=0", &block));
    assert!(uploads(&mut r).is_empty());
    assert!(r.replies().starts_with("\x1b_Gi=3;EFBIG"));
    // The transfer is over: the next command is a command again.
    r.feed(&apc("a=t,f=32,s=1,v=1,i=4", &[0; 4]));
    assert_eq!(uploads(&mut r).len(), 1);
}

#[test]
fn a_png_is_answered_when_the_host_has_decoded_it_and_later_replies_wait() {
    // The host decodes PNG; the answer waits for it, and so does everything asked after.
    let mut r = rig(40, 10);
    r.feed(&apc("a=t,f=100,i=9", &png(30, 20)));
    let up = uploads(&mut r);
    assert_eq!((up[0].format, up[0].width, up[0].height), (ImageFormat::Png, 30, 20));
    r.feed("\x1b[5n");
    assert_eq!(r.replies(), "", "held behind the picture's answer");
    r.term.image_decoded(up[0].image, Ok(()));
    assert_eq!(r.replies(), "\x1b_Gi=9;OK\x1b\\\x1b[0n");
    r.feed("\x1b[5n");
    assert_eq!(r.replies(), "\x1b[0n", "and the gate is open again");
}

#[test]
fn a_picture_the_host_cannot_decode_is_forgotten_and_the_program_told() {
    let mut r = rig(40, 10);
    r.feed(&apc("a=T,f=100,i=9", &png(16, 16)));
    let up = uploads(&mut r);
    assert_eq!(spots(&r).len(), 1);
    r.term.image_decoded(up[0].image, Err(ImageError::BadData));
    assert_eq!(r.replies(), "\x1b_Gi=9;ENODATA:the data could not be decoded as that format\x1b\\");
    assert!(spots(&r).is_empty(), "its placements go");
    assert_eq!(freed(&mut r), [up[0].image]);
    // A picture decoded after it was deleted still settles the answer.
    r.feed(&apc("a=t,f=100,i=10", &png(16, 16)));
    let up = uploads(&mut r);
    r.feed(&apc_bare("a=d,d=I,i=10"));
    r.term.image_decoded(up[0].image, Ok(()));
    assert!(r.replies().contains("Gi=10;OK"));
}

#[test]
fn zlib_compressed_raw_data_is_the_hosts_to_inflate() {
    let mut r = rig(40, 10);
    r.feed(&apc("a=t,f=24,s=10,v=10,o=z,i=1", &[0x78, 0x9C, 1, 2, 3]));
    let up = uploads(&mut r);
    assert_eq!(up[0].compression, Compression::Zlib);
    assert_eq!((up[0].width, up[0].height), (10, 10));
    assert_eq!(r.replies(), "", "waits: the size is only known once inflated");
    r.term.image_decoded(up[0].image, Err(ImageError::BadData));
    assert!(r.replies().contains("ENODATA"));
}

#[test]
fn a_query_checks_without_storing() {
    // kitty: "a=q … the terminal replies with the id, OK or an error; the image is not stored".
    let mut r = rig(40, 10);
    r.feed(&apc("a=q,i=31,s=1,v=1,t=d,f=24", &[0, 0, 0]));
    assert_eq!(r.replies(), "\x1b_Gi=31;OK\x1b\\");
    assert!(uploads(&mut r).is_empty());
    r.feed(&apc("a=q,i=32,s=1,v=1,t=d,f=24", &[0, 0]));
    assert!(r.replies().starts_with("\x1b_Gi=32;ENODATA"));
    r.feed(&apc("a=q,i=33,f=100", &png(5, 5)));
    assert_eq!(r.replies(), "\x1b_Gi=33;OK\x1b\\");
    r.feed(&apc("a=q,i=34,f=100", b"not a png"));
    assert!(r.replies().starts_with("\x1b_Gi=34;ENODATA"));
    // The usual probe: a query, then DA1; both answers arrive, in that order.
    r.feed(&format!("{}\x1b[c", apc("a=q,i=1,s=1,v=1,f=24", &[0, 0, 0])));
    assert_eq!(r.replies(), "\x1b_Gi=1;OK\x1b\\\x1b[?65;1;4;6;9;15;16;17;18;21;22;28;29;52c");
}

#[test]
fn what_the_terminal_does_not_do_is_refused_with_the_protocols_codes() {
    let mut r = rig(40, 10);
    for (control, code) in [
        ("a=t,t=f,f=100,i=1", "EBADF"),
        ("a=t,t=t,f=100,i=1", "EBADF"),
        ("a=t,t=s,f=100,i=1", "EBADF"),
        ("a=f,i=1", "EBADF"),
        ("a=a,i=1", "EBADF"),
        ("a=c,i=1", "EBADF"),
        ("a=T,f=24,s=1,v=1,i=1,P=2", "EBADF"),
        ("a=t,f=77,s=1,v=1,i=1", "EINVAL"),
        ("a=t,f=24,i=1", "EINVAL"),
        ("a=t,f=24,s=0,v=3,i=1", "EINVAL"),
        ("a=t,f=24,s=20000,v=1,i=1", "EFBIG"),
        ("a=t,f=24,s=9000,v=9000,i=1", "EFBIG"),
        ("a=t,f=100,o=z,i=1", "EINVAL"),
        ("a=t,f=100,i=1", "ENODATA"),
        ("a=z,i=1", "EINVAL"),
        ("a=p,i=1", "ENOENT"),
        ("a=p", ""),
    ] {
        r.feed(&apc(control, &[1, 2, 3]));
        let reply = r.replies();
        if code.is_empty() {
            assert_eq!(reply, "", "no id, no answer ({control})");
        } else {
            assert!(reply.contains(code), "{control}: {reply:?}");
        }
    }
    assert!(uploads(&mut r).is_empty());
}

#[test]
fn a_numbered_image_is_given_an_id_and_the_newest_with_a_number_is_used() {
    // kitty: "I … the terminal responds with i=<assigned id>,I=<number>".
    let mut r = rig(40, 10);
    r.feed(&apc("a=t,f=24,s=1,v=1,I=13", &rgb(1, 1)));
    assert_eq!(r.replies(), "\x1b_Gi=1,I=13;OK\x1b\\");
    r.feed(&apc("a=t,f=24,s=1,v=1,I=13", &rgb(1, 1)));
    assert_eq!(r.replies(), "\x1b_Gi=2,I=13;OK\x1b\\");
    r.feed(&apc_bare("a=p,I=13,p=4"));
    assert_eq!(r.replies(), "\x1b_Gi=2,I=13,p=4;OK\x1b\\");
    let up = uploads(&mut r);
    assert_eq!(super::placed(&r)[0].image, up[1].image, "the newest picture with that number");
    r.feed(&apc("a=t,f=24,s=1,v=1,i=5,I=1", &rgb(1, 1)));
    assert!(r.replies().contains("EINVAL"), "both an id and a number");
}

#[test]
fn sending_a_picture_again_under_its_id_replaces_it_and_its_placements() {
    let mut r = rig(40, 10);
    r.feed(&apc("a=T,f=24,s=8,v=16,i=1", &rgb(8, 16)));
    let first = uploads(&mut r)[0].image;
    assert_eq!(spots(&r).len(), 1);
    r.feed(&apc("a=t,f=24,s=8,v=16,i=1", &rgb(8, 16)));
    assert_eq!(freed(&mut r), [first]);
    assert!(spots(&r).is_empty());
}

#[test]
fn handles_are_never_reused() {
    let mut r = rig(40, 10);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..5 {
        r.feed(&apc("a=t,f=24,s=1,v=1,i=1", &rgb(1, 1)));
        for up in uploads(&mut r) {
            assert!(seen.insert(up.image), "{:?}", up.image);
        }
    }
    assert!(seen.iter().all(|h: &ImageHandle| h.get() > 0));
}

#[test]
fn replies_that_are_not_about_pictures_keep_their_order() {
    let mut r = rig(40, 10);
    r.feed(&format!("\x1b[5n{}\x1b[5n", apc("a=t,f=24,s=1,v=1,i=1", &rgb(1, 1))));
    assert_eq!(r.replies(), "\x1b[0n\x1b_Gi=1;OK\x1b\\\x1b[0n");
}
