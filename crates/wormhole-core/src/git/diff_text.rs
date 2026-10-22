//! What the diff card says about a file it shows no lines of: a patch too large to load
//! unasked, a binary file's sizes, what a Git LFS pointer stands for. Sizes are written in the
//! units the `size_units` setting picks: Finder's by default, git's as an option.

use super::{DIFF_MEASURE_BYTES, LfsChange, LfsPointer};
use crate::config::SizeUnits;
use crate::view::limits::count_grouped;

/// A size in `units`, at most one decimal and none from 100 up, whole bytes below a unit:
/// decimal the way Finder writes it ("48.2 KB", "12.4 MB", "100 MB"), binary the way git and
/// `du -h` do ("47.1 KiB", "11.8 MiB").
pub fn byte_size(bytes: u64, units: SizeUnits) -> String {
    const DECIMAL: [&str; 4] = ["KB", "MB", "GB", "TB"];
    const BINARY: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    let (names, step) = match units {
        SizeUnits::Decimal => (DECIMAL, 1000.0),
        SizeUnits::Binary => (BINARY, 1024.0),
    };
    match bytes {
        1 => return "1 byte".into(),
        _ if (bytes as f64) < step => return format!("{bytes} bytes"),
        _ => {}
    }
    let mut value = bytes as f64 / step;
    let mut unit = 0;
    // Rounded to the whole number it gets from 100 up, 999.5 KB would read "1000 KB": that is
    // 1 MB (and 1023.5 KiB 1 MiB).
    while value >= step - 0.5 && unit + 1 < names.len() {
        value /= step;
        unit += 1;
    }
    let number = if value >= 99.95 { format!("{value:.0}") } else { format!("{value:.1}") };
    format!("{} {}", number.strip_suffix(".0").unwrap_or(&number), names[unit])
}

/// The first line of the card in place of a diff too large to load unasked.
pub fn too_large_title(patch_bytes: Option<u64>, units: SizeUnits) -> String {
    match patch_bytes {
        Some(bytes) => format!("This diff is {}", byte_size(bytes, units)),
        None => format!("This diff is larger than {}", byte_size(DIFF_MEASURE_BYTES, units)),
    }
}

/// Its second line: how many lines changed, git's own count (known before the patch is read).
pub fn too_large_detail(changed_lines: Option<usize>) -> String {
    const SLOW: &str = "Loading it can take a few seconds.";
    match changed_lines {
        Some(1) => format!("1 changed line. {SLOW}"),
        Some(lines) => format!("{} changed lines. {SLOW}", count_grouped(lines)),
        None => SLOW.into(),
    }
}

/// A file's size before and after ("48.2 KB → 52.9 KB"), or the one side's size it has.
pub fn sizes_label(old: Option<u64>, new: Option<u64>, units: SizeUnits) -> Option<String> {
    match (old, new) {
        (Some(old), Some(new)) => Some(format!("{} → {}", byte_size(old, units), byte_size(new, units))),
        (Some(size), None) | (None, Some(size)) => Some(byte_size(size, units)),
        (None, None) => None,
    }
}

/// A file stored with Git LFS: its sizes, and the object it is now (the last one it was, when
/// it was deleted): "12.3 MB → 12.8 MB · oid 4d7a21…".
pub fn lfs_summary(change: &LfsChange, units: SizeUnits) -> Option<String> {
    let size = |pointer: &Option<LfsPointer>| pointer.as_ref().map(|p| p.size);
    let sizes = sizes_label(size(&change.old), size(&change.new), units)?;
    let pointer = change.new.as_ref().or(change.old.as_ref())?;
    Some(format!("{sizes} · oid {}", short_oid(&pointer.oid)))
}

/// One side of it: "12.8 MB · oid 4d7a21…".
pub fn lfs_side(pointer: &LfsPointer, units: SizeUnits) -> String {
    format!("{} · oid {}", byte_size(pointer.size, units), short_oid(&pointer.oid))
}

/// An object id cut to its first six digits, like a short commit hash.
fn short_oid(oid: &str) -> String {
    let short: String = oid.chars().take(6).collect();
    format!("{short}…")
}

