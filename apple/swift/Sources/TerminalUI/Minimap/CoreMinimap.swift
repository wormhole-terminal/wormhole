// `MinimapSource` over the real terminal: the geometry is read each time (Rust re-reads the lines only when the
// output or the view changed), and the lines are fetched again only when its `stamp` moved. Without a terminal
// (the source has not started, or it is the mock) the minimap is empty.

import Foundation
import TerminalCore

@MainActor
public final class CoreMinimap: MinimapSource {
    private let terminal: @MainActor () -> (any TerminalMinimap)?
    private let currentPalette: @MainActor () -> TerminalPalette
    private var cached: (stamp: UInt64, lines: [[MinimapRun]])?

    /// `palette` is read each time: a theme change reaches the minimap with the next draw.
    public init(terminal: @escaping @MainActor () -> (any TerminalMinimap)?, palette: @escaping @MainActor () -> TerminalPalette) {
        (self.terminal, currentPalette) = (terminal, palette)
    }

    public var palette: TerminalPalette { currentPalette() }

    public func snapshot(fits: Int) -> MinimapSnapshot {
        guard let terminal = terminal() else {
            return MinimapSnapshot(geometry: MinimapGeometry(total: 0, screen: 0, fits: fits, viewTop: 0), columns: 0, lines: [])
        }
        let read = terminal.minimapRead(fits: fits)
        if cached?.stamp != read.stamp { cached = (read.stamp, terminal.minimapLines()) }
        return MinimapSnapshot(
            geometry: MinimapGeometry(total: read.total, screen: read.screen, fits: read.fits, viewTop: read.viewTop),
            columns: read.columns, lines: cached?.lines ?? [])
    }

    public func scroll(toDisplayOffset offset: Int) { terminal()?.scroll(toDisplayOffset: offset) }
}
