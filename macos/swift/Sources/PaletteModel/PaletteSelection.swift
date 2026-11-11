// Which result is selected. It is kept by what it is, not by position, because rows come and go while typing;
// with nothing chosen it is the first result. ↑↓ stop at the ends instead of wrapping, as in the GPUI palette.

import Foundation

public enum PaletteSelection {
    /// The id the selection is on: `chosen` while it is still listed, else the first result.
    public static func current(chosen: String?, in rows: [PaletteRow]) -> String? {
        let ids = rows.compactMap(\.result?.id)
        return chosen.flatMap { ids.contains($0) ? $0 : nil } ?? ids.first
    }

    /// The id `delta` results away, clamped to the first and last.
    public static func step(_ delta: Int, from chosen: String?, in rows: [PaletteRow]) -> String? {
        let ids = rows.compactMap(\.result?.id)
        guard let current = current(chosen: chosen, in: rows), let at = ids.firstIndex(of: current) else {
            return nil
        }
        return ids[min(max(at + delta, 0), ids.count - 1)]
    }
}
