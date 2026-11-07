// Makes a terminal's Metal view over the source the options ask for: the mock, or the real engine running a
// shell, whose title and folder the core keeps. The shell ending goes to the closure the app sets after it has a window.

import AppKit
import Foundation
import TerminalCore
import TerminalMetal
import WorkspaceModel

@MainActor
final class TerminalFactory {
    private let options: Options
    private var configuration: RendererConfiguration?
    var onExit: (TerminalID, Int32?) -> Void = { _, _ in }
    /// The source of a terminal that the core's workspace runs; nil means the mock sources.
    private let real: ((TerminalID) -> (any FrameSource)?)?

    init(options: Options, source: ((TerminalID) -> (any FrameSource)?)?) {
        self.options = options
        real = source
    }

    /// The program's name for the exited bar ("zsh exited"): the shell asked for, else what a terminal starts with.
    var shellName: String { options.program.map { URL(fileURLWithPath: $0).lastPathComponent } ?? "zsh" }

    func make(_ id: TerminalID) -> MetalTerminalView? {
        guard let configuration = rendererConfiguration() else { return nil }
        do {
            // Each mock terminal draws from its own seed; the first keeps the one given.
            var options = options
            options.seed &+= UInt64(max(0, id - 1))
            let source: any FrameSource
            if let real {
                // The core started its program; no function result means it did not start.
                guard let started = real(id) else { return nil }
                source = started
            } else {
                source = try SourceFactory.make(options, size: GridSize(cols: 100, rows: 30))
            }
            let view = try MetalTerminalView(source: source, configuration: configuration)
            view.fixedGrid = options.grid
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
