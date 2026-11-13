// A way out of a pair that falls short, written as an expression the person applies or not: the Studio never
// changes a value by itself. Core proposes it (`contrast_issues`: `readable(<what it is now>, <backgrounds>, <min>)`
// over every background the text falls short on, following tokens that merely name a failing one); this holds what
// it said. Without the core a pair is still listed, with no fix.

public struct ContrastFix: Sendable, Equatable {
    public let token: String
    public let expression: String
    public let label: String

    public init(token: String, expression: String, label: String) { (self.token, self.expression, self.label) = (token, expression, label) }
}

public struct ContrastIssue: Sendable, Equatable {
    public let check: ContrastCheck
    public let fix: ContrastFix?
    /// The failing token this pair's text follows: that token's fix clears this pair too.
    public let follows: String?

    public init(check: ContrastCheck, fix: ContrastFix? = nil, follows: String? = nil) { (self.check, self.fix, self.follows) = (check, fix, follows) }
}
