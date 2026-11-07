// What the viewer has to show of a file (`wormhole_core::storage::files::viewing::Loaded`).

import Foundation

public enum FileContent: Equatable, Sendable {
    case reading
    /// Text, whole or its first bytes (`truncated`); `size` is the file's in bytes.
    case text(String, size: Int, truncated: Bool)
    case binary(size: Int)
    case image(Data, size: Int)
    case unavailable(Problem)

    public enum Problem: Sendable { case gone, outside, notAFile, denied, other }

    public var size: Int? {
        switch self {
        case let .text(_, size, _), let .binary(size), let .image(_, size): size
        default: nil
        }
    }
}

/// Why a rename was refused (`RenameError`): the field stays open and the notice says why.
public enum RenameFailure: Error, Sendable { case name, exists, denied, gone, other }

/// A byte count as the viewer writes it, "48.2 KB" (the app's `size_units` setting decides decimal or binary; decimal
/// here, as Finder).
public enum ByteSize {
    public static func text(_ bytes: Int) -> String { bytes.formatted(.byteCount(style: .file)) }
}
