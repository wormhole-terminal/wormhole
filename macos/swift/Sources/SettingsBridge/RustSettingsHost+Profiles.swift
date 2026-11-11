// Making, switching and deleting profiles: core's `ProfileList` behind `profiles.json`. A profile is a whole set of
// settings, so after a switch the window opens a handle for the new id (`SettingsBridge.make(profile:)`).

import WormholeBindings

extension RustSettingsHost {
    /// Every profile, Default first, with the active one marked.
    public func profiles() throws -> [SettingsProfileRow] { try settingsProfiles() }

    /// Makes a profile, as a copy of `copyOf`'s settings or with the defaults; returns its id.
    public func createProfile(name: String, copyOf: String? = nil) throws -> String {
        try settingsCreateProfile(name: name, copyOf: copyOf)
    }

    public func switchProfile(to id: String) throws {
        try settingsSwitchProfile(id: id)
        refreshProfile()
    }

    public func deleteProfile(_ id: String) throws {
        try settingsDeleteProfile(id: id)
        refreshProfile()
    }
}
