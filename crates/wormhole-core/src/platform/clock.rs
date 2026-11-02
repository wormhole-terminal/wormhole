//! The wall clock as a person reads it: the time of day in the system's time zone.

use super::unix::clock as os;

/// A time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millis: u16,
}

impl std::fmt::Display for TimeOfDay {
    /// `14:02:07.120`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}:{:02}.{:03}", self.hour, self.minute, self.second, self.millis)
    }
}

/// The time of day, in the system's time zone, at `unix_ms` milliseconds after the epoch; `None` when
/// the system can't say.
pub fn local_time_of_day(unix_ms: u64) -> Option<TimeOfDay> {
    let (hour, minute, second) = os::local_hms(unix_ms / 1000)?;
    Some(TimeOfDay { hour, minute, second, millis: (unix_ms % 1000) as u16 })
}

/// `2026-11-02 14:03`: the date and the time of day, in the system's time zone, at `unix_seconds`; `None` when
/// the system can't say.
pub fn local_date_time(unix_seconds: u64) -> Option<String> {
    let (year, month, day, hour, minute) = os::local_ymd_hm(unix_seconds)?;
    Some(format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}"))
}

#[cfg(test)]
mod tests {
    use super::{TimeOfDay, local_date_time, local_time_of_day};

    #[test]
    fn the_date_and_time_read_as_a_stamp() {
        // 2026-10-26 14:02:07 UTC: a day within a day of that date wherever the zone is.
        let stamp = local_date_time(1_793_023_327).expect("the system knows its zone");
        assert_eq!(stamp.len(), 16, "{stamp}");
        assert!(["2026-10-25", "2026-10-26", "2026-10-27"].iter().any(|d| stamp.starts_with(d)), "{stamp}");
        assert_eq!(&stamp[10..11], " ");
    }

    #[test]
    fn the_time_of_day_keeps_the_seconds_and_milliseconds() {
        // 2026-10-26 14:02:07.120 UTC. Zones differ in whole hours or half-hours, never in seconds.
        let time = local_time_of_day(1_793_023_327_120).expect("the system knows its zone");
        assert_eq!((time.second, time.millis), (7, 120));
        assert!(time.hour < 24 && time.minute < 60);
        assert!(time.minute % 15 == 2, "zones are whole quarter hours away from UTC: {time}");
    }

    #[test]
    fn it_reads_as_a_clock() {
        let time = TimeOfDay { hour: 4, minute: 5, second: 6, millis: 7 };
        assert_eq!(time.to_string(), "04:05:06.007");
    }
}
