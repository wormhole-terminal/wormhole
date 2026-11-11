// The Licenses window's state: the search query, the packages that match it, and the selected one. The rule of the
// GPUI window: keep the open license while it is still listed; otherwise show the first match.

import Foundation
import Observation

@MainActor @Observable
public final class LicenseBrowser {
    public let list: LicenseList
    public var query = "" { didSet { refilter() } }
    public private(set) var matches: [Int]
    public var selected: Int?

    public init(list: LicenseList) {
        self.list = list
        matches = Array(list.packages.indices)
        selected = matches.first
    }

    public var selectedPackage: LicenseList.Package? { selected.map { list.packages[$0] } }
    public var texts: [String] { selectedPackage.map(list.texts(of:)) ?? [] }
    public var countLine: String { AuxText.licensesCount(shown: matches.count, total: list.packages.count) }

    private func refilter() {
        matches = list.search(query)
        if !(selected.map(matches.contains) ?? false) { selected = matches.first }
    }
}
