// The app: the delegate that opens the workspace window and holds the parts around it. The window's state is
// the hub's (`WindowHub`); the menus, the ambient surfaces, the secondary windows and the control folder are
// each their own file and reach the window only through it.

import AppKit
import PlatformModel
import SettingsModel
import PlatformUI
import SettingsUI
import TerminalMetal
import WindowUI
import WorkspaceModel

#if WITH_UNIFFI
    import WorkspaceBridge
#endif

@MainActor
final class AppController: NSObject, NSApplicationDelegate {
    let options: Options
    let policy: PlatformPolicy
    let dataDirectory: URL
    /// How the window itself looks: its translucency, glass, status bar. The theme is `ThemeStore.shared`.
    let look = WindowLook()
    let factory: TerminalFactory
    let hub: WindowHub
    var devctl: DevCtl?
    var windowController: WorkspaceWindowController?
    var settingsController: SettingsWindowController?
    var settingsApplier: SettingsApplier?
    var notesCoordinator: NotesCoordinator?
    var notices: NoticeFeed?
    /// What the project's actions ask of the window (real windows only), and where the Notes tab keeps its text.
    let panelEffects: AnyObject?
    let panelNotes: (any NotesPersisting)?
    let panelUsage: AnyObject?
    var hooks: HookHost?
    var ambient: AmbientController?
    var menus: MenuController?
    /// Native context menus (the terminal's), whose choices go to the same router as the menu bar's.
    private(set) lazy var contextMenus = ContextMenuPresenter(dispatcher: MenuDispatcher { [weak self] in self?.router.dispatch($0) })
    /// Quitting asks first while something runs, except when a control command or the idle check quits.
    var confirmsQuit = true
    var endsAllOnQuit = false
    let activityClock = ActivityClock()
    /// The fonts and colours every terminal is drawn with: Settings' and the theme's.
    private(set) lazy var terminalLook = TerminalLook(terminals: hub.terminals)
    /// The settings the window follows, whether or not the Settings window is open: the core's when real.
    private(set) lazy var settingsModel = AppSettings.makeModel(app: self)
    private(set) lazy var studio = StudioLauncher(dataDirectory: dataDirectory)
    private(set) lazy var folderPicking = FolderPicking(app: self)
    private(set) lazy var router = ActionRouter(app: self)
    private(set) lazy var aux = AuxController(app: self, dataDirectory: dataDirectory)

    init(options: Options, dataDirectory: URL) {
        self.options = options
        self.dataDirectory = dataDirectory
        policy = PlatformPolicy(background: options.background)
        PlatformSetup.install(options)
        // The core's workspace when a real program is asked for (it also starts the terminals' programs), else the sample.
        let host = WorkspaceHost.make(options)
        let factory = TerminalFactory(options: options, source: host.source)
        self.factory = factory
        let effects = Self.makeEffects()
        let panel = PanelBackendsFactory.make(options, store: host.store, policy: policy, effects: effects)
        hub = WindowHub(store: host.store, terminals: TerminalViews { factory.make($0) }, panelBackends: panel.backends)
        (panelEffects, panelNotes, panelUsage) = (effects, panel.notes, panel.usage)
        #if WITH_UNIFFI
            // The title row's order of giving way is the core's, wherever the core runs the window.
            if options.program != nil { hub.rowFitting = CoreRowFitting() }
        #endif
        super.init()
        factory.look = terminalLook
        #if WITH_UNIFFI
            (effects as? PanelEffects)?.hub = hub
        #endif
    }

    private static func makeEffects() -> AnyObject? {
        #if WITH_UNIFFI
            PanelEffects()
        #else
            nil
        #endif
    }

    /// The notification rules' view of Settings.
    var settingsNotice: NoticeSettings { NoticeSettingsMaker.settings(settingsModel.backend.values) }

    var window: NSWindow? { windowController?.window }
    /// The selected terminal's view, if it exists: what the control commands act on.
    var selectedView: MetalTerminalView? { hub.state.selected.flatMap { hub.terminals.existing($0) } }

    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            try open()
        } catch {
            FileHandle.standardError.write(Data("cannot start: \(error)\n".utf8))
            exit(1)
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func applicationWillTerminate(_ notification: Notification) {
        hooks?.stop()
        hub.terminals.shutdown()
    }
    func applicationDockMenu(_ sender: NSApplication) -> NSMenu? { ambient?.dockMenu() }
}
