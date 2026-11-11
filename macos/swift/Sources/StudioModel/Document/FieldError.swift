// What is wrong with the value typed for a token, which the field shows (and offers the closest name for) while
// the draft keeps its last good value (`studio/token_field.rs`).

public struct FieldError: Sendable, Equatable {
    public let token: String
    public let text: String
    public let message: String
}
