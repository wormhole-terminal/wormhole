// The Profile page's fields and the draft the page edits. The fields are core's (`profiles.json` through
// `ProfileList::edit`); a text field applies when it loses focus or takes Return, a swatch and the folder list at
// once (`profile_form.rs`). What core refuses, it says in a sentence the page shows, and the profile stays as it was.

import Observation

public struct ProfileForm: Sendable, Equatable {
    public struct Login: Sendable, Equatable {
        public var agent: String
        public var name: String
        public var placeholder: String
        public var dir: String

        public init(agent: String, name: String, placeholder: String, dir: String) {
            (self.agent, self.name, self.placeholder, self.dir) = (agent, name, placeholder, dir)
        }
    }

    public var id: String
    public var name: String
    public var avatar: String
    /// A group colour's key (`blue`).
    public var color: String
    public var startDir: String
    public var folders: [String]
    /// `NAME=value`, one per line.
    public var env: String
    public var gitName: String
    public var gitEmail: String
    public var logins: [Login]
    /// "Folders under ~/work open in Home too.", in the catalogs' words.
    public var startCaption: String

    public init(
        id: String, name: String, avatar: String = "", color: String = "blue", startDir: String = "", folders: [String] = [],
        env: String = "", gitName: String = "", gitEmail: String = "", logins: [Login] = [], startCaption: String = ""
    ) {
        (self.id, self.name, self.avatar, self.color, self.startDir) = (id, name, avatar, color, startDir)
        (self.folders, self.env, self.gitName, self.gitEmail, self.logins) = (folders, env, gitName, gitEmail, logins)
        self.startCaption = startCaption
    }

    /// The letter in the chip: the typed one, else the name's first.
    public var letter: String { avatar.isEmpty ? String(name.prefix(1)).uppercased() : avatar.uppercased() }
}

/// The page's copy of the profile Wormhole is in: edited by the fields, written back by `commit`.
@MainActor @Observable
public final class ProfileDraft {
    public var form: ProfileForm?
    /// Why the last edit was refused; nil after one that stuck.
    public private(set) var problem: String?
    @ObservationIgnored private let host: any SettingsHost

    public init(host: any SettingsHost) {
        self.host = host
        reload()
    }

    /// Reads the profile again: the window opened, or Wormhole is in another profile.
    public func reload() { form = host.profileForm() }

    /// Writes the fields; core's sentence stays when it refuses, and the fields keep what was typed.
    public func commit() {
        guard let form else { return }
        problem = host.saveProfile(form)
        if problem == nil { reload() }
    }

    /// An edit that is not a text field (a swatch, the folder list) lands at once.
    public func edit(_ change: (inout ProfileForm) -> Void) {
        guard let before = form else { return }
        change(&form!)
        commit()
        if problem != nil { form = before }
    }
}
