// What macOS says about appearance, Increase Contrast, Reduce Transparency and Reduce Motion, and when it changes.
// KVO on `effectiveAppearance` and the workspace's display-options notification: events, never polling.

import AppKit

@MainActor
final class SystemLook {
    struct Snapshot {
        let appearance: ThemeAppearance
        let increaseContrast: Bool
        let reduceTransparency: Bool
        let reduceMotion: Bool
    }

    var onChange: ((Snapshot) -> Void)?
    private var appearanceObservation: NSKeyValueObservation?
    // Read by `deinit`, which is not isolated; written once in `init`.
    private nonisolated(unsafe) var displayObserver: (any NSObjectProtocol)?

    init() {
        appearanceObservation = NSApplication.shared.observe(\.effectiveAppearance) { [weak self] _, _ in
            Task { @MainActor in self?.notify() }
        }
        displayObserver = NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil, queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.notify() }
        }
    }

    deinit {
        if let displayObserver { NSWorkspace.shared.notificationCenter.removeObserver(displayObserver) }
    }

    var snapshot: Snapshot {
        let workspace = NSWorkspace.shared
        let dark = NSApplication.shared.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
        return Snapshot(
            appearance: dark ? .dark : .light,
            increaseContrast: workspace.accessibilityDisplayShouldIncreaseContrast,
            reduceTransparency: workspace.accessibilityDisplayShouldReduceTransparency,
            reduceMotion: workspace.accessibilityDisplayShouldReduceMotion)
    }

    private func notify() { onChange?(snapshot) }
}
