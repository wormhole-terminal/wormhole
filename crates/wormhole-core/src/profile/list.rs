//! The profile list: `profiles.json` at the top of the data folder.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::copy::copy_settings;
use super::model::{Profile, ProfileError, ProfileId, first_letter};
use super::paths::DataRoot;
use crate::folder_color::FolderColor;
use crate::fsutil::{self, LinkPolicy};

/// Largest `profiles.json` read; a few dozen profiles are a few KiB.
const MAX_FILE: u64 = 512 * 1024;
const MAX_PROFILES: usize = 64;
const VERSION: u32 = 1;

/// What a new profile starts as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartFrom {
    /// A copy of this profile's settings, themes and fonts, start folder, environment and git
    /// identity. Never its terminals, trusted projects, folders or agent logins: the new profile
    /// signs in on its own.
    Copy(ProfileId),
    /// Wormhole's defaults.
    Defaults,
}

/// The "New profile" sheet's answers.
#[derive(Clone, Debug)]
pub struct NewProfile {
    pub name: String,
    /// The next colour no profile wears when `None`.
    pub color: Option<FolderColor>,
    /// The name's first letter when `None`.
    pub avatar: Option<char>,
    pub from: StartFrom,
}

/// The profiles and which one Wormhole is in. Default is always the first and can't go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileList {
    active: ProfileId,
    profiles: Vec<Profile>,
}

#[derive(Serialize, Deserialize)]
struct FileForm {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    active: String,
    #[serde(default)]
    profiles: Vec<Value>,
}

impl Default for ProfileList {
    fn default() -> Self {
        Self { active: ProfileId::default_profile(), profiles: vec![Profile::default_profile()] }
    }
}

