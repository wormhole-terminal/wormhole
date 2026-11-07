// The command line and the environment. The environment half is what `apple/MEASURING.md` promises the
// comparison scripts: WORMHOLE_DEVCTL, WORMHOLE_SHELL, WORMHOLE_SWIFT_GRID, WORMHOLE_BACKGROUND.

import AppKit
import DesignSystem
import Foundation
import TerminalCore
import TerminalMetal

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
    var devctl: URL?
    /// Seconds to hold the window idle and report wake-ups, then quit.
    var idleSeconds: Double?

    struct UsageError: Error, CustomStringConvertible {
        var description: String
    }

    static let usage = """
        Wormhole (Swift spike) [options]

          --mock <text|flood|noise|matrix|unicode|cursor|mixed>   synthetic output (default mixed)
          --mock-hz <n>        steps a second; 0 as fast as possible (default 60)
          --seed <n>           the mock's seed (default 1)
          --shell <program>    the real engine running this program (WORMHOLE_SHELL); needs the FFI linked in
          --grid <COLSxROWS>   a fixed grid, the window sized to fit (WORMHOLE_SWIFT_GRID)
          --font <family>      an installed font instead of the bundled JetBrains Mono
          --font-dir <dir>     the folder with the four JetBrainsMono-*.ttf (WORMHOLE_FONT_DIR)
          --size <points>      font size (default 14 with JetBrains Mono, 13 otherwise)
          --line-height <x>    a multiple of the font size (1.3 with JetBrains Mono)
          --scale <x>          device pixels per point (default: the main screen's)
          --render-png <prefix> [--frames N] [--png-every K]
                               no window: write <prefix>-<step>.png of the mock, one per K steps of N
          --idle <seconds>     hold the window idle that long, print the wake-ups, quit
          --theme <id>         a bundled theme: shows it, and makes it the pick for its appearance
          --appearance <light|dark|system>   which appearance shows (default system)
          --status-bar         show the optional status bar
          --background         accessory app, window behind the others, nothing takes focus (WORMHOLE_BACKGROUND=1)

        Needs WORMHOLE_DATA_DIR=<a folder named Wormhole-swift*>; WORMHOLE_DEVCTL=<dir> turns on the control folder.
        """

    static func parse(_ arguments: [String], environment: [String: String]) throws -> Options {
        var options = Options()
        options.program = environment["WORMHOLE_SHELL"].flatMap { $0.isEmpty ? nil : $0 }
        options.background = environment["WORMHOLE_BACKGROUND"] == "1"
        options.devctl = environment["WORMHOLE_DEVCTL"].flatMap { $0.isEmpty ? nil : URL(fileURLWithPath: $0) }
        options.fontDirectory = environment["WORMHOLE_FONT_DIR"].map { URL(fileURLWithPath: $0) }
        if let grid = environment["WORMHOLE_SWIFT_GRID"] { options.grid = try parseGrid(grid) }

        var prefix: String?
        var frames = 1, every = 1
        var index = 0
        func value(_ flag: String) throws -> String {
            index += 1
            guard index < arguments.count else { throw UsageError(description: "\(flag) needs a value") }
            return arguments[index]
        }
        func number(_ flag: String) throws -> Double {
            guard let parsed = Double(try value(flag)) else { throw UsageError(description: "\(flag) needs a number") }
            return parsed
        }
        while index < arguments.count {
            let flag = arguments[index]
            switch flag {
            case "--mock":
                let name = try value(flag)
                guard let scenario = MockScenario(rawValue: name) else {
                    throw UsageError(description: "no scenario \(name) (\(MockScenario.allCases.map(\.rawValue).joined(separator: ", ")))")
                }
                options.scenario = scenario
                options.program = nil
            case "--mock-hz": options.hertz = try number(flag)
            case "--seed": options.seed = UInt64(try number(flag))
            case "--shell": options.program = try value(flag)
            case "--grid": options.grid = try parseGrid(try value(flag))
            case "--font": options.fontFamily = try value(flag)
            case "--font-dir": options.fontDirectory = URL(fileURLWithPath: try value(flag))
            case "--size": options.pointSize = try number(flag)
            case "--line-height": options.lineHeight = try number(flag)
            case "--scale": options.scale = try number(flag)
            case "--render-png": prefix = try value(flag)
            case "--frames": frames = Int(try number(flag))
            case "--png-every": every = max(1, Int(try number(flag)))
            case "--idle": options.idleSeconds = try number(flag)
            case "--background": options.background = true
            case "--status-bar": options.statusBar = true
            case "--theme":
                let id = try value(flag)
                guard ThemeLibrary.bundled.theme(id: id) != nil else {
                    let known = ThemeLibrary.bundled.themes.map(\.id).joined(separator: ", ")
                    throw UsageError(description: "no theme \(id) (\(known))")
                }
                options.theme = id
            case "--appearance":
                let name = try value(flag)
                guard let mode = ThemeMode(rawValue: name) else { throw UsageError(description: "--appearance is light, dark or system") }
                options.appearance = mode
            case "-h", "--help": throw UsageError(description: usage)
            default: throw UsageError(description: "unknown option \(flag)\n\n\(usage)")
            }
            index += 1
        }
        if let prefix { options.mode = .renderPNG(prefix: prefix, frames: max(1, frames), every: every) }
        return options
    }

    static func parseGrid(_ text: String) throws -> (Int, Int) {
        let parts = text.lowercased().split(separator: "x").compactMap { Int($0) }
        guard parts.count == 2, parts.allSatisfy({ $0 > 0 }) else { throw UsageError(description: "a grid is COLSxROWS, not \(text)") }
        return (parts[0], parts[1])
    }

    // MARK: the renderer's configuration

    /// JetBrains Mono at 14 pt with a 1.3 line, as the GPUI app has it, when the font folder can be found;
    /// Menlo otherwise (and then the caller says so).
    func rendererConfiguration() -> (RendererConfiguration, note: String?) {
        let scale = self.scale ?? Double(NSScreen.main?.backingScaleFactor ?? 2)
        if let family = fontFamily {
            return (RendererConfiguration(font: .system(family: family), pointSize: pointSize ?? 13, lineHeight: lineHeight, scale: scale), nil)
        }
        if let directory = fontDirectory ?? Self.findBundledFonts() {
            return (
                RendererConfiguration(
                    font: .jetBrainsMono(directory: directory), pointSize: pointSize ?? 14, lineHeight: lineHeight ?? 1.3, scale: scale),
                nil
            )
        }
        return (
            RendererConfiguration(font: .system(family: "Menlo"), pointSize: pointSize ?? 13, lineHeight: lineHeight, scale: scale),
            "JetBrains Mono not found (set WORMHOLE_FONT_DIR): using Menlo, so sizes differ from the GPUI app"
        )
    }

    /// The repository's bundled fonts, found by walking up from the executable and from the working folder.
    static func findBundledFonts() -> URL? {
        let relative = "crates/wormhole-core/assets/fonts/JetBrainsMono"
        var starts = [URL(fileURLWithPath: FileManager.default.currentDirectoryPath)]
        if let executable = Bundle.main.executableURL { starts.append(executable.deletingLastPathComponent()) }
        for start in starts {
            var directory = start.standardizedFileURL
            for _ in 0..<12 {
                let candidate = directory.appendingPathComponent(relative)
                if FileManager.default.fileExists(atPath: candidate.appendingPathComponent("JetBrainsMono-Regular.ttf").path) {
                    return candidate
                }
                directory.deleteLastPathComponent()
            }
        }
        return nil
    }
}
