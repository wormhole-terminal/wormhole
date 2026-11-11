// What About shows: the name, version and copyright (`about::version_line`, `COPYRIGHT` of `packaging/app.env`).

import Foundation

public struct AppInfo: Equatable, Sendable {
    public var name: String
    public var version: String
    public var copyright: String
    public var debugBuild: Bool

    public init(name: String = AuxText.brand, version: String, copyright: String, debugBuild: Bool = false) {
        (self.name, self.version, self.copyright, self.debugBuild) = (name, version, copyright, debugBuild)
    }

    /// "Version 1.6.3", with "(debug build)" for a debug build.
    public var versionLine: String { AuxText.version(version) + (debugBuild ? " " + AuxText.debugBuild : "") }

    /// The running app's own facts from its bundle; the copyright is `NSHumanReadableCopyright`.
    public static func from(bundle: Bundle) -> AppInfo {
        let info = bundle.infoDictionary ?? [:]
        #if DEBUG
        let debug = true
        #else
        let debug = false
        #endif
        return AppInfo(
            name: info["CFBundleName"] as? String ?? AuxText.brand, // text: Info.plist key
            version: info["CFBundleShortVersionString"] as? String ?? "0", // text: Info.plist key
            copyright: info["NSHumanReadableCopyright"] as? String ?? "", debugBuild: debug) // text: Info.plist key
    }
}
