//! The profile list and the switch: `profiles.json` through core's `ProfileList`. A profile is a whole
//! set of settings (its own `config.json`, themes and fonts), so a `SettingsHandle` is opened per
//! profile by id; this lists them, says which one Wormhole is in and makes, switches and deletes them.
//! Copy rules for a new profile (`StartFrom::Copy`) and every check on a name or a folder are core's.

use wormhole_core::config::profile::{NewProfile, Profile, ProfileList, StartFrom};
use wormhole_core::storage::paths::{DataRoot, ProfileId};

use super::error::SettingsError;
use crate::isolation;

/// One profile, with the fields the Profile page shows.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsProfileRow {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    /// Wormhole is in this one now.
    pub is_active: bool,
    /// Where new terminals start (`~/…` or absolute); empty for the home folder.
    pub start_dir: String,
    pub git_name: String,
    pub git_email: String,
}

fn failed(err: impl std::fmt::Display) -> SettingsError {
    SettingsError::Profile { reason: err.to_string() }
}

fn id_of(text: &str) -> Result<ProfileId, SettingsError> {
    ProfileId::parse(text).map_err(failed)
}

fn row(profile: &Profile, active: &ProfileId) -> SettingsProfileRow {
    SettingsProfileRow {
        id: profile.id.to_string(),
        name: profile.name.clone(),
        is_default: profile.id.is_default(),
        is_active: &profile.id == active,
        start_dir: profile.start_dir.clone(),
        git_name: profile.git.name.clone(),
        git_email: profile.git.email.clone(),
    }
}

pub(crate) fn list_in(root: &DataRoot) -> Vec<SettingsProfileRow> {
    // A profiles.json that does not read leaves Default alone, as the app starts.
    let list = crate::profile::list_in(root);
    list.profiles().iter().map(|profile| row(profile, list.active_id())).collect()
}

pub(crate) fn switch_in(root: &DataRoot, id: &str) -> Result<(), SettingsError> {
    let mut list = ProfileList::load(root).map_err(failed)?;
    list.set_active(&id_of(id)?).map_err(failed)?;
    list.save(root).map_err(failed)
}

/// Makes a profile named `name` with the next free colour, as a copy of `copy_of`'s settings, themes
/// and fonts (never its terminals or logins) or with the defaults. Returns its id.
pub(crate) fn create_in(root: &DataRoot, name: &str, copy_of: Option<&str>) -> Result<String, SettingsError> {
    let mut list = ProfileList::load(root).map_err(failed)?;
    let from = match copy_of {
        Some(id) => StartFrom::Copy(id_of(id)?),
        None => StartFrom::Defaults,
    };
    let id = list
        .create(root, NewProfile { name: name.to_string(), color: None, avatar: None, from })
        .map_err(failed)?;
    list.save(root).map_err(failed)?;
    Ok(id.to_string())
}

pub(crate) fn delete_in(root: &DataRoot, id: &str) -> Result<(), SettingsError> {
    let mut list = ProfileList::load(root).map_err(failed)?;
    list.delete(root, &id_of(id)?).map_err(failed)?;
    list.save(root).map_err(failed)
}

/// Every profile, Default first.
#[uniffi::export]
pub fn settings_profiles() -> Result<Vec<SettingsProfileRow>, SettingsError> {
    isolation::require()?;
    Ok(list_in(&DataRoot::current()))
}

/// Makes `id` the profile Wormhole is in (the choice is kept in `profiles.json`). The window follows it
/// by opening a `SettingsHandle` for that id.
#[uniffi::export]
pub fn settings_switch_profile(id: String) -> Result<(), SettingsError> {
    isolation::require()?;
    switch_in(&DataRoot::current(), &id)
}

/// Makes a profile; `copy_of` is the id of the profile whose settings it starts as. Returns its id.
#[uniffi::export]
pub fn settings_create_profile(name: String, copy_of: Option<String>) -> Result<String, SettingsError> {
    isolation::require()?;
    create_in(&DataRoot::current(), &name, copy_of.as_deref())
}

/// Deletes a profile and its folder. Default cannot be deleted.
#[uniffi::export]
pub fn settings_delete_profile(id: String) -> Result<(), SettingsError> {
    isolation::require()?;
    delete_in(&DataRoot::current(), &id)
}

#[cfg(test)]
mod tests {
    use wormhole_core::storage::paths::DataRoot;

    use super::{create_in, delete_in, list_in, switch_in};

    #[test]
    fn a_profile_is_made_switched_to_and_deleted() {
        let dir = tempfile::tempdir().expect("temp data dir");
        let root = DataRoot::new(dir.path());
        let rows = list_in(&root);
        assert_eq!((rows.len(), rows[0].is_default, rows[0].is_active), (1, true, true));

        let id = create_in(&root, "Work", None).expect("made");
        let rows = list_in(&root);
        assert_eq!(rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Default", "Work"]);

        switch_in(&root, &id).expect("switched");
        assert!(list_in(&root).iter().any(|r| r.id == id && r.is_active));

        assert!(delete_in(&root, "default").is_err(), "Default cannot go");
        switch_in(&root, "default").expect("back");
        delete_in(&root, &id).expect("deleted");
        assert_eq!(list_in(&root).len(), 1);
        assert!(switch_in(&root, "nope").is_err());
    }
}
