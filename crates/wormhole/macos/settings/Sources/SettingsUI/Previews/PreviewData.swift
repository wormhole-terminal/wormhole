import SwiftUI
import AppKit

/// What the previews draw from: the terminal's fonts, the themes in effect for dark and light, and the accent.
struct PreviewData: Equatable {
    struct Side: Equatable { var palette: [UInt32]?; var accent: UInt32? }
    struct Face: Equatable { var family: String; var size: Double }
    var fonts: [String: Face] = [:]
    var lineHeight = 1.3
    var ligatures = true
    var dark = Side(), light = Side()

    init?(json: [String: Any]) {
        func side(_ k: String) -> Side {
            let d = json[k] as? [String: Any] ?? [:]
            let p = (d["palette"] as? [Int])?.map { UInt32($0) }
            return Side(palette: (p?.count ?? 0) >= 7 ? p : nil, accent: (d["accent"] as? Int).map { UInt32($0) })
        }
        guard let f = json["fonts"] as? [String: [Any]] else { return nil }
        for (slot, v) in f { if let n = v.first as? String, let s = (v.last as? NSNumber)?.doubleValue { fonts[slot] = Face(family: n, size: s) } }
        lineHeight = (json["line_height"] as? NSNumber)?.doubleValue ?? 1.3
        ligatures = json["ligatures"] as? Bool ?? true
        dark = side("dark"); light = side("light")
    }
    func side(light l: Bool) -> Side { l ? light : dark }
}

extension SettingsModel {
    var preview: PreviewData? { store.preview }
    /// The terminal's own face, as the app resolved it.
    func terminalFont(_ size: CGFloat) -> Font {
        specimenFont(preview?.fonts["terminal"]?.family ?? "JetBrains Mono", size: size)
    }
    /// The primary the app draws with in this mode, for the previews.
    func previewAccent(light: Bool) -> Color? { preview?.side(light: light).accent.map { Color(hex: $0) } }
}
