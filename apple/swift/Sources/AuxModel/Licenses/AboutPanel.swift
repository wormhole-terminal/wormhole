// What the standard About panel is given (`NSApp.orderFrontStandardAboutPanel(options:)`): the name, the version
// line, the copyright, as Terminal and Finder show them: icon, name, version, copyright, nothing to press.

import Foundation

extension AppInfo {
    /// The panel's option keys by their raw names, so the model needs no AppKit. The build number is hidden
    /// (empty) and the version carries the "(debug build)" note, so the panel reads "Version 1.6.3".
    public var aboutPanelOptions: [String: Any] {
        var options: [String: Any] = [
            "ApplicationName": name,
            "ApplicationVersion": version + (debugBuild ? " (debug build)" : ""),
            "Version": "",
        ]
        if !copyright.isEmpty { options["Copyright"] = copyright }
        return options
    }
}
