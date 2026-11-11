// Bringing settings over from other terminals (core `import`), as plain values for the Look step's offer and the
// Settings page: which terminals have settings on this Mac and what taking them changes.

import Foundation
import WormholeBindings

public struct ImportOffer: Equatable, Sendable, Identifiable {
    public struct Row: Equatable, Sendable {
        /// The setting's stable key (`font_size`).
        public var setting: String
        /// What the other terminal wrote, as it wrote it.
        public var from: String
        public var tickedByDefault: Bool
    }

    public var id: String { source }
    /// The stable name (`ghostty`, `iterm2`).
    public var source: String
    public var name: String
    public var files: [String]
    public var rows: [Row]
    /// How many of its keys Wormhole has no setting for.
    public var skipped: Int
}

public enum ImportBridge {
    /// The terminals with settings under `home` (the person's own when `nil`), the one changed last first. Reads
    /// files: off the main actor.
    public static func scan(home: URL? = nil) async throws -> [ImportOffer] {
        try await Task.detached { try importScan(home: home?.path) }.value.map { found in
            ImportOffer(
                source: found.source, name: found.name, files: found.files,
                rows: found.rows.map { .init(setting: $0.setting, from: $0.from, tickedByDefault: $0.defaultOn) },
                skipped: Int(found.skipped))
        }
    }

    /// Applies `source`'s settings (the ticked rows' keys, or the ones ticked by default) to the Default profile;
    /// how many were taken.
    @discardableResult
    public static func apply(source: String, home: URL? = nil, ticked: [String]? = nil, dataDir: URL? = nil) async throws -> Int {
        let taken = try await Task.detached {
            try importApply(source: source, home: home?.path, ticked: ticked, dataDir: dataDir?.path)
        }.value
        return Int(taken)
    }
}
