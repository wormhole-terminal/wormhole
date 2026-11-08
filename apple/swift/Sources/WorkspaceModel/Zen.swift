// How much a window shows under a preset (`wormhole_core::config::presets::Shown`): Zen mode draws every window with the
// Zen preset's values for a while and saves nothing. The core owns the values; `WorkspaceBridge` installs the Zen preset
// (a build without the core has none, and Zen mode is unavailable, never different).

import Foundation
import Synchronization

public struct WindowShown: Equatable, Sendable {
    /// The title row's parts as `config.json` names them.
    public var titleRow: [String]
    public var statusBar: Bool
    public var minimap: Bool
    public var columnDividers: Bool

    public init(titleRow: [String], statusBar: Bool, minimap: Bool, columnDividers: Bool) {
        (self.titleRow, self.statusBar, self.minimap, self.columnDividers) = (titleRow, statusBar, minimap, columnDividers)
    }
}

public enum Zen {
    private static let installed = Mutex<WindowShown?>(nil)

    public static func install(_ preset: WindowShown?) { installed.withLock { $0 = preset } }

    /// The Zen preset, nil without the core.
    public static var preset: WindowShown? { installed.withLock { $0 } }
}
