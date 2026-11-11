// The keys of the open list: arrows walk the families (skipping captions, stopping at the ends as a list does),
// Return picks the row, and a search that is not empty is cleared before the picker closes.

extension FontPickerModel {
    private var selectable: [FontEntry] { entries.filter(\.isSelectable) }

    /// Moves the highlight `steps` rows (negative: up) and brings it into view.
    public func move(_ steps: Int) {
        let rows = selectable
        guard !rows.isEmpty else { return }
        let at = rows.firstIndex { $0.id == highlighted } ?? (steps > 0 ? -1 : rows.count)
        highlighted = rows[min(max(at + steps, 0), rows.count - 1)].id
        scrollTarget = highlighted
    }

    public func moveToEdge(first: Bool) { move(first ? -selectable.count : selectable.count) }

    /// The pointer is over a row: the preview follows it, the list does not scroll.
    public func hover(_ entry: FontEntry?) {
        guard let entry, entry.isSelectable else { return }
        highlighted = entry.id
    }

    /// Return: picks the highlighted row.
    public func commitHighlighted() {
        if let entry = entries.first(where: { $0.id == highlighted }) { choose(entry) }
    }

    /// Esc: false when there was a search to clear (the picker stays), true when it should close.
    public func escape() -> Bool {
        if query.isEmpty { return true }
        query = ""
        return false
    }

    /// What the preview says about `family`, asked of the system once.
    public func coverage(of family: String) -> FontCoverage {
        if let known = coverages[family] { return known }
        let system = library.system
        let drawn = Dictionary(uniqueKeysWithValues: GlyphGroup.all.map { ($0.id, system.covers(family, $0.scalars)) })
        let made = FontCoverage(drawn: drawn, ligatures: system.hasLigatures(family), styles: system.styles(of: family))
        coverages[family] = made
        return made
    }
}
