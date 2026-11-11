// The settings window's pages as data: sections, groups and rows, with their words and the `config.json` keys
// they write. Read from `Resources/settings.json` (`macos/scripts/dump-settings.sh` writes it from
// `wormhole-core`'s settings index); a file this code can't read is a build error, so loading traps.

import Foundation

public struct SettingsSchema: Sendable {
    public static let formatVersion = 1

    public struct Entry: Sendable, Decodable, Identifiable, Equatable {
        public let id: String
        public let label: String
        public let description: String
        public let keywords: String
        public let keys: [String]
        /// A warning that stays on the page instead of behind the (i).
        public let visibleHint: Bool
        public let control: ControlSpec
        /// The optional feature the row is about (`notes`), so a window with it off can hide the row.
        public let feature: String?
        public let visibleWhen: [Condition]?

        public struct Condition: Sendable, Decodable, Equatable {
            public let key: String
            public let equals: JSONValue
        }
    }

    public struct Group: Sendable, Decodable, Equatable {
        public let title: String
        public let blurb: String
        public let aside: String
        public let entries: [Entry]
    }

    public struct Section: Sendable, Decodable, Identifiable, Equatable {
        public let id: String
        public let title: String
        public let intro: String
        /// The nav group's id and title (`look`, "Look"); an empty title lists the section without a heading.
        public let nav: String
        public let navTitle: String
        public let preview: Bool
        public let groups: [Group]
        public var entries: [Entry] { groups.flatMap(\.entries) }
    }

    public let sections: [Section]
    /// Every key rows write, with the value it has when the file says nothing.
    public let defaults: [String: JSONValue]

    public func section(_ id: String) -> Section? { sections.first { $0.id == id } }

    public func entry(_ id: String) -> (section: Section, group: Group, entry: Entry)? {
        for section in sections {
            for group in section.groups {
                if let entry = group.entries.first(where: { $0.id == id }) { return (section, group, entry) }
            }
        }
        return nil
    }

    /// Parses a dump; throws for a file that is not the current format.
    public init(json: Data) throws {
        let dump = try JSONDecoder().decode(Dump.self, from: json)
        guard dump.format == Self.formatVersion else {
            throw DecodingError.dataCorrupted(
                .init(codingPath: [], debugDescription: "settings.json format \(dump.format), expected \(Self.formatVersion)")) // text: a decoding error, for developers
        }
        (sections, defaults) = (dump.sections, dump.defaults)
    }

    public static let bundled: SettingsSchema = {
        do {
            guard let url = Bundle.module.url(forResource: "settings", withExtension: "json") else {
                throw CocoaError(.fileNoSuchFile)
            }
            return try SettingsSchema(json: Data(contentsOf: url))
        } catch {
            fatalError("SettingsModel: settings.json is missing or unreadable: \(error)")
        }
    }()

    private struct Dump: Decodable {
        let format: Int
        let sections: [Section]
        let defaults: [String: JSONValue]
    }
}
