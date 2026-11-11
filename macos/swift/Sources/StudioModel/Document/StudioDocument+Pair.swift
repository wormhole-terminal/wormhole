// A single theme made a pair (Add Light / Dark Appearance, `studio/commit.rs`) and a copy brought up to date with the
// theme it was copied from (Update from, `studio/update_from.rs`). The rules are core's (`StudioFiles`); the draft takes
// the answer as one undoable step, after Rust resolved it, like every edit.

import DesignSystem
import Foundation

extension StudioDocument {
    /// The appearance Add Appearance would add: the one a single theme lacks; `nil` for a theme that is a pair
    /// already, or a newer file, or when there are no core rules to make one.
    public var missingAppearance: ThemeAppearance? {
        guard environment.files != nil, file.versions.isEmpty, !isNewer else { return nil }
        return theme.appearance == .light ? .dark : .light
    }

    /// Make the single theme a pair: what it is becomes one appearance and `to` is added.
    public func addAppearance(_ to: ThemeAppearance) {
        guard let files = environment.files, missingAppearance != nil else { return }
        let id = file.id ?? files.newID(name: name, itself: subject.url)
        do {
            let made = try ThemeFile(parsing: files.addAppearance(file.json(), dark: to == .dark, id: id))
            try take(StudioText.undoAddAppearance) { $0 = made }
            switchVersion(to: to.rawValue)
        } catch {
            notice = .failed(StudioText.addFailed)
        }
    }

    /// The sheet's rows: what changed in the source since the base was copied.
    public func updateRows() -> UpdateCheck? {
        guard let files = environment.files, let check = try? files.updateCheck(file.json(), shown: shown), check.state == .changed else { return nil }
        update = check
        return check
    }

    /// Replace the base under the shown version by a fresh copy of its source, as one step. Your own values stay.
    public func applyUpdate() {
        guard let files = environment.files, let text = try? files.updateApply(file.json(), shown: shown), let made = try? ThemeFile(parsing: text) else { return }
        do {
            try take(StudioText.undoUpdateFrom) { $0 = made }
            updateKey = nil
            refreshUpdate()
        } catch {
            fieldError = FieldError(token: "", text: "", message: (error as? LocalizedError)?.errorDescription ?? "\(error)")
        }
    }
}
