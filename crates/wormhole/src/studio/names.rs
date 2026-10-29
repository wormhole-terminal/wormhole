//! The words and dates a theme's title row and notices are made from. Pure.

use std::time::SystemTime;

/// The name a theme opened from a built-in has until it is named.
pub(super) fn untitled_name(from: &str) -> String {
    wormhole_text::keys::studio_untitled_from(from)
}

/// A day as `2026-10-29`, UTC, for the day a base was copied.
pub(super) fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    let (y, m, d) = civil(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// The calendar date of a day count since 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// `2026-10-29` as "29 Oct 2026"; anything else as it is.
pub(super) fn display_date(day: &str) -> String {
    const MONTHS: [&str; 12] =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let mut parts = day.split('-');
    let (Some(y), Some(m), Some(d), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return day.to_string();
    };
    match (m.parse::<usize>(), d.parse::<u32>()) {
        (Ok(m @ 1..=12), Ok(d)) => format!("{d} {} {y}", MONTHS[m - 1]),
        _ => day.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_become_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_025), (2024, 10, 29));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }

    #[test]
    fn a_day_reads_as_a_date() {
        assert_eq!(display_date("2026-10-29"), "29 Oct 2026");
        assert_eq!(display_date("2026-01-05"), "5 Jan 2026");
        assert_eq!(display_date("soon"), "soon");
        assert_eq!(display_date("2026-13-01"), "2026-13-01");
        assert_eq!(today().len(), 10);
    }
}
