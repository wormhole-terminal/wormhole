// What a diff against the working tree adds for the coding agent in the terminal (`features/git/diff/review.rs`): a
// click on a line writes a comment on it, commented lines wear a bar, and while the terminal has comments a bar under
// the rows gathers them with Paste into Terminal.

import PanelModel

struct DiffReview {
    var review: Review
    let clear: () -> Void
    let paste: () -> Void
    /// Opens the comment editor on a line, with the comment it has: the store holds what is written (`commenting`).
    var comment: (ReviewComment) -> Void = { _ in }
}

/// A line a click comments on: which side it is on, its number there and its text. An added or unchanged line is
/// pinned by its number in the new file, a removed one by its number in the old.
struct LineAt {
    let side: LineSide
    let line: Int
    let code: String

    init?(_ cell: DiffCell) {
        let removed = cell.kind == .removed
        guard let number = removed ? cell.old : cell.new else { return nil }
        (side, line, code) = (removed ? .old : .new, number, cell.text)
    }
}
