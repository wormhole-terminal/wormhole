// Makes a terminal's Metal view over the source the options ask for: the mock, or the real engine running a
// shell. The views' own events (a title, the shell ending) go to the closures the app sets after it has a window.

import AppKit
import Foundation
import TerminalCore
import TerminalMetal
import WorkspaceModel

@MainActor
final class TerminalFactory {
    private let options: Options
    private var configuration: RendererConfiguration?
    var onTitle: (TerminalID, String) -> Void = { _, _ in }
    var onExit: (TerminalID, Int32?) -> Void = { _, _ in }

    init(options: Options) { self.options = options }

    /// The program's name for the exited bar ("zsh exited"): the shell asked for, else what a terminal starts with.
    var shellName: String { options.program.map { URL(fileURLWithPath: $0).lastPathComponent } ?? "zsh" }

    func make(_ id: TerminalID) -> MetalTerminalView? {
        guard let configuration = rendererConfiguration() else { return nil }
        do {
            // Each mock terminal draws from its own seed; the first keeps the one given.
            var options = options
            options.seed &+= UInt64(max(0, id - 1))
            let source = try SourceFactory.make(options, size: GridSize(cols: 100, rows: 30))
            let view = try MetalTerminalView(source: source, configuration: configuration)
            view.fixedGrid = options.grid
            view.onTitle = { [weak self] in self?.onTitle(id, $0) }
            view.onExit = { [weak self] in self?.onExit(id, $0) }
            return view
        } catch {
            FileHandle.standardError.write(Data("terminal \(id): cannot start: \(error)\n".utf8))
            return nil
        }
    }

    private func rendererConfiguration() -> RendererConfiguration? {
        if configuration == nil {
            let (made, note) = options.rendererConfiguration()
            if let note { FileHandle.standardError.write(Data((note + "\n").utf8)) }
            configuration = made
        }
        return configuration
    }
}
