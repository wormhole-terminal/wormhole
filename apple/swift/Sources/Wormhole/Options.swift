// What the command line and the environment asked for (`Options+Parse.swift` reads them).

import DesignSystem
import Foundation
import TerminalCore

struct Options {
    enum Mode: Equatable {
        /// A window.
        case app
        /// No window: render mock frames to PNG files.
        case renderPNG(prefix: String, frames: Int, every: Int)
    }

    var mode = Mode.app
    var scenario = MockScenario.mixed
    var seed: UInt64 = 1
    var hertz = 60.0
    var program: String?
    var grid: (cols: Int, rows: Int)?
    var fontFamily: String?
    var fontDirectory: URL?
    var pointSize: Double?
    var lineHeight: Double?
    var scale: Double?
    var background = false
    /// A bundled theme's id (`--theme nord`); nil keeps the default pick.
    var theme: String?
    var appearance: ThemeMode?
    var statusBar = false
    /// Show the first-run welcome even when this data folder has seen it (`--onboarding`).
    var onboarding = false
    var devctl: URL?
    /// `--update-feed <url>`: a feed the update window may ask. Without it no feed is ever contacted.
    var updateFeedURL: String?
    /// `--update-key <file>` / `--update-bundle <app>`: a fixture's own minisign key and the scratch bundle an install replaces
    /// (inside the data folder, or it is refused): the updater end to end without the app's key or an installed app.
    var updateKeyFile: URL?
    var updateBundle: URL?
    /// Seconds to hold the window idle and report wake-ups, then quit.
    var idleSeconds: Double?

    struct UsageError: Error, CustomStringConvertible {
        var description: String
    }
}
