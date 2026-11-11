// The keys a version 1 theme file spells differently from the resolver's own names (`terminal.cursor_text` is
// `cursor.text` in the file). The table is core's (`theme::tokens::format::keys::KEY_TABLE`); this copy is the draft's,
// and a test against the FFI (`studio_key_table`) fails when the two differ.

enum StudioKeys {
    static let table: [(block: String, old: String, new: String)] = [
        ("terminal", "cursor_text", "cursor.text"), ("terminal", "selection_background", "selection.background"),
        ("terminal", "selection_foreground", "selection.foreground"), ("metrics", "radius_sm", "radius.sm"),
        ("metrics", "radius_chip", "radius.chip"), ("metrics", "radius_lg", "radius.lg"),
        ("metrics", "border_width", "border.width"), ("metrics", "divider_width", "divider.width"),
        ("metrics", "focus_ring_width", "focus_ring.width"), ("metrics", "ui_font_family", "ui_font.family"),
        ("metrics", "ui_font_size", "ui_font.size"),
    ]

    /// The version 1 spelling of `key` in `block` (the key itself when it did not change).
    static func new(_ block: String, _ key: String) -> String { table.first { $0.block == block && $0.old == key }?.new ?? key }
}
