//! The words of the Updates page that more than one place needs: how long ago a check was, what is available, and the
//! headers a counted check sends.

use wormhole_core::update::Release;
use wormhole_text::keys::{
    SETTINGS_UPDATES_CHECKED_A_MINUTE_AGO, SETTINGS_UPDATES_CHECKED_AN_HOUR_AGO,
    SETTINGS_UPDATES_CHECKED_JUST_NOW, settings_updates_checked_days_ago, settings_updates_checked_hours_ago,
    settings_updates_checked_minutes_ago, settings_updates_security_available,
    settings_updates_version_available,
};
use wormhole_text::{text, whole};

/// The header of the first check of a day and of a month, as `update::Count` sends it.
pub(in crate::settings_window) const COUNT_HEADERS: [&str; 2] =
    ["Wormhole-Count: day", "Wormhole-Count: month"]; // text: protocol, sent as is

/// "Checked 4 minutes ago", from the last check's time.
pub(in crate::settings_window) fn checked_ago(last: u64, now: u64) -> String {
    let minutes = now.saturating_sub(last) / 60;
    match minutes {
        0 => text(SETTINGS_UPDATES_CHECKED_JUST_NOW).into(),
        1 => text(SETTINGS_UPDATES_CHECKED_A_MINUTE_AGO).into(),
        2..=59 => settings_updates_checked_minutes_ago(whole(minutes)),
        60..=119 => text(SETTINGS_UPDATES_CHECKED_AN_HOUR_AGO).into(),
        _ if minutes < 48 * 60 => settings_updates_checked_hours_ago(whole(minutes / 60)),
        _ => settings_updates_checked_days_ago(whole(minutes / (24 * 60))),
    }
}

/// "Security update available: Wormhole 0.3.1." for a security update, else "Version 0.3.1 is available."
pub(in crate::settings_window) fn available_text(release: &Release) -> String {
    if release.is_security_update() {
        settings_updates_security_available(&release.version.to_string())
    } else {
        settings_updates_version_available(&release.version.to_string())
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::update::{Asset, Release, Security, Severity};

    use super::{available_text, checked_ago};

    #[test]
    fn the_last_check_reads_as_an_age() {
        assert_eq!(checked_ago(1000, 1010), "Checked just now.");
        assert_eq!(checked_ago(0, 4 * 60), "Checked 4 minutes ago.");
        assert_eq!(checked_ago(0, 3 * 3600), "Checked 3 hours ago.");
        assert_eq!(checked_ago(0, 3 * 86400), "Checked 3 days ago.");
        assert_eq!(checked_ago(100, 0), "Checked just now.", "a clock that went back");
    }

    fn release(security: Option<Security>) -> Release {
        Release {
            version: semver::Version::new(0, 3, 1),
            notes: String::new(),
            pub_date: None,
            minimum_system_version: None,
            asset: Asset { url: String::new(), sha256: String::new(), signature: String::new(), size: 0 },
            security,
        }
    }

    #[test]
    fn a_security_update_says_so() {
        assert_eq!(available_text(&release(None)), "Version 0.3.1 is available.");
        let high = Security { severity: Severity::High, advisories: Vec::new() };
        assert_eq!(available_text(&release(Some(high))), "Security update available: Wormhole 0.3.1.");
    }
}
