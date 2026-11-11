// Touch ID, an unlocked paired Apple Watch, or the login password, through LocalAuthentication's owner policy
// (`platform/macos/auth.rs`). The reason string is what the system dialog shows after the app's name. No
// Info.plist key and no entitlement is needed on macOS. Tests and test instances use `ScriptedAuthenticator`:
// they must never raise the prompt.

import Foundation
import LocalAuthentication
import PlatformModel

public struct LocalAuthenticator: Authenticating {
    public init() {}

    public func canAuthenticate() -> Bool { Self.can(.deviceOwnerAuthentication) }
    public func biometricsAvailable() -> Bool { Self.can(.deviceOwnerAuthenticationWithBiometrics) }

    public func authenticate(reason: String) async throws {
        let context = LAContext()
        var error: NSError?
        guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error) else {
            throw error.map(Self.map) ?? .unavailable("no owner authentication")  // text: a diagnostic of a refused prompt, not shown as a sentence
        }
        do {
            _ = try await context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason)
        } catch {
            throw Self.map(error as NSError)
        }
    }

    private static func can(_ policy: LAPolicy) -> Bool {
        var error: NSError?
        return LAContext().canEvaluatePolicy(policy, error: &error)
    }

    private static func map(_ error: NSError) -> AuthError {
        AuthErrorMapping.map(domain: error.domain, code: error.code, message: error.localizedDescription)
    }
}
