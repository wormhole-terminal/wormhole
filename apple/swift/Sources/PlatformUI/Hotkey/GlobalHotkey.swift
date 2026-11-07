// The system-wide show/hide hotkey (`platform/hotkey.rs`): Carbon's `RegisterEventHotKey`, which needs no
// Accessibility or Input Monitoring permission and is dispatched by whatever runs the application's event loop.
// A test instance (`--background`) never registers anything: the spec is checked as anywhere and the key is shown
// as it would be, but it holds nothing, so a test never takes a key from the person at the Mac.

import Carbon.HIToolbox
import Foundation
import PlatformModel

@MainActor
public final class GlobalHotkey {
    public enum Failure: Error, Equatable, CustomStringConvertible {
        case unknownKey(String)
        case registration(OSStatus)

        public var description: String {
            switch self {
            case .unknownKey(let key): "no key code for \(key)"
            case .registration(let status): "another app owns this combination (error \(status))"
            }
        }
    }

    /// Four-character signature of our hotkeys ("WHhk"), so the handler ignores another library's.
    private static let signature: OSType = 0x5748_686B

    public private(set) var spec: KeySpec?
    /// Whether the system holds the key: false in a test instance.
    public private(set) var registered = false
    // Read by `deinit`, which is not isolated; only the main actor writes them.
    private nonisolated(unsafe) var reference: EventHotKeyRef?
    private nonisolated(unsafe) var handler: EventHandlerRef?
    private let policy: PlatformPolicy
    private let onPress: @MainActor () -> Void

    public init(policy: PlatformPolicy = .current(), onPress: @escaping @MainActor () -> Void) {
        self.policy = policy
        self.onPress = onPress
    }

    deinit {
        // The Carbon handles are plain pointers: releasing them needs no actor.
        if let reference { UnregisterEventHotKey(reference) }
        if let handler { RemoveEventHandler(handler) }
    }

    /// Registers `spec`, replacing the previous key. Throws when the key has no code or another app owns it.
    public func register(_ spec: KeySpec) throws {
        guard let (code, modifiers) = KeyCodes.code(for: spec) else { throw Failure.unknownKey(spec.glyphText) }
        unregister()
        self.spec = spec
        guard policy.touchesSystem else { return }
        try installHandler()
        var ref: EventHotKeyRef?
        let status = RegisterEventHotKey(code, modifiers, EventHotKeyID(signature: Self.signature, id: 1), GetApplicationEventTarget(), 0, &ref)
        guard status == noErr, let ref else { throw Failure.registration(status) }
        reference = ref
        registered = true
    }

    /// Lets the key go (a cleared setting, a changed key).
    public func unregister() {
        if let reference { UnregisterEventHotKey(reference) }
        reference = nil
        registered = false
        spec = nil
    }

    /// Delivers a press: the Carbon callback, and a test's way to press the key.
    func pressed() { onPress() }

    private func installHandler() throws {
        guard handler == nil else { return }
        var kind = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        let status = InstallEventHandler(
            GetApplicationEventTarget(),
            { _, event, context in
                guard let event, let context else { return OSStatus(eventNotHandledErr) }
                var id = EventHotKeyID()
                GetEventParameter(event, EventParamName(kEventParamDirectObject), EventParamType(typeEventHotKeyID), nil, MemoryLayout<EventHotKeyID>.size, nil, &id)
                guard id.signature == GlobalHotkey.signature else { return OSStatus(eventNotHandledErr) }
                let hotkey = Unmanaged<GlobalHotkey>.fromOpaque(context).takeUnretainedValue()
                // Carbon delivers on the main thread, inside the application's event loop.
                MainActor.assumeIsolated { hotkey.pressed() }
                return noErr
            }, 1, &kind, Unmanaged.passUnretained(self).toOpaque(), &handler)
        guard status == noErr else { throw Failure.registration(status) }
    }
}
