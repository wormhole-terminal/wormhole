//! Profiles: a whole Wormhole of its own (RF-21). One process, one profile active at a time;
//! switching changes every window, and a window can be opened in another profile.
//!
//! What a profile has: its terminals, groups and pinned groups (the database), theme, fonts, keys,
//! shells, notifications, menu bar item and hotkey (`config.json`), trusted projects, and, in the
//! profile list, its start folder, the folders that open in it, an environment, a git identity and
//! a login per coding agent. What is shared: the update schedule, the single-instance lock, the hook
//! socket, the `wh` command's folder, the logs and `profiles.json` itself, all at the top of the
//! data folder.
//!
//! - `model`: [`Profile`], its identity, environment and the variables its terminals get;
//! - `list`: [`ProfileList`], `profiles.json` and what creating, renaming and deleting do;
//! - `paths`: [`DataRoot`] and [`ProfilePaths`], where each profile's files live;
//! - `copy`: what a new profile takes from the one it starts as a copy of.
//!
//! Layout: the Default profile keeps its files where the data folder has always had them, so an
//! existing user's data never moves; every other profile has `profiles/<id>/` with the same file
//! names. Nothing here writes to a repository, and the git identity only reaches a terminal as
//! environment variables.

mod copy;
mod list;
mod model;
mod paths;
#[cfg(test)]
mod tests;

pub use list::{NewProfile, ProfileList, StartFrom};
pub use model::{
    AgentLogin, DEFAULT_ID, GitIdentity, PROFILE_VAR, Profile, ProfileError, ProfileId, first_letter,
    login_env_var,
};
pub(crate) use paths::data_dir_name;
pub use paths::{DataRoot, ProfilePaths, data_dir};
