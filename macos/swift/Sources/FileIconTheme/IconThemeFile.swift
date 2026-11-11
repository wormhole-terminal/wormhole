// The icon theme file as it is written (Zed's icon theme format, v0.3.0, plus the `x-wormhole` block): the folder icons,
// the names and suffixes that pick a file's icon type, and the SVG each type has. Tolerant: unknown fields are ignored,
// so Zed's own files load unchanged.

struct IconThemeFile: Decodable {
    struct Pair: Decodable {
        var collapsed: String?
        var expanded: String?
    }

    struct Icon: Decodable { var path: String }

    struct Theme: Decodable {
        var appearance: String
        var directoryIcons: Pair?
        var namedDirectoryIcons: [String: Pair]?
        var fileStems: [String: String]?
        var fileSuffixes: [String: String]?
        var fileIcons: [String: Icon]?

        enum CodingKeys: String, CodingKey {
            case appearance
            case directoryIcons = "directory_icons"
            case namedDirectoryIcons = "named_directory_icons"
            case fileStems = "file_stems"
            case fileSuffixes = "file_suffixes"
            case fileIcons = "file_icons"
        }
    }

    struct Extension: Decodable { var render: String? }

    var name: String
    var wormhole: Extension?
    var themes: [Theme]

    enum CodingKeys: String, CodingKey {
        case name, themes
        case wormhole = "x-wormhole"
    }
}
