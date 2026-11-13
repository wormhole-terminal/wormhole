// One choice of a segmented control or a select: the value the file stores and the words shown for it.

public struct SettingOption: Sendable, Equatable, Identifiable, Decodable {
    public let value: JSONValue
    public let label: String
    /// An SF Symbol, for the few segmented controls that carry one (Dark / Light / Match system).
    public let icon: String?
    /// What the choice does, under its name in the menu (the Default shell's command line).
    public let detail: String?
    public var id: JSONValue { value }

    public init(value: JSONValue, label: String, icon: String? = nil, detail: String? = nil) {
        (self.value, self.label, self.icon, self.detail) = (value, label, icon, detail)
    }
}

/// Where a select's options come from when the platform lists them (installed apps, shells, icon themes).
public enum OptionSource: String, Sendable, Decodable {
    case openIn, shells, iconThemes
}

/// What a button row asks the host to do (`controls.rs` names them).
public enum SettingsAction: String, Sendable, Decodable {
    case openConfig, openData, openLogs, openFontsFolder, openIconThemesFolder, openDeveloperTools
    case makeDefaultTerminal, resetAll, resetTips
    /// A page's header buttons, which the schema has no row for.
    case rescanAgents, scanImports
    case openNotificationSettings, sendTestNotification, askNotifications, checkForUpdates
    /// The Command line tool row's buttons.
    case installCommandLine, uninstallCommandLine
}
