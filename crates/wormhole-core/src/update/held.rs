//! What a change of channel does to an update that is on its way or already on disk.
//!
//! An update has no resting place between "downloaded" and "installed": the app downloads an archive
//! into a temporary file and, in the same run, swaps the new bundle in for the old one
//! ([`install_unless`](super::install_unless)), which is gone afterwards. So there are two things a
//! switch back to Stable can meet, and they differ in what can honestly be done:
//!
//! - **on its way**: not on disk yet, so it can still be called off, and nothing is left behind;
//! - **installed**: the bundle on disk already is that build (the running app is the old one until
//!   it restarts). Nothing keeps the previous bundle, so there is nothing to put back: the build
//!   stays, and Stable takes over with the next Stable release that is newer than it.

use super::UpdateChannel;

/// The update the app holds when the channel changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held<'a> {
    Nothing,
    /// Being downloaded, or checked and swapped in; not yet on disk.
    OnItsWay(&'a semver::Version),
    /// Already in place of the bundle on disk, waiting for a restart.
    Installed(&'a semver::Version),
}

/// What to do about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnSwitch {
    /// The new channel offers this build too (or there is none): leave it.
    Keep,
    /// Call it off: the new channel would not have offered it, and it is not on disk yet.
    CallOff,
    /// It is on disk and the new channel would not have offered it: it stays, and the person is
    /// told so, instead of being offered a restart into a build their channel does not take.
    StaysInstalled,
}

/// Whether `held` belongs to `to`. Only a prerelease can be a build that a channel does not take:
/// Stable never moves a Mac onto one (`UpdateChannel::takes_prereleases`).
pub fn on_channel_switch(held: Held<'_>, to: UpdateChannel) -> OnSwitch {
    let foreign = |version: &semver::Version| !version.pre.is_empty() && !to.takes_prereleases();
    match held {
        Held::OnItsWay(version) if foreign(version) => OnSwitch::CallOff,
        Held::Installed(version) if foreign(version) => OnSwitch::StaysInstalled,
        _ => OnSwitch::Keep,
    }
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::{Held, OnSwitch, on_channel_switch};
    use crate::update::UpdateChannel::{Stable, Unstable};

    fn v(text: &str) -> Version {
        Version::parse(text).expect("a version")
    }

    #[test]
    fn nothing_held_means_nothing_to_do() {
        assert_eq!(on_channel_switch(Held::Nothing, Stable), OnSwitch::Keep);
        assert_eq!(on_channel_switch(Held::Nothing, Unstable), OnSwitch::Keep);
    }

    #[test]
    fn an_unstable_build_on_its_way_is_called_off_by_stable_only() {
        let build = v("1.7.0-dev.12");
        assert_eq!(on_channel_switch(Held::OnItsWay(&build), Stable), OnSwitch::CallOff);
        assert_eq!(on_channel_switch(Held::OnItsWay(&build), Unstable), OnSwitch::Keep);
    }

    #[test]
    fn an_unstable_build_on_disk_stays_and_is_not_rolled_back() {
        let build = v("1.7.0-dev.12");
        assert_eq!(on_channel_switch(Held::Installed(&build), Stable), OnSwitch::StaysInstalled);
        assert_eq!(on_channel_switch(Held::Installed(&build), Unstable), OnSwitch::Keep);
    }

    #[test]
    fn a_stable_release_belongs_to_both_channels() {
        let release = v("1.7.0");
        for held in [Held::OnItsWay(&release), Held::Installed(&release)] {
            assert_eq!(on_channel_switch(held, Stable), OnSwitch::Keep);
            assert_eq!(on_channel_switch(held, Unstable), OnSwitch::Keep);
        }
    }
}
