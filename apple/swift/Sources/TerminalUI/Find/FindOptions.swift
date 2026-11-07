// The find bar's three toggles (`wormhole_term::find::FindOptions`).

import Foundation

public enum FindOption: CaseIterable, Sendable {
    case matchCase, wholeWord, regex
}

public struct FindOptions: Equatable, Sendable {
    public var matchCase = false
    public var wholeWord = false
    public var regex = false

    public init(matchCase: Bool = false, wholeWord: Bool = false, regex: Bool = false) {
        (self.matchCase, self.wholeWord, self.regex) = (matchCase, wholeWord, regex)
    }

    public subscript(option: FindOption) -> Bool {
        get {
            switch option {
            case .matchCase: matchCase
            case .wholeWord: wholeWord
            case .regex: regex
            }
        }
        set {
            switch option {
            case .matchCase: matchCase = newValue
            case .wholeWord: wholeWord = newValue
            case .regex: regex = newValue
            }
        }
    }
}
