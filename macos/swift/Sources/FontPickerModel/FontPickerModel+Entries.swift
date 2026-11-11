// The list's contents: the inherit row if its label matches, the recents (empty search only), then the groups
// core's rules gave, with captions once anything bundled or custom is listed (`FontMenu::entries`).

extension FontPickerModel {
    func rebuild(keeping keep: String?) {
        let catalog = library.catalog
        let choices = library.rules.choices(in: catalog, query: query, monospaceOnly: monospaceOnly)
        let trimmed = query.trimmingCharacters(in: .whitespaces).lowercased()
        var list: [FontEntry] = []
        if let label = slot.inheritLabel, trimmed.isEmpty || label.lowercased().contains(trimmed) { list.append(.inherit(label)) }

        let listed = Dictionary(choices.map { ($0.name, $0) }, uniquingKeysWith: { first, _ in first })
        let recent = trimmed.isEmpty ? library.recents.families(for: slot.key).compactMap { listed[$0] } : []
        if !recent.isEmpty {
            list.append(.caption(.recent))
            list += recent.map { .family($0, group: .recent) }
        }
        let captions = choices.contains { $0.source != .installed } || !recent.isEmpty
        var group: FontSource?
        for choice in choices {
            if captions, group != choice.source {
                group = choice.source
                list.append(.caption(.source(choice.source)))
            }
            list.append(.family(choice, group: nil))
        }
        entries = list
        let ids = Set(list.map(\.id))
        let start = keep.flatMap { ids.contains($0) ? $0 : nil } ?? list.first { isSelected($0) && $0.isSelectable }?.id
        highlighted = start ?? list.first { $0.isSelectable }?.id
        scrollTarget = highlighted
    }

    /// How many families the list holds (the inherit row and the recents' copies are not families of their own).
    public var familyCount: Int { entries.filter { if case .family(_, nil) = $0 { true } else { false } }.count }

    /// The line under the list: what is still loading, else the count (`FontMenu::new`'s status).
    public var status: String {
        let catalog = library.catalog
        if catalog.isLoading { return FontWords.loading }
        if monospaceOnly, catalog.isClassifying { return FontWords.findingMonospace }
        return FontWords.count(familyCount)
    }

    /// Shown instead of an empty list: why it is empty and, in the monospace list, what would widen it.
    public var emptyNote: String? {
        guard familyCount == 0, !library.catalog.isLoading else { return nil }
        return FontWords.noMatches(query.trimmingCharacters(in: .whitespaces))
    }
}
