// The menus as text, one item per line with its key and marks: for tests and devctl's `menus`, in the form
// core's `keymap::menus::dump` writes, so the two can be compared line by line.

import Foundation

public enum MenuDump {
    public static func text(_ menus: [Menu]) -> String {
        menus.map { "\($0.title)\n" + lines($0.items, depth: 1) }.joined()
    }

    public static func lines(_ items: [MenuNode], depth: Int) -> String {
        let indent = String(repeating: "  ", count: depth)
        return items.map { node -> String in
            switch node {
            case .separator: "\(indent)---\n"
            case .services: "\(indent)Services ▸\n"
            case .header(let title): "\(indent)# \(title)\n"
            case .placeholder(let title): "\(indent)\(title)  (disabled)\n"
            case .swatches(let row): "\(indent)Swatches \(row.swatches.map(\.name).joined(separator: ", "))\n"
            case .submenu(let title, let inner, let enabled):
                "\(indent)\(title) ▸\(enabled ? "" : "  (disabled)")\n" + lines(inner, depth: depth + 1)
            case .item(let item):
                "\(indent)\(item.checked ? "✓ " : "")\(item.title)\(item.key.map { "  " + $0.glyphText } ?? "")"
                    + "\(item.enabled ? "" : "  (disabled)")\n"
            }
        }.joined()
    }
}
