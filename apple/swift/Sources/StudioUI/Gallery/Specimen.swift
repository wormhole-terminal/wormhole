// One section of the gallery page: a component of the design on sample data, in the live (previewed) theme,
// with what it is and where it belongs. The tokens it draws with come from the sources themselves
// (`GalleryUses`), so the selected token's outline and the editor's "Used by" are always what the views use.
// The facts are plain data (the Studio's token knowledge reads them off the main actor); the view is
// `SpecimenContent`'s.

import DesignSystem
import StudioModel

struct Specimen: Identifiable, Sendable {
    enum Column: Sendable { case left, right, both }

    let id: String
    let title: String
    let detail: String
    /// The part of the app it belongs to (the gallery's filter); `nil` for the basics, listed under All only.
    let area: StudioArea?
    let column: Column
    let tokens: [String]

    init(_ id: String, _ title: String, _ detail: String, area: StudioArea?, column: Column, tokens: [String]) {
        (self.id, self.title, self.detail, self.area, self.column, self.tokens) = (id, title, detail, area, column, tokens)
    }

    var use: TokenUse { TokenUse(id: id, label: title, area: area ?? .all, tokens: tokens) }
}

/// What a specimen is told: the token selected, and how to select another by clicking a swatch.
struct GalleryContext {
    var selected: String?
    var select: (String) -> Void = { _ in }
}
