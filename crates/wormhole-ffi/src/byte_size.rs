//! A size as the panel writes it (`wormhole_core::git::byte_size`): decimal the way Finder does, binary the way git does,
//! by the `size_units` setting. The words are core's, so the viewer says what the diff card would.

use wormhole_core::config::SizeUnits;

/// "48.2 KB" (`binary` false) or "47.1 KiB" (`binary` true).
#[uniffi::export]
pub fn byte_size(bytes: u64, binary: bool) -> String {
    let units = if binary { SizeUnits::Binary } else { SizeUnits::Decimal };
    wormhole_core::git::byte_size(bytes, units)
}

#[cfg(test)]
mod tests {
    use super::byte_size;

    #[test]
    fn the_units_are_the_settings() {
        assert_eq!(byte_size(48_200, false), "48.2 KB");
        assert_eq!(byte_size(48_200, true), "47.1 KiB");
        assert_eq!(byte_size(1, false), "1 byte");
    }
}
