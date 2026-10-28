use super::decode;
use crate::platform::image::{MAX_SIDE, Refused, crafted};

/// A 16 x 16 picture as `sips -s format heic` writes it: red climbs with x, green with y, blue is 128.
const HEIC: [&str; 12] = [
    "000000246674797068656963000000006d6966314d6950726d6961664d69484268656963000001866d65746100000000",
    "0000002168646c72000000000000000070696374000000000000000000000000000000002464696e660000001c647265",
    "6600000000000000010000000c75726c20000000010000000e7069746d0000000000010000002369696e660000000000",
    "0100000015696e666502000000000100006876633100000000e669707270000000c56970636f00000013636f6c726e63",
    "6c78000200020006800000000c636c6c6900cb004000000014697370650000000000000010000000100000000969726f",
    "7400000000107069786900000000030808080000007168766343010370000000b000000000001ef000fcfdf8f800000b",
    "03a00001001740010c01ffff037000000300b0000003000003001e7024a100010023420101037000000300b000000300",
    "0003001ea0142041c0930ce21ee45954dc08081802a2000100094401c06172c84453640000001969706d610000000000",
    "0000010001068102030586840000001e696c6f630000000044000001000100000001000001ba0000005b000000016d64",
    "6174000000000000006b000000572801afa37d2a93eef62784c1245bd06703c1199070e93a34cd0a9cee578582f991d5",
    "b6ebe230b4faedb0957e3fc52347d237c85e3e1cdfffdba092d02fffb7987ac04d743eb8158fffde47aff1b955fb339b",
    "9c1cc57838",
];

fn heic() -> Vec<u8> {
    HEIC.concat()
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(width, height, image::Rgba([200, 40, 40, 255]))
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

#[test]
fn a_heic_picture_is_decoded_to_straight_rgba() {
    let decoded = decode(&heic()).expect("the system reads HEIC");
    assert_eq!((decoded.width, decoded.height), (16, 16));
    assert_eq!(decoded.rgba.len(), 16 * 16 * 4);
    let near = |a: u8, b: u8| a.abs_diff(b) <= 24;
    let pixel = |x: usize, y: usize| &decoded.rgba[(y * 16 + x) * 4..][..4];
    // Top left is (0, 0, 128); bottom right is (240, 240, 128): lossy, so near.
    assert!(
        near(pixel(0, 0)[0], 0) && near(pixel(0, 0)[1], 0) && near(pixel(0, 0)[2], 128),
        "{:?}",
        pixel(0, 0)
    );
    assert!(near(pixel(15, 15)[0], 240) && near(pixel(15, 15)[1], 240), "{:?}", pixel(15, 15));
    assert_eq!(pixel(0, 0)[3], 255);
}

#[test]
fn a_png_keeps_its_colors_and_a_small_picture_is_not_enlarged() {
    let decoded = decode(&png(3, 2)).expect("PNG");
    assert_eq!((decoded.width, decoded.height), (3, 2));
    assert_eq!(&decoded.rgba[..4], &[200, 40, 40, 255]);
}

#[test]
fn a_big_picture_is_scaled_down_while_it_is_read() {
    let decoded = decode(&png(6000, 24)).expect("PNG");
    assert_eq!(decoded.width, MAX_SIDE);
    assert!(decoded.height <= 24 && decoded.height >= 14, "{}", decoded.height);
}

#[test]
fn a_header_that_declares_an_absurd_size_is_refused_before_anything_is_decoded() {
    // 17 GB a frame as RGBA, in a few bytes: the system refuses what has no pixels behind its header, and
    // nothing is allocated for the size it declares.
    for hostile in [
        crafted::gif_declaring(65535, 65535),
        crafted::png_declaring(100_000, 100_000),
        crafted::png_declaring(20_000, 20_000),
        // The canvas a GIF declares is what some decoders allocate, whatever its one frame is.
        crafted::gif_with_a_pixel_on_a_screen(5000, 5000),
    ] {
        assert!(matches!(decode(&hostile), Err(Refused::TooLarge { .. } | Refused::Unreadable)));
    }
}

#[test]
fn a_real_picture_over_the_pixel_cap_is_refused_from_its_header() {
    // 100,010,000 pixels of one color: a PNG of a hundred kilobytes that would take 400 MB as RGBA.
    let mut out = std::io::Cursor::new(Vec::new());
    image::GrayImage::new(10_001, 10_000).write_to(&mut out, image::ImageFormat::Png).unwrap();
    let bytes = out.into_inner();
    assert!(bytes.len() < 2_000_000, "{}", bytes.len());
    assert_eq!(decode(&bytes).map(|_| ()), Err(Refused::TooLarge { width: 10_001, height: 10_000 }));
    // Just under it is decoded, scaled to the side cap.
    let mut out = std::io::Cursor::new(Vec::new());
    image::GrayImage::new(9_999, 10_000).write_to(&mut out, image::ImageFormat::Png).unwrap();
    let decoded = decode(&out.into_inner()).expect("under the cap");
    assert!((MAX_SIDE - 4..=MAX_SIDE).contains(&decoded.width), "scaled to the side cap: {}", decoded.width);
}

#[test]
fn bytes_that_are_no_picture_are_unreadable() {
    assert_eq!(decode(b"").map(|_| ()), Err(Refused::Unreadable));
    assert_eq!(decode(b"GIF89a").map(|_| ()), Err(Refused::Unreadable));
    assert_eq!(decode(&[0u8; 4096]).map(|_| ()), Err(Refused::Unreadable));
}
