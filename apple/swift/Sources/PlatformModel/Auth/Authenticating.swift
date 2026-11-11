// Owner authentication (Touch ID, an Apple Watch, the login password) before a project's `.wormhole/` folder is
// trusted (`platform/auth.rs`). A protocol, so the window asks a stand-in in tests and test instances: a test
// must never raise the Touch ID prompt.

import Foundation

/// Why an authentication attempt did not succeed.
public enum AuthError: Error, Equatable, Sendable, CustomStringConvertible {
    /// Owner authentication cannot be performed here (no password set, no UI session, a test instance).
    case unavailable(String)
    /// The person, the system or the app dismissed the dialog.
    case cancelled
    /// The person did not prove ownership (wrong fingerprint or password, lockout).
    case failed(String)

    public var description: String {
        switch self {
        case .unavailable(let message): "authentication unavailable: \(message)"
        case .cancelled: "authentication cancelled"
        case .failed(let message): "authentication failed: \(message)"
        }
    }
}

public protocol Authenticating: Sendable {
    /// Whether any owner authentication is possible.
    func canAuthenticate() -> Bool
    /// Whether Touch ID specifically is set up, to word the UI ("Approve with Touch ID" or "…with your password").
    func biometricsAvailable() -> Bool
    /// Ask the person to authenticate; `reason` is what the system dialog shows after the app's name.
    func authenticate(reason: String) async throws
}

/// What `WORMHOLE_AUTH=allow|deny|cancel` does in a test instance: answers instead of showing the prompt.
public struct ScriptedAuthenticator: Authenticating {
    public enum Answer: String, Sendable { case allow, deny, cancel }
    public var answer: Answer
    /// What a Mac with Touch ID set up would say: it words the approve button.
    public var biometrics: Bool

    public init(answer: Answer, biometrics: Bool = false) { (self.answer, self.biometrics) = (answer, biometrics) }

    /// From WORMHOLE_AUTH; a test instance without it cancels, so nothing ever prompts. WORMHOLE_AUTH_BIOMETRICS=on says
    /// Touch ID is there (a wording only: the answer is still WORMHOLE_AUTH's).
    public init(environment: [String: String] = ProcessInfo.processInfo.environment) {
        self.init(
            answer: environment["WORMHOLE_AUTH"].flatMap(Answer.init(rawValue:)) ?? .cancel,
            biometrics: environment["WORMHOLE_AUTH_BIOMETRICS"] == "on")
    }

    public func canAuthenticate() -> Bool { true }
    public func biometricsAvailable() -> Bool { biometrics }

    public func authenticate(reason: String) async throws {
        switch answer {
        case .allow: return
        case .deny: throw AuthError.failed("denied by WORMHOLE_AUTH")
        case .cancel: throw AuthError.cancelled
        }
    }
}

/// `LAError` codes mapped to our errors (`platform::auth::map_error`), as plain integers so the mapping is testable.
public enum AuthErrorMapping {
    public static let domain = "com.apple.LocalAuthentication"

    public static func map(domain: String, code: Int, message: String) -> AuthError {
        guard domain == Self.domain else { return .failed("\(domain) (\(code)): \(message)") }
        switch code {
        case -2, -4, -9: return .cancelled
        // With the owner policy the password fallback is the system's, so these only surface when it could not finish.
        case -1, -3, -8: return .failed(message)
        case -5, -6, -7, -10, -11, -12, -13, -14, -1004: return .unavailable(message)
        default: return .failed("LAError \(code): \(message)")
        }
    }
}
