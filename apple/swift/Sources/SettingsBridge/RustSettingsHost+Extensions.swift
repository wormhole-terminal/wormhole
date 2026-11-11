// What the Agents, Feature flags and nav pages read from the core beyond values: each agent's Wormhole skill and the line
// that adds `wh mcp` to Claude Code, the flags and which pages a build shows. Skills are written only under
// `WORMHOLE_AGENT_HOME`, as the hooks are.

import SettingsModel
import WormholeBindings

extension RustSettingsHost {
    /// Reads the skills again (after an install or a removal, Rescan PATH): stats the PATH, so off the main thread.
    func refreshSkills() {
        Task.detached {
            let rows = settingsSkillRows().map(RecordMapping.skill)
            let command = settingsMcpCommand()
            await MainActor.run {
                self.records.skills = rows
                self.records.mcpCommand = command
            }
        }
    }

    public func perform(skill id: String, install: Bool) {
        Task.detached {
            let result = Result { try settingsSetSkill(word: id, install: install) }
            await MainActor.run {
                self.problem(result.failure.map(SettingsFailure.sentence))
                self.refreshSkills()
            }
        }
    }

    public func flags() -> [FlagRecord] { handle.featureFlags().map(RecordMapping.flag) }

    public func setFlag(_ key: String, on: Bool) {
        let result = Result { try handle.setFeatureFlag(key: key, on: on) }
        problem(result.failure.map(SettingsFailure.sentence))
        backend.reload()
    }

    /// ⌃⌥⌘F: shows the flags in a release, or hides them again.
    public func toggleFlagsUnlocked() -> Bool {
        let unlocked = (try? handle.toggleFlagsUnlocked()) ?? false
        backend.reload()
        return unlocked
    }

    public func shownSections() -> Set<String>? { Set(handle.visibleSections()) }
}
