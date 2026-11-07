import Foundation

extension L10n {
    /// The grid's size on the chip; the sign between the numbers is a multiplication sign.
    static func sizeReadout(_ columns: Int, _ rows: Int) -> String {
        tr("terminal-size-readout", "\(columns.formatted(.number.grouping(.never))) × \(rows.formatted(.number.grouping(.never)))")
    }
}
