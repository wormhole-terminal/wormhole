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

#[cfg(test)]
mod tests {
    use super::{TimeOfDay, local_time_of_day};

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
