// An action of the keymap by id (`keymap::ActionId`): `newTerminal`, `terminal.3`. A string, because the table
// is data (`keymap.json`, written from core's own); the few ids the code itself names are listed here.

import Foundation

public struct ActionID: Hashable, Sendable, Codable, RawRepresentable, ExpressibleByStringLiteral,
    CustomStringConvertible
{
    public let rawValue: String
    public init(rawValue: String) { self.rawValue = rawValue }
    public init(stringLiteral value: String) { rawValue = value }
    public var description: String { rawValue }

    public init(from decoder: any Decoder) throws {
        rawValue = try decoder.singleValueContainer().decode(String.self)
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }

    /// The sidebar's terminal with this number, 1–9.
    public static func terminal(_ number: Int) -> ActionID { ActionID(rawValue: "terminal.\(number)") }
    public static func group(_ number: Int) -> ActionID { ActionID(rawValue: "group.\(number)") }

    public static let closeTerminal: ActionID = "closeTerminal"
    public static let goToWaiting: ActionID = "goToWaiting"
    public static let undo: ActionID = "undo"
    public static let redo: ActionID = "redo"
    public static let newProfile: ActionID = "newProfile"
    public static let manageProfiles: ActionID = "manageProfiles"
    public static let newTerminal: ActionID = "newTerminal"
    public static let newWindow: ActionID = "newWindow"
    public static let openFolder: ActionID = "openFolder"
    public static let settings: ActionID = "settings"
    public static let checkForUpdates: ActionID = "checkForUpdates"
    public static let quit: ActionID = "quit"
}
