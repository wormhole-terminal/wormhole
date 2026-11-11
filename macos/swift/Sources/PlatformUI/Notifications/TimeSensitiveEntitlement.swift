// Whether this process was signed with the Time Sensitive entitlement (`entitled_to_time_sensitive`): only a
// Developer ID build with the provisioning profile has it; everything else delivers "needs input" at Active.

import Foundation
import Security

enum TimeSensitiveEntitlement {
    static let name = "com.apple.developer.usernotifications.time-sensitive"

    static let isGranted: Bool = {
        guard let task = SecTaskCreateFromSelf(nil) else { return false }
        return SecTaskCopyValueForEntitlement(task, name as CFString, nil) as? Bool == true
    }()
}