#[cfg(test)]
mod tests {
    use super::{byte_size, lfs_side, lfs_summary, sizes_label, too_large_detail, too_large_title};
    use crate::config::SizeUnits::{Binary, Decimal};
    use crate::git::{LfsChange, LfsPointer};

    #[test]
    fn sizes_read_like_finder() {
        let cases = [
            (0, "0 bytes"),
            (1, "1 byte"),
            (999, "999 bytes"),
            (1000, "1 KB"),
            (48_200, "48.2 KB"),
            (52_949, "52.9 KB"),
            (99_960, "100 KB"),
            (512_000, "512 KB"),
            (999_499, "999 KB"),
            (999_500, "1 MB"),
            (999_960, "1 MB"),
            (12_400_000, "12.4 MB"),
            (8 << 20, "8.4 MB"),
            (100_000_000, "100 MB"),
            (1_234_000_000, "1.2 GB"),
            (u64::MAX, "18446744 TB"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(byte_size(bytes, Decimal), expected, "{bytes}");
        }
    }

    #[test]
    fn binary_sizes_read_like_git() {
        // 13.9 MB is 13.3 MiB, 48.2 KB 47.1 KiB.
        let cases = [
            (0, "0 bytes"),
            (1, "1 byte"),
            (1023, "1023 bytes"),
            (1024, "1 KiB"),
            (48_200, "47.1 KiB"),
            (52_900, "51.7 KiB"),
            (13_900_000, "13.3 MiB"),
            (8 << 20, "8 MiB"),
            (1_048_063, "1023 KiB"),
            (1_048_064, "1 MiB"),
            (100 << 20, "100 MiB"),
            (1_234_000_000, "1.1 GiB"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(byte_size(bytes, Binary), expected, "{bytes}");
        }
    }

    #[test]
    fn the_too_large_card_says_how_big_and_how_many_lines() {
        assert_eq!(too_large_title(Some(12_400_000), Decimal), "This diff is 12.4 MB");
        assert_eq!(too_large_title(Some(12_400_000), Binary), "This diff is 11.8 MiB");
        assert_eq!(too_large_title(None, Decimal), "This diff is larger than 100 MB");
        assert_eq!(
            too_large_detail(Some(36_154)),
            "36,154 changed lines. Loading it can take a few seconds."
        );
        assert_eq!(too_large_detail(Some(1)), "1 changed line. Loading it can take a few seconds.");
        assert_eq!(too_large_detail(None), "Loading it can take a few seconds.");
    }

    #[test]
    fn sizes_before_and_after() {
        assert_eq!(sizes_label(Some(48_200), Some(52_900), Decimal).as_deref(), Some("48.2 KB → 52.9 KB"));
        assert_eq!(sizes_label(Some(48_200), Some(52_900), Binary).as_deref(), Some("47.1 KiB → 51.7 KiB"));
        assert_eq!(sizes_label(None, Some(52_900), Decimal).as_deref(), Some("52.9 KB"));
        assert_eq!(sizes_label(Some(48_200), None, Decimal).as_deref(), Some("48.2 KB"));
        assert_eq!(sizes_label(None, None, Decimal), None);
    }

    #[test]
    fn an_lfs_file_reads_as_its_sizes_and_object() {
        let pointer = |oid: &str, size| Some(LfsPointer { oid: oid.repeat(32), size });
        let both = LfsChange { old: pointer("1a", 12_300_000), new: pointer("4d", 12_800_000) };
        assert_eq!(lfs_summary(&both, Decimal).as_deref(), Some("12.3 MB → 12.8 MB · oid 4d4d4d…"));
        assert_eq!(lfs_summary(&both, Binary).as_deref(), Some("11.7 MiB → 12.2 MiB · oid 4d4d4d…"));
        let deleted = LfsChange { old: pointer("1a", 12_300_000), new: None };
        assert_eq!(lfs_summary(&deleted, Decimal).as_deref(), Some("12.3 MB · oid 1a1a1a…"));
        assert_eq!(lfs_summary(&LfsChange::default(), Decimal), None);
        let side = LfsPointer { oid: "0f".repeat(32), size: 2048 };
        assert_eq!(lfs_side(&side, Decimal), "2 KB · oid 0f0f0f…");
        assert_eq!(lfs_side(&side, Binary), "2 KiB · oid 0f0f0f…");
    }
}
