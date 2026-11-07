// Changing the draft: a value typed or picked, a reset, a rename, undo and redo, revert to saved. A value is
// resolved before it is taken; one Rust refuses (an unknown name, a loop) stays in the field with the reason,
// and the draft keeps its last good value (themes.md: "applied on Return").

import DesignSystem
import Foundation

extension StudioDocument {
    /// Sets `token` to what was typed. Empty resets it. Returns whether the draft took it.
    @discardableResult
    public func set(_ token: String, to text: String) -> Bool {
        let value = text.trimmingCharacters(in: .whitespaces)
        if token.hasPrefix("metrics."), !value.isEmpty, Double(value.replacingOccurrences(of: "px", with: "")) == nil {
            fieldError = FieldError(token: token, text: text, message: StudioText.notPixels(token, value))
            return false
        }
        let label = value.isEmpty ? StudioText.undoReset(token) : StudioText.undoChange(token)
        return attempt(token, text, label) { $0.set(token, value.isEmpty ? nil : value, version: shown, group: group(of: token)) }
    }

    @discardableResult
    public func reset(_ token: String) -> Bool { set(token, to: "") }

    public func rename(_ name: String) {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty, trimmed != file.name else { return }
        _ = try? take(StudioText.undoRename) { $0.name = trimmed }
    }

    /// Moves a value between both versions of a pair and this one alone.
    public func move(_ token: String, to place: ThemeFile.Place) {
        _ = try? take(StudioText.undoMove(token)) { $0.move(token, to: place, version: shown) }
    }

    public func switchVersion(to version: String) {
        guard file.versions.contains(version) else { return }
        shown = version
        refresh()
    }

    public var undoLabel: String? { history.undoLabel }
    public var redoLabel: String? { history.redoLabel }

    public func undo() { history.undo().map(load) }
    public func redo() { history.redo().map(load) }

    public func revertToSaved() { _ = try? take(StudioText.undoRevert) { $0 = saved } }

    func group(of token: String) -> String? { TokenGroup.group(of: token, in: groups) }

    /// Applies `change` to a copy of the file and takes it when Rust resolves it; throws Rust's reason otherwise.
    func take(_ label: String, _ change: (inout ThemeFile) -> Void) throws {
        var candidate = file
        change(&candidate)
        guard candidate != file else { return }
        let resolved = try workspace.resolve(candidate, version: shown, increaseContrast: store.increaseContrast)
        file = candidate
        history.record(label, candidate)
        apply(resolved)
    }

    private func attempt(_ token: String, _ text: String, _ label: String, _ change: (inout ThemeFile) -> Void) -> Bool {
        do {
            try take(label, change)
            fieldError = nil
            return true
        } catch {
            let reason = (error as? LocalizedError)?.errorDescription ?? "\(error)"
            fieldError = FieldError(token: token, text: text, message: reason)
            return false
        }
    }

    private func load(_ file: ThemeFile) {
        self.file = file
        fieldError = nil
        if let resolved = try? workspace.resolve(file, version: shown, increaseContrast: store.increaseContrast) { apply(resolved) }
    }
}
