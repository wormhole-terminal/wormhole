// A theme file read as what it is, a JSON tree, so editing one token keeps everything else the file says:
// `$schema`, comments' neighbours, keys this build doesn't know. Only what the Studio changes changes.

import Foundation

public enum JSONValue: Sendable, Equatable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public var string: String? { if case .string(let value) = self { value } else { nil } }
    public var number: Double? { if case .number(let value) = self { value } else { nil } }
    public var object: [String: JSONValue]? { if case .object(let value) = self { value } else { nil } }
    public var array: [JSONValue]? { if case .array(let value) = self { value } else { nil } }

    /// From what `JSONSerialization` makes of a (comment-free) document.
    init(foundation value: Any) {
        switch value {
        case let number as NSNumber where CFGetTypeID(number) == CFBooleanGetTypeID(): self = .bool(number.boolValue)
        case let number as NSNumber: self = .number(number.doubleValue)
        case let text as String: self = .string(text)
        case let items as [Any]: self = .array(items.map(JSONValue.init(foundation:)))
        case let fields as [String: Any]: self = .object(fields.mapValues(JSONValue.init(foundation:)))
        default: self = .null
        }
    }
}
