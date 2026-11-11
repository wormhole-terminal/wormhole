// The whole Session Hosts window's data (`ipc::hosts::report::Report`): this app, the folder, the feature's switch,
// the host program found, and every host in the folder. Collected off the main thread when the hosts folder
// changes and on a slow look while the window is open; the window only draws it.

import Foundation

public struct HostsReport: Equatable, Sendable {
    public enum HostProgram: Equatable, Sendable {
        case found(path: String)
        /// The app runs from a disk image or a temporary copy: there is no host beside it.
        case disposableCopy
        case missing
    }

    public var appVersion: String
    public var protocolText: String
    public var folder: String
    public var flagOn: Bool
    public var program: HostProgram
    public var cards: [HostCard]

    public init(
        appVersion: String, protocolText: String, folder: String, flagOn: Bool, program: HostProgram, cards: [HostCard]
    ) {
        (self.appVersion, self.protocolText, self.folder, self.flagOn) = (appVersion, protocolText, folder, flagOn)
        (self.program, self.cards) = (program, cards)
    }

    public func count(_ standing: HostStanding) -> Int { cards.filter { $0.standing == standing }.count }

    /// Everything, for Copy all: this app and each host.
    public var text: String {
        let head = [
            AuxText.hostsHeaderApp(version: appVersion, protocol: protocolText), "\(AuxText.hostsFolder): \(folder)",
        ]
        return (head + cards.map(\.text)).joined(separator: "\n\n")
    }
}

/// What the window asks of the hosts folder and the host processes (core `ipc::hosts`: `report`, `end_host`,
/// `forget`). The window never touches a socket or a pid.
@MainActor
public protocol HostsBackend: AnyObject {
    func report() async -> HostsReport
    /// Fires when the hosts folder changed, so the window looks again; the window also looks on a slow timer.
    func changes() -> AsyncStream<Void>
    func end(_ id: String) async
    func forget(_ id: String) async
}
