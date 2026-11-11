// One section of the gallery page: a component of the design on sample data, in the live (previewed) theme,
// with what it is and where it belongs. What the selected token is drawn on is outlined on each element that draws
// with it (`.marked`, which the views carry), and the elements' tokens come from the sources themselves
// (`GalleryUses`), so the outline and the editor's "Used by" are always what the views use. The view is
// `SpecimenContent`'s.

import StudioModel

struct Specimen: Identifiable, Sendable {
    enum Column: Sendable { case left, right, both }

    let id: String
    let title: String
    let detail: String
    /// The part of the app it belongs to (the gallery's filter); `nil` for the basics, listed under All only.
    let area: StudioArea?
    let column: Column

    init(_ id: String, _ title: String, _ detail: String, area: StudioArea?, column: Column) {
        (self.id, self.title, self.detail, self.area, self.column) = (id, title, detail, area, column)
    }
}

/// What a specimen is told: the token selected, and how to select another by clicking a swatch.
struct GalleryContext {
    var selected: String?
    var select: (String) -> Void = { _ in }
}
