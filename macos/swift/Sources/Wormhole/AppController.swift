// The app: the delegate that opens the workspace window and holds the parts around it. The window's state is
// the hub's (`WindowHub`); the menus, the ambient surfaces, the secondary windows and the control folder are
// each their own file and reach the window only through it.

import AppKit
import DesignSystem
import AuxWindows
import PanelUI
import PlatformModel
import SettingsModel
#if WITH_UNIFFI
    import SettingsBridge
#endif
import PlatformUI
import SettingsUI
import TerminalMetal
import WindowUI
import WindowsModel
import WorkspaceModel

@MainActor
final class AppController: NSObject, NSApplicationDelegate {
    let options: Options
    let policy: PlatformPolicy
    let dataDirectory: URL
    /// How the window itself looks: its translucency, glass, status bar. The theme is `ThemeStore.shared`.
    let look = WindowLook()
    /// The windows of the profile Wormhole is in: what was saved of them and where each one's workspace comes from.
    var windowsHost: WindowsHost
    /// Every profile opened since launch, by id: the ones parked keep their windows (their terminals running, out of sight).
    var profileHosts: [String: WindowsHost] = [:]
    /// Every window of every profile, in the order they were made; the ones commands act on are `shown`.
    var sessions: [WindowSession] = []
    /// The window that was in front last: what commands act on while none is the key window (a test instance never has one).
    var lastFront: WindowSession? { didSet { if lastFront !== oldValue { updateFollower?.keepOffered() } } }
    /// Open folder… in a test instance, which shows no panel: what a chosen folder does, until `open-folder` gives one.
    var pendingFolder: ((String) -> Void)?
    /// Set while the app quits, so the windows that go with it are saved, not forgotten.
    var terminating = false
    var devctl: DevCtl?
    /// The window (1-based, in the order they were made) whose control file a line came from (`cmd2`): the window commands
    /// act on and the file `reply` writes to, for as long as that line runs. `nil` for `cmd`.
    var commandWindow: Int?
    /// The control folder's pointer: what it pressed, the tracking areas it is in, the menu it found.
    let injector = Injector()
    var settingsController: SettingsWindowController?
    var settingsApplier: SettingsApplier?
    var notices: NoticeFeed?
    /// The quick panel driven without its window by the control folder (`quick-open`).
    var quickDriver: QuickPanelModel?
    /// Layers the settings of the terminal in front's project over the person's (real windows only).
    var projectSettings: AnyObject?
    var hooks: HookHost?
    /// Files and folders macOS asked this app to open that are still waiting for the windows and the socket (`SystemOpen.swift`).
    var systemOpens: [URL] = []
    /// The updater's follower: the schedule, the chip that says an update is ready, Restart When Idle (a run with a feed only).
    var updateFollower: UpdateFollower?
    var ambient: AmbientController?
    /// The global hotkey: made for a test instance too, which registers nothing with the system.
    var hotkey: HotkeyController?
    var menus: MenuController?
    /// Native context menus (the terminal's), whose choices go to the same router as the menu bar's.
    private(set) lazy var contextMenus = ContextMenuPresenter(dispatcher: MenuDispatcher { [weak self] in self?.router.dispatch($0) })
    /// Quitting asks first while something runs, except when a control command or the idle check quits.
    var confirmsQuit = true
    var endsAllOnQuit = false
    /// The quit question's answer for the session hosts: end them with the app, or leave them running (the default).
    var endsHostsOnQuit = false
    let activityClock = ActivityClock()
    /// The chip that follows a terminal dragged out of its window, and the growth the next window opens with (`TearOff/`).
    let tearChip = TearOffChip()
    var growing: TearOffGrow.Start?
    #if WITH_UNIFFI
        /// Feature switches read in a window's own profile (`flagsOn(_:)`).
        let profileFlags = ProfileFlags()
    #endif
    /// The fonts and colours every terminal is drawn with: Settings' and the theme's.
    private(set) lazy var terminalLook = TerminalLook(terminals: { [weak self] in self?.ownSessions.map(\.hub.terminals) ?? [] })
    /// The looks of the other profiles with a window open, by profile (`profileLook(for:)`).
    var profileLooks: [String: ProfileLook] = [:]
    /// The settings the window follows, whether or not the Settings window is open: the core's when real.
    lazy var settingsModel = AppSettings.makeModel(app: self)
    private(set) lazy var studio = StudioLauncher(dataDirectory: dataDirectory, project: { [unowned self] in StudioProjects.source(app: self) })
    private(set) lazy var folderPicking = FolderPicking(app: self)
    private(set) lazy var router = ActionRouter(app: self)
    /// Where Go to Waiting (true) and Go to Finished (false) went last, so the next press goes on.
    var lastJumped: [Bool: Int64] = [:]
    /// The table's keys no menu item holds (`ExtraKeys`).
    private(set) lazy var extraKeys = ExtraKeys(app: self)
    private(set) lazy var recents = RecentProjects(dataDirectory: dataDirectory)
    /// SIGTERM as an event on the main queue (`quitOnSigterm`).
    var sigterm: DispatchSourceSignal?
    private(set) lazy var aux = AuxController(app: self, dataDirectory: dataDirectory)

