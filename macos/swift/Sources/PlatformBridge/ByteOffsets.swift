// Core marks where text matched in bytes of its UTF-8; the views mark characters.

import WormholeBindings

/// A range of UTF-8 bytes of `text` as the range of `Character`s it covers.
func characters(_ bytes: ByteRangeRecord, in text: String) -> Range<Int> {
    let utf8 = text.utf8
    guard let start = utf8.index(utf8.startIndex, offsetBy: Int(bytes.start), limitedBy: utf8.endIndex),
        let end = utf8.index(utf8.startIndex, offsetBy: Int(bytes.end), limitedBy: utf8.endIndex)
    else { return 0..<0 }
    return text.distance(from: text.startIndex, to: start)..<text.distance(from: text.startIndex, to: end)
}
