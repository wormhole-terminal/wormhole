// The Profile page's fields, the Import page's review and the update header, over core: each is one FFI call whose
// rules and words are core's (`ProfileList::edit`, `import::scan_one` and `apply`, `counted_user_agent`).

import CoreText
import Foundation
import SettingsModel
import WormholeBindings

/// A PostScript font name (iTerm2, Terminal.app write those) as its family, which only the system can say.
public final class CoreTextFamilies: FontFamilyResolver {
    public init() {}

    public func family(postscriptName: String) -> String? {
        let font = CTFontCreateWithName(postscriptName as CFString, 12, nil)
        // A name that is not installed comes back as the system font: only the same PostScript name is the face.
        guard CTFontCopyPostScriptName(font) as String == postscriptName else { return nil }
        return CTFontCopyFamilyName(font) as String
    }
}

extension RustSettingsHost {
    public func profileForm() -> ProfileForm? {
        (try? settingsProfileForm(id: nil)).map(RecordMapping.form)
    }

    public func saveProfile(_ form: ProfileForm) -> String? {
        do {
            if try settingsProfileSave(form: RecordMapping.record(form)) { refreshProfile() }
            return nil
        } catch {
            return SettingsFailure.sentence(error)
        }
    }

    public func importReview(_ id: String) async -> ImportReview? {
        let home = importHome
        return await Task.detached {
            settingsImportReview(home: home, source: id, fonts: CoreTextFamilies()).map(RecordMapping.review)
        }.value
    }

    public func applyImport(_ id: String, ticked: [Int], shown: Int) -> (text: String, failed: Bool) {
        do {
            let done = try handle.importApply(
                home: importHome, source: id, ticked: ticked.map(UInt32.init), shown: UInt32(shown), fonts: CoreTextFamilies())
            backend.reload()  // the write was core's own, so no wake says the values moved
            return (done.summary, false)
        } catch {
            return (SettingsFailure.sentence(error), true)
        }
    }

    public func othersHead(count: Int) -> String { settingsOthersHead(count: UInt32(count)) }

    public func shellProfiles() -> [ShellLine] { handle.shellProfiles().map { ShellLine(name: $0.name, commandLine: $0.commandLine) } }

    public func worktreeCaption(base: String) -> String? { handle.worktreeCaption(base: base) }

    public func previewNote(section: String) -> String? { settingsPreviewNote(section: section) }

    public func resetQuestion(count: Int) -> String { settingsResetQuestion(count: UInt32(count)) }

    public func importSelectedLine(selected: Int, total: Int) -> String {
        settingsImportSelectedLine(selected: UInt32(selected), total: UInt32(total))
    }

    public func updateUserAgent(version: String, counting: Bool, unstable: Bool) -> String? {
        settingsUpdateUserAgent(version: version, counting: counting, unstable: unstable)
    }

    public func updateCountWait() -> String? { settingsUpdateCountWait() }
}

extension RecordMapping {
    static func form(_ row: SettingsProfileForm) -> ProfileForm {
        ProfileForm(
            id: row.id, name: row.name, avatar: row.avatar, color: row.color, startDir: row.startDir, folders: row.folders,
            env: row.env, gitName: row.gitName, gitEmail: row.gitEmail,
            logins: row.logins.map { ProfileForm.Login(agent: $0.agent, name: $0.name, placeholder: $0.placeholder, dir: $0.dir) },
            startCaption: row.startCaption)
    }

    static func record(_ form: ProfileForm) -> SettingsProfileForm {
        SettingsProfileForm(
            id: form.id, name: form.name, avatar: form.avatar, color: form.color, startDir: form.startDir,
            folders: form.folders, env: form.env, gitName: form.gitName, gitEmail: form.gitEmail,
            logins: form.logins.map { SettingsLoginField(agent: $0.agent, name: $0.name, placeholder: $0.placeholder, dir: $0.dir) },
            startCaption: form.startCaption)
    }

    static func review(_ row: SettingsImportReview) -> ImportReview {
        ImportReview(
            files: row.files,
            items: row.items.map {
                ImportReview.Item(label: $0.label, from: $0.from, value: $0.value, command: $0.command, defaultOn: $0.defaultOn)
            },
            skipped: row.skipped.map { ImportReview.Skip(key: $0.key, reason: $0.reason) })
    }
}
