//! AppKit's side of `platform::image`: `NSImage` reads what ImageIO can (HEIC included) and hands the
//! pixels back as TIFF.

use objc2::AnyThread as _;
use objc2_app_kit::NSImage;
use objc2_foundation::NSData;

pub fn to_tiff(bytes: &[u8]) -> Option<Vec<u8>> {
    let data = NSData::with_bytes(bytes);
    let image = NSImage::initWithData(NSImage::alloc(), &data)?;
    let tiff = image.TIFFRepresentation()?;
    Some(tiff.to_vec())
}

#[cfg(test)]
mod tests {
    use super::to_tiff;

    /// A 16 x 16 picture as `sips -s format heic` writes it.
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

    #[test]
    fn a_heic_picture_comes_back_as_a_tiff_and_other_bytes_do_not() {
        let bytes: Vec<u8> = HEIC
            .concat()
            .as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let tiff = to_tiff(&bytes).expect("the system reads HEIC");
        assert!(tiff.starts_with(b"II*\0") || tiff.starts_with(b"MM\0*"), "a TIFF header");
        assert!(to_tiff(b"not a picture").is_none());
    }
}
