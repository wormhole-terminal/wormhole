// The picker's words, named like the Fluent keys of `settings-fonts.ftl` (`settings-fonts-search-fonts`). The ones
// the GPUI picker already has use its ids, so the FFI's text table fills them; the picker's additions (recents,
// coverage) carry ids of their own with the English as the fallback.

import WordsModel

public enum FontWords {
    public static var searchFonts: String { Words.text("settings-fonts-search-fonts", fallback: "Search fonts") }
    public static var bundled: String { Words.text("settings-fonts-bundled", fallback: "Bundled") }
    public static var custom: String { Words.text("settings-fonts-source-custom", fallback: "Custom") }
    public static var installed: String { Words.text("settings-fonts-source-installed", fallback: "Installed") }
    public static var recent: String { Words.text("settings-fonts-recent", fallback: "Recent") }
    public static var monospace: String { Words.text("settings-fonts-monospace", fallback: "Monospace") }
    public static var all: String { Words.text("settings-fonts-all", fallback: "All") }
    public static var monospaceTag: String { Words.text("settings-fonts-monospace-tag", fallback: "monospace") }
    public static var loading: String {
        Words.text("settings-fonts-loading-installed-fonts", fallback: "Loading installed fonts…")
    }
    public static var findingMonospace: String {
        Words.text("settings-fonts-finding-monospace-fonts", fallback: "Finding monospace fonts…")
    }
    public static func count(_ n: Int) -> String {
        Words.text("settings-fonts-count", fallback: n == 1 ? "1 font" : "\(n) fonts")
    }
    public static func noMatches(_ query: String) -> String {
        Words.text("settings-fonts-no-matches", fallback: "No font has these words in its name: “\(query)”")
    }
    public static var showAllHint: String {
        Words.text("settings-fonts-show-all-hint", fallback: "Show all fonts to include proportional ones")
    }
    public static var pickerLabel: String { Words.text("settings-fonts-picker", fallback: "Font list") }
    public static var preview: String { Words.text("settings-fonts-preview", fallback: "Preview") }
    public static var coverage: String { Words.text("settings-fonts-coverage", fallback: "Glyphs this font draws itself") }
    public static var fallsBack: String { Words.text("settings-fonts-falls-back", fallback: "falls back to another font") }
    public static var drawn: String { Words.text("settings-fonts-drawn", fallback: "drawn by this font") }
    public static var boxDrawing: String { Words.text("settings-fonts-box-drawing", fallback: "Box drawing") }
    public static var powerline: String { Words.text("settings-fonts-powerline", fallback: "Powerline") }
    public static var nerdIcons: String { Words.text("settings-fonts-nerd-icons", fallback: "Nerd icons") }
    public static var wide: String { Words.text("settings-fonts-wide", fallback: "Wide characters") }
    public static var ligatures: String { Words.text("settings-fonts-ligatures", fallback: "Ligatures") }
    public static var bold: String { Words.text("settings-fonts-bold", fallback: "Bold") }
    public static var italic: String { Words.text("settings-fonts-italic", fallback: "Italic") }
    public static var noBold: String {
        Words.text("settings-fonts-no-bold", fallback: "No bold face: bold text will be drawn heavier by the system")
    }
    public static var noItalic: String {
        Words.text("settings-fonts-no-italic", fallback: "No italic face: italic text will be slanted by the system")
    }
}
