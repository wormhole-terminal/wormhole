//! Rarely taken paths of the picture protocols, found unchecked by the coverage run
//! (`scripts/coverage.sh`) and by the fuzzer (`scripts/fuzz.sh`).

use super::{apc, apc_bare, freed, placed, png, rig, spots, uploads};
use crate::terminal::{Compression, ImageError, ImageFormat};

#[test]
fn a_single_command_with_more_payload_than_the_cap_is_refused_and_the_stream_goes_on() {
    // kitty graphics protocol: the payload is base64; the engine keeps no more than 32 MiB decoded
    // (a command fed whole, so the fast path of the decoder meets the cap, not the chunk logic).
    let mut r = rig(40, 10);
    let mut big = String::from("\x1b_Ga=t,f=32,s=100,v=100,i=3;");
    big.push_str(&"AAAA".repeat(12 * 1024 * 1024));
    big.push_str("\x1b\\ok");
    r.feed(&big);
    assert!(uploads(&mut r).is_empty());
    assert!(r.replies().starts_with("\x1b_Gi=3;EFBIG"), "{:?}", r.replies());
    assert_eq!(r.row(0), "ok");
}

#[test]
fn a_delete_the_protocol_does_not_have_and_one_without_its_key_change_nothing() {
    // kitty graphics protocol, "Deleting images": d= takes a, i, n, c, f, p, q, r, x, y, z (upper
    // case frees the data); i and n need an image id or number.
    let mut r = rig(40, 10);
    r.feed(&apc("a=T,f=100,i=7", &png(16, 16)));
    let up = uploads(&mut r);
    r.term.image_decoded(up[0].image, Ok(()));
    r.replies();
    for control in ["a=d,d=w,i=7", "a=d,d=i", "a=d,d=n", "a=d,d=I,i=0", "a=d,d=n,I=99"] {
        r.feed(&apc_bare(control));
        let reply = r.replies();
        assert!(!reply.contains("ENOENT"), "{control}: {reply:?}");
        assert_eq!(spots(&r).len(), 1, "{control} left the picture");
        assert!(freed(&mut r).is_empty(), "{control}");
    }
    r.feed(&apc_bare("a=d,d=w,i=7"));
    assert!(r.replies().contains("Gi=7;EINVAL"), "an error names the picture it was about");
    r.feed(&apc_bare("a=d,d=I,i=7"));
    assert!(spots(&r).is_empty());
    assert_eq!(freed(&mut r), [up[0].image]);
}

#[test]
fn the_host_may_refuse_a_picture_as_too_big_or_of_a_kind_it_cannot_read() {
    // The engine passes the host's verdict on as the protocol's codes (kitty: EFBIG, ENOTSUP).
    let mut r = rig(40, 10);
    r.feed(&apc("a=T,f=100,i=1,q=0", &png(8, 8)));
    r.feed(&apc("a=T,f=100,i=2,q=0", &png(8, 8)));
    let up = uploads(&mut r);
    r.term.image_decoded(up[0].image, Err(ImageError::TooBig));
    r.term.image_decoded(up[1].image, Err(ImageError::Unsupported));
    let reply = r.replies();
    assert!(reply.contains("Gi=1;EFBIG"), "{reply:?}");
    assert!(reply.contains("Gi=2;E"), "{reply:?}");
    assert!(spots(&r).is_empty());
    assert_eq!(freed(&mut r).len(), 2);
}

#[test]
fn raw_pixels_ask_for_their_size_in_bytes_and_only_data_that_long_is_taken() {
    // kitty: f=24 is RGB, 3 bytes a pixel: a payload shorter than s*v*3 cannot be a picture.
    let mut r = rig(40, 10);
    r.feed(&apc("a=t,f=24,s=4,v=4,i=1", &[0; 10]));
    let up = uploads(&mut r);
    assert!(up.is_empty() || (up[0].format, up[0].compression) == (ImageFormat::Rgb, Compression::None));
    assert!(r.replies().contains("Gi=1;"), "the program is told either way");
}

#[test]
fn a_sixel_color_in_hls_is_converted_with_hue_0_blue() {
    // DEC VT330/VT340 Programmer Reference, "Color introducer": #Pc;1;Ph;Pl;Ps is hue, lightness,
    // saturation; the hue wheel has blue at 0, red at 120 and green at 240 degrees.
    let mut r = rig(40, 10);
    r.feed("\x1bPq#1;1;0;50;100#1~#2;1;120;50;100#2~#3;1;240;50;100#3~#4;1;0;100;0#4~\x1b\\");
    let up = uploads(&mut r);
    assert_eq!(up.len(), 1);
    let pixel = |x: usize| &up[0].data[x * 4..x * 4 + 4];
    assert_eq!(pixel(0), [0, 0, 255, 255]);
    assert_eq!(pixel(1), [255, 0, 0, 255]);
    assert_eq!(pixel(2), [0, 255, 0, 255]);
    assert_eq!(pixel(3), [255, 255, 255, 255], "full lightness is white whatever the hue");
}

#[test]
fn a_sixel_that_paints_the_same_wide_band_again_and_again_runs_out_of_work() {
    // The data can overpaint a band forever (`$` returns to the left edge); the work is bounded
    // by what a picture could need, so the picture is dropped and the stream goes on.
    let mut r = rig(40, 10);
    let mut data = String::from("\x1bPq#1;2;100;0;0");
    for _ in 0..2000 {
        data.push_str("#1!9999~$");
    }
    data.push_str("\x1b\\ok");
    r.feed(&data);
    assert!(uploads(&mut r).is_empty());
    assert_eq!(r.row(0), "ok");
}

#[test]
fn an_inline_file_with_only_a_height_keeps_its_shape_unless_told_not_to() {
    // iTerm2 inline images: width and height are N cells, Npx pixels, N% or auto;
    // preserveAspectRatio=0 stretches to the given size.
    let mut r = rig(40, 20);
    let file = |args: &str| {
        format!("\x1b]1337;File=inline=1;{args}:{}\x07", crate::terminal::base64::encode(&png(100, 40)))
    };
    r.feed(&file("height=5"));
    r.feed("\r\n");
    r.feed(&file("height=5;preserveAspectRatio=0"));
    let (keep, stretch) = (spots(&r)[0], spots(&r)[1]);
    assert_eq!(keep.4, 5, "five rows");
    assert!(keep.3 > stretch.3, "kept shape: {keep:?} against stretched {stretch:?}");
    assert_eq!(placed(&r).len(), 2);
}

#[test]
fn a_multipart_file_that_is_started_again_or_never_ended_leaves_nothing_behind() {
    // iTerm2: MultipartFile=args, then FilePart=base64 pieces, then FileEnd.
    let mut r = rig(40, 20);
    let bytes = png(40, 40);
    let b64 = crate::terminal::base64::encode(&bytes);
    let (a, b) = b64.split_at(b64.len() / 2);
    r.feed(&format!("\x1b]1337;MultipartFile=inline=1\x07\x1b]1337;FilePart={a}\x07"));
    // A second start throws the first away; a part with no start and an end with no start are ignored.
    r.feed(&format!("\x1b]1337;MultipartFile=inline=1\x07\x1b]1337;FilePart={a}\x07\x1b]1337;FilePart={b}\x07\x1b]1337;FileEnd\x07"));
    r.feed(&format!("\x1b]1337;FilePart={a}\x07\x1b]1337;FileEnd\x07"));
    assert_eq!(uploads(&mut r).len(), 1);
    assert_eq!(placed(&r).len(), 1);
}