impl ProfileList {
    /// Read `profiles.json`; without one there is only Default (and nothing is written until
    /// something changes). A profile entry that doesn't read is left out and the rest are kept.
    pub fn load(root: &DataRoot) -> Result<Self, ProfileError> {
        let path = root.profiles_file();
        let text = match fsutil::read_to_string_capped(&path, MAX_FILE, LinkPolicy::Follow) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(ProfileError::io(&path, err)),
        };
        let form: FileForm = crate::jsonc::parse(&text)
            .map_err(|message| ProfileError::Parse { path: path.clone(), message })?;
        let mut list = Self { active: ProfileId::default_profile(), profiles: Vec::new() };
        for value in form.profiles {
            match serde_json::from_value::<Profile>(value) {
                Ok(mut profile) if list.get(&profile.id).is_none() && list.profiles.len() < MAX_PROFILES => {
                    profile.sanitize();
                    list.profiles.push(profile);
                }
                Ok(profile) => tracing::warn!("profiles.json: {} is listed twice or too many", profile.id),
                Err(err) => tracing::warn!("profiles.json: a profile doesn't read, left out: {err}"),
            }
        }
        match list.profiles.iter().position(|p| p.id.is_default()) {
            Some(0) => {}
            Some(at) => {
                let default = list.profiles.remove(at);
                list.profiles.insert(0, default);
            }
            None => list.profiles.insert(0, Profile::default_profile()),
        }
        if let Some(active) = ProfileId::parse(&form.active).ok().filter(|id| list.get(id).is_some()) {
            list.active = active;
        }
        Ok(list)
    }

    /// [`load`](Self::load); a file that doesn't parse is kept aside (`profiles.json.bad`) and the
    /// list is rebuilt from the folders under `profiles/`, named after them, so no profile's data
    /// is orphaned. The error is for the person.
    pub fn load_or_recover(root: &DataRoot) -> (Self, Option<ProfileError>) {
        match Self::load(root) {
            Ok(list) => (list, None),
            Err(err) => {
                let path = root.profiles_file();
                let mut aside = path.as_os_str().to_owned();
                aside.push(".bad");
                if let Err(rename) = std::fs::rename(&path, &aside) {
                    tracing::warn!("setting aside {}: {rename}", path.display());
                }
                (Self::rebuilt(root), Some(err))
            }
        }
    }

    fn rebuilt(root: &DataRoot) -> Self {
        let mut list = Self::default();
        let mut ids: Vec<ProfileId> = std::fs::read_dir(root.profiles_dir())
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| ProfileId::parse(e.file_name().to_str()?).ok())
            .filter(|id| !id.is_default())
            .collect();
        ids.sort();
        for id in ids.into_iter().take(MAX_PROFILES - 1) {
            let name: String = id.as_str().replace('-', " ");
            let color = list.next_color();
            list.profiles.push(Profile::new(id, &name, color));
        }
        list
    }

    pub fn save(&self, root: &DataRoot) -> Result<(), ProfileError> {
        let form = FileForm {
            version: VERSION,
            active: self.active.to_string(),
            profiles: self.profiles.iter().filter_map(|p| serde_json::to_value(p).ok()).collect(),
        };
        let path = root.profiles_file();
        let json = serde_json::to_string_pretty(&form).map_err(|e| ProfileError::invalid(e.to_string()))?;
        fsutil::write_atomic(&path, json.as_bytes(), LinkPolicy::Follow)
            .map_err(|e| ProfileError::io(&path, e))
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn get(&self, id: &ProfileId) -> Option<&Profile> {
        self.profiles.iter().find(|p| &p.id == id)
    }

    pub fn default_profile(&self) -> &Profile {
        &self.profiles[0]
    }

    pub fn active_id(&self) -> &ProfileId {
        &self.active
    }

    pub fn active(&self) -> &Profile {
        self.get(&self.active).unwrap_or_else(|| self.default_profile())
    }

    /// Wormhole is in this profile from now on (the caller saves, so the next launch is too).
    pub fn set_active(&mut self, id: &ProfileId) -> Result<(), ProfileError> {
        if self.get(id).is_none() {
            return Err(ProfileError::Unknown(id.to_string()));
        }
        self.active = id.clone();
        Ok(())
    }

    /// Change a profile. The closure gets a copy; what it leaves is checked (the profile's rules,
    /// and a name no other profile has) before it replaces the profile, and its id is not
    /// the closure's to change. The caller saves.
    pub fn edit(&mut self, id: &ProfileId, change: impl FnOnce(&mut Profile)) -> Result<(), ProfileError> {
        let at = self
            .profiles
            .iter()
            .position(|p| &p.id == id)
            .ok_or_else(|| ProfileError::Unknown(id.to_string()))?;
        let mut edited = self.profiles[at].clone();
        change(&mut edited);
        edited.id = id.clone();
        edited.name = edited.name.trim().to_string();
        edited.check()?;
        if self.name_taken(&edited.name, Some(id)) {
            return Err(ProfileError::invalid(format!("There is already a profile called {}", edited.name)));
        }
        self.profiles[at] = edited;
        Ok(())
    }

    pub fn rename(&mut self, id: &ProfileId, name: &str) -> Result<(), ProfileError> {
        let name = name.to_string();
        self.edit(id, |p| p.name = name)
    }

    /// Make a profile: its folder, its settings from `new.from`, and its place in the list, saved.
    /// On a failure nothing is left behind.
    pub fn create(&mut self, root: &DataRoot, new: NewProfile) -> Result<ProfileId, ProfileError> {
        if self.profiles.len() >= MAX_PROFILES {
            return Err(ProfileError::invalid(format!("At most {MAX_PROFILES} profiles")));
        }
        let name = new.name.trim().to_string();
        let source = match &new.from {
            StartFrom::Copy(from) => {
                Some(self.get(from).ok_or_else(|| ProfileError::Unknown(from.to_string()))?.clone())
            }
            StartFrom::Defaults => None,
        };
        let id = self.fresh_id(root, &name);
        let color = new.color.unwrap_or_else(|| self.next_color());
        let mut profile = Profile::new(id.clone(), &name, color);
        profile.avatar = new.avatar.unwrap_or_else(|| first_letter(&name));
        if let Some(source) = &source {
            // Agent logins and folders stay behind on purpose; see `StartFrom::Copy`.
            profile.start_dir.clone_from(&source.start_dir);
            profile.env.clone_from(&source.env);
            profile.git.clone_from(&source.git);
        }
        profile.check()?;
        if self.name_taken(&profile.name, None) {
            return Err(ProfileError::invalid(format!("There is already a profile called {}", profile.name)));
        }
        let paths = root.profile(&id);
        let made = match &source {
            Some(source) => copy_settings(&root.profile(&source.id), &paths),
            None => std::fs::create_dir_all(paths.dir()),
        };
        if let Err(err) = made {
            remove_profile_dir(root, &id);
            return Err(ProfileError::io(paths.dir(), err));
        }
        self.profiles.push(profile);
        if let Err(err) = self.save(root) {
            self.profiles.pop();
            remove_profile_dir(root, &id);
            return Err(err);
        }
        Ok(id)
    }

    /// Remove a profile and its files. Never Default. The list is saved before the files go, so a
    /// failure in between leaves a folder nobody lists, not a profile with no files. If Wormhole was
    /// in it, it is in Default now. The caller has closed the profile's windows first.
    pub fn delete(&mut self, root: &DataRoot, id: &ProfileId) -> Result<Profile, ProfileError> {
        if id.is_default() {
            return Err(ProfileError::IsDefault);
        }
        let at = self
            .profiles
            .iter()
            .position(|p| &p.id == id)
            .ok_or_else(|| ProfileError::Unknown(id.to_string()))?;
        let removed = self.profiles.remove(at);
        let was_active = self.active.clone();
        if &self.active == id {
            self.active = ProfileId::default_profile();
        }
        if let Err(err) = self.save(root) {
            self.profiles.insert(at, removed);
            self.active = was_active;
            return Err(err);
        }
        remove_profile_dir(root, id);
        Ok(removed)
    }

    /// The profile a folder opens in, if one claims it ("folders under ~/work and ~/clients open
    /// in Work too"): the one with the deepest matching folder. `None` leaves it to the profile
    /// the window is in.
    pub fn profile_for_folder(&self, path: &Path) -> Option<&Profile> {
        self.profile_for_folder_with_home(path, dirs::home_dir().as_deref())
    }

    pub(super) fn profile_for_folder_with_home(&self, path: &Path, home: Option<&Path>) -> Option<&Profile> {
        self.profiles
            .iter()
            .filter_map(|p| Some((p, p.claim_depth(path, home)?)))
            .max_by_key(|(_, depth)| *depth)
            .map(|(p, _)| p)
    }

    fn name_taken(&self, name: &str, except: Option<&ProfileId>) -> bool {
        self.profiles
            .iter()
            .any(|p| Some(&p.id) != except && p.name.trim().to_lowercase() == name.to_lowercase())
    }

    /// The first colour no profile wears; once all eight are worn, the least worn.
    pub fn next_color(&self) -> FolderColor {
        let worn = |c: FolderColor| self.profiles.iter().filter(|p| p.color == c).count();
        FolderColor::ALL.into_iter().min_by_key(|c| worn(*c)).unwrap_or(FolderColor::Blue)
    }

    /// An id from the name that no profile and no folder uses (a folder a failed delete left
    /// behind included), and that isn't Default's.
    fn fresh_id(&self, root: &DataRoot, name: &str) -> ProfileId {
        let mut slug = String::new();
        for c in name.chars().flat_map(char::to_lowercase) {
            if c.is_ascii_alphanumeric() {
                slug.push(c);
            } else if !slug.is_empty() && !slug.ends_with('-') {
                slug.push('-');
            }
        }
        slug.truncate(28);
        let slug = slug.trim_matches('-');
        let base = if slug.is_empty() { "profile" } else { slug };
        let free = |text: &str| {
            ProfileId::parse(text).ok().filter(|id| {
                !id.is_default()
                    && self.get(id).is_none()
                    && std::fs::symlink_metadata(root.profile(id).dir()).is_err()
            })
        };
        free(base).or_else(|| (2..1000).find_map(|n| free(&format!("{base}-{n}")))).unwrap_or_else(|| {
            // A thousand profiles of one name: the clock is as unique as anything.
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            ProfileId(format!("p{}", now.as_secs()))
        })
    }
}

/// Delete `profiles/<id>/`, and only that: never the data folder (Default's files), and a link in
/// its place is removed, not followed.
fn remove_profile_dir(root: &DataRoot, id: &ProfileId) {
    if id.is_default() {
        return;
    }
    let dir = root.profile(id).dir().to_path_buf();
    let result = match std::fs::symlink_metadata(&dir) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(&dir),
        Ok(_) => std::fs::remove_file(&dir),
        Err(_) => return,
    };
    if let Err(err) = result {
        tracing::warn!("removing {}: {err}", dir.display());
    }
}
