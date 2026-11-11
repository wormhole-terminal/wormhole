// A dragged terminal as a drag session carries it: plain text, so a sidebar row, a pane header and the
// pane area's drop agree without a type the app must declare.

import CoreTransferable
import UniformTypeIdentifiers

extension DraggedTerminal: Transferable {
    public static var transferRepresentation: some TransferRepresentation {
        DataRepresentation(contentType: .plainText) { Data($0.encoded.utf8) } importing: { data in
            guard let text = String(data: data, encoding: .utf8), let dragged = DraggedTerminal(encoded: text) else {
                throw CocoaError(.fileReadCorruptFile)
            }
            return dragged
        }
    }
}
