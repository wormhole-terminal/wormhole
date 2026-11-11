// One font picker's state: what is selected, what is searched, the list that follows, the row the pointer or the
// keys are on, and what is shown of it. Nothing here draws or asks the system by itself: the list is a function
// of the library's catalog (listed by core's rules) and the query, so it is the same under test and in the app.

import Foundation
import Observation

@MainActor @Observable
public final class FontPickerModel {
    public let slot: FontPickerSlot
    public let library: FontLibrary
    public var isOpen = false {
        didSet { if isOpen != oldValue { isOpen ? opened() : closed() } }
    }
    public var query = "" { didSet { if query != oldValue { rebuild(keeping: nil) } } }
    /// Terminal and code pickers list monospace fonts only until this is on.
    public var showsAll = false { didSet { if showsAll != oldValue { rebuild(keeping: nil) } } }
    public internal(set) var entries: [FontEntry] = []
    /// The row the keys or the pointer are on (an entry's id): Return picks it, the preview shows it.
    public internal(set) var highlighted: String?
    /// Set when the list should bring a row into view (keys, opening); the pointer moving never does.
    public internal(set) var scrollTarget: String?
    @ObservationIgnored var coverages: [String: FontCoverage] = [:]
    @ObservationIgnored let current: @MainActor () -> String
    @ObservationIgnored let inherited: @MainActor () -> String
    @ObservationIgnored let pick: @MainActor (String) -> Void

    /// `current` reads the slot's family (empty: it follows another), `inherited` the family an empty one comes to,
    /// `pick` writes the choice back. Closures, so the settings stay the one place the value lives.
    public init(
        slot: FontPickerSlot, library: FontLibrary, current: @escaping @MainActor () -> String,
        inherited: @escaping @MainActor () -> String, pick: @escaping @MainActor (String) -> Void
    ) {
        (self.slot, self.library, self.current, self.inherited, self.pick) = (slot, library, current, inherited, pick)
    }

    /// The family value now: empty means the slot follows another.
    public var selected: String { current() }
    public var isInherited: Bool { selected.trimmingCharacters(in: .whitespaces).isEmpty }
    public var monospaceOnly: Bool { slot.prefersMonospace && !showsAll }
    public var catalog: FontCatalog { library.catalog }

    /// The family the preview shows: the highlighted row's, else what is selected (resolved when it follows another).
    public var previewFamily: String {
        let family = highlightedFamily ?? selected
        return family.trimmingCharacters(in: .whitespaces).isEmpty ? inherited() : family
    }

    var highlightedFamily: String? { entries.first { $0.id == highlighted }?.family }

    public func isSelected(_ entry: FontEntry) -> Bool {
        switch entry {
        case .inherit: isInherited
        case .caption: false
        case .family(let choice, _): choice.name == selected
        }
    }

    /// Sets the family and closes; a family goes to the recents.
    public func choose(_ entry: FontEntry) {
        guard let family = entry.family else { return }
        if !family.isEmpty { library.recents.remember(family, for: slot.key) }
        pick(family)
        isOpen = false
    }

    private func opened() {
        query = ""
        showsAll = false
        library.ensureLoaded()
        rebuild(keeping: nil)
        Task { await library.refreshCustom(); rebuild(keeping: highlighted) }
    }

    private func closed() {
        highlighted = nil
        scrollTarget = nil
    }

    /// The catalog changed under an open picker (the monospace answer arrived): list again, keeping the row.
    public func catalogChanged() { if isOpen { rebuild(keeping: highlighted) } }
}
