// The global hotkey as Settings sets it (`hotkey.rs`): registered as the setting says, held off while the recorder in Settings
// has the keys (so pressing the current combination records it instead of opening the quick panel), and saying why it did not
// register. Everything up to the system call runs in a test instance too: `GlobalHotkey` holds nothing there, so a test never
// takes a key from the person at the Mac, but the decisions, the shown key and a press are the same.

import Foundation
import PlatformModel

@MainActor
public final class HotkeyController {
    /// What a press does: `hotkey_action`.
    public enum Action: Sendable, Equatable { case quickPanel, toggleApp }

    /// The setting last applied, so a bad one is not tried again at every change.
    public private(set) var applied: String?
    public private(set) var action = Action.quickPanel
    /// Held off while the Settings recorder records.
    public private(set) var paused = false
    /// Why the key set now did not register.
    public private(set) var error: String?
    private var key: GlobalHotkey?
    private let policy: PlatformPolicy
    private let onPress: @MainActor (Action) -> Void

    public init(policy: PlatformPolicy = .current(), onPress: @escaping @MainActor (Action) -> Void) {
        self.policy = policy
        self.onPress = onPress
    }

    /// Never an empty spec, and not while the recorder has the keys.
    nonisolated static func registers(_ spec: String, paused: Bool) -> Bool { !spec.isEmpty && !paused }

    /// The settings `hotkey` (config.json's spelling) and `hotkey_action` (`toggle_app` or the quick panel).
    public func apply(spec: String, action: String) {
        self.action = action == "toggle_app" ? .toggleApp : .quickPanel
        let spec = spec.trimmingCharacters(in: .whitespaces)
        guard spec != applied else { return }
        register(spec)
    }

    /// Holds the key off while its field in Settings records, or puts it back.
    public func pause(_ on: Bool) {
        guard paused != on else { return }
        paused = on
        if let applied { register(applied) }
    }

    /// The key as the menus show it: the spec in force while it is registered or held off by a recording.
    public var shown: String? {
        let registered = key != nil || (paused && error == nil)
        return applied.flatMap { registered && !$0.isEmpty ? $0 : nil }
    }

    /// A press: the system's, and a test's way to press the key.
    public func press() { onPress(action) }

    private func register(_ spec: String) {
        key?.unregister()
        key = nil
        error = nil
        applied = spec
        guard Self.registers(spec, paused: paused) else { return }
        guard let parsed = try? KeySpec(parsing: spec) else {
            error = "no key spec \(spec)"
            return
        }
        let candidate = GlobalHotkey(policy: policy) { [weak self] in self?.press() }
        do {
            try candidate.register(parsed)
            key = candidate
        } catch {
            self.error = "\(error)"
            FileHandle.standardError.write(Data("hotkey \(spec): \(error)\n".utf8))
        }
    }
}
