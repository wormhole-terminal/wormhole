// A file name that is not text (`wormhole-ffi` `files_panel::names`). The disk's names are bytes; one that is not valid
// UTF-8 comes from the core as a key: its text as it is and each other byte as a NUL followed by the byte's two hex
// digits. A key is opaque: paths are joined from keys and handed back to the core, which reads the bytes back. Here it is
// only told apart, drawn (the lost bytes as U+FFFD, as the core's label has them) and turned into a file URL of the same
// bytes for Finder. Hostile input: a key that does not read back is no URL.

import Foundation

public enum DiskName {
    private static let mark: UInt8 = 0

    /// Whether the name or path carries bytes that are not text.
    public static func isKey(_ text: String) -> Bool { text.utf8.contains(mark) }

    /// The name or path as it may be drawn or typed over: every carried byte reads U+FFFD.
    public static func shown(_ text: String) -> String {
        guard isKey(text) else { return text }
        var out = String.UnicodeScalarView()
        var skip = 0
        for scalar in text.unicodeScalars {
            if skip > 0 { skip -= 1 } else if scalar.value == 0 { out.append("\u{FFFD}"); skip = 2 } else { out.append(scalar) }
        }
        return String(out)
    }

    /// The file URL of an absolute path, with the very bytes a key carries; nil for a key that does not read back.
    public static func url(_ path: String) -> URL? {
        guard isKey(path) else { return URL(fileURLWithPath: path) }
        var bytes: [CChar] = []
        var rest = path.utf8[...]
        while let first = rest.popFirst() {
            guard first == mark else { bytes.append(CChar(bitPattern: first)); continue }
            let hex = rest.prefix(2)
            // Only a byte that is not ASCII is carried this way (the core refuses the rest too).
            guard hex.count == 2, let byte = UInt8(String(decoding: hex, as: UTF8.self), radix: 16), byte >= 0x80 else { return nil }
            bytes.append(CChar(bitPattern: byte))
            rest = rest.dropFirst(2)
        }
        bytes.append(0)
        return bytes.withUnsafeBufferPointer { buffer in
            buffer.baseAddress.map { URL(fileURLWithFileSystemRepresentation: $0, isDirectory: false, relativeTo: nil) }
        }
    }
}