    init(options: Options, dataDirectory: URL) throws {
        self.options = options
        self.dataDirectory = dataDirectory
        policy = PlatformPolicy(background: options.background)
        // A test instance never puts a question on the screen (it would bring a window forward): the control folder answers it,
        // so a `--background` instance without one has nobody to answer and shows its questions as any app does.
        if options.background, options.devctl != nil { PendingQuestions.current = PendingQuestions() }
        // And it never reads or replaces the person's clipboard: what it copies is a file in its data folder.
        if options.background {
            Pasteboard.divert(to: dataDirectory.appendingPathComponent("pasteboard.txt"))
            PanelPick.divert()
            Trash.divert(to: dataDirectory.appendingPathComponent("trash"))
            // A test instance cannot enter the system's full screen: a window keeps that state itself, so saving and restoring it still run.
            WindowFrames.simulatesFullScreen = true
        }
        // Nor does it launch the person's browser, Finder or editor: what it would launch is a file there too (`opened.txt`),
        // the core's launches (a file in an editor at a line) included.
        let opened = dataDirectory.appendingPathComponent("opened.txt")
        if options.background { ExternalOpen.divert(to: opened) }
        PlatformSetup.install(options, launchLog: options.background ? opened : nil)
        // Before anything opens a file in the data folder: a folder from before the rename is moved (never in an experiment folder).
        LegacyLaunch.moveDataFolder(dataDirectory)
        // Then the log files and the panic record, and the data folder's older files made owner-only.
        LaunchLogging.start()
        windowsHost = try WindowsHost.make(options)
        profileHosts[windowsHost.profile] = windowsHost
        super.init()
    }

    /// The notification rules' view of Settings.
    var settingsNotice: NoticeSettings { NoticeSettingsMaker.settings(settingsModel.backend.values) }

    /// The windows in sight: the profile Wormhole is in, and the odd ones opened in another (the rest are parked).
    var shown: [WindowSession] { sessions.filter { !$0.parked } }
    /// The window commands act on: the key one, else the one that was in front last, else the first.
    var front: WindowSession {
        let shown = shown
        if let n = commandWindow, shown.indices.contains(n - 1) { return shown[n - 1] }
        return shown.first { $0.window?.isKeyWindow == true } ?? shown.first { $0 === lastFront } ?? shown[0]
    }
    var hub: WindowHub { front.hub }
    /// What the right panel reads through: the project backend is also Settings' source of what a project sets.
    var panelBackends: PanelUI.PanelBackends { front.panelBackends }
    var factory: TerminalFactory { front.factory }
    var windowController: WorkspaceWindowController? { front.controller }
    var window: NSWindow? { front.window }
    /// The core's usage service, which the hook socket feeds: the first window's.
    var panelUsage: AnyObject? { sessions.first?.panelUsage }
    /// The selected terminal's view, if it exists: what the control commands act on.
    var selectedView: MetalTerminalView? { hub.state.selected.flatMap { hub.terminals.existing($0) } }

    func applicationDidFinishLaunching(_ notification: Notification) {
        quitOnSigterm()
        extraKeys.start()
        do {
            try open()
        } catch {
            FileHandle.standardError.write(Data("cannot start: \(error)\n".utf8))
            exit(1)
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func applicationWillTerminate(_ notification: Notification) {
        // Every window as it is, so the next launch brings them back; a window the person closed was forgotten already.
        terminating = true
        saveAllWindows()
        hooks?.stop()
        // Before the views go: a terminal let go of is not ended by its view closing.
        sessions.forEach { $0.hub.store.finishQuit(endHosts: endsHostsOnQuit) }
        sessions.forEach { $0.hub.terminals.shutdown() }
    }
    func applicationDockMenu(_ sender: NSApplication) -> NSMenu? { ambient?.dockMenu() }
}
