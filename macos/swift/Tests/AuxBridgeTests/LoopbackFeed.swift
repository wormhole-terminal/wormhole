// A feed on 127.0.0.1 for the updater's tests: the only address core's update client accepts over plain HTTP, and
// the only one these tests ever talk to.

import Foundation
import Network

final class LoopbackFeed: @unchecked Sendable {
    private let listener: NWListener
    private let routes: [String: Data]
    private let queue = DispatchQueue(label: "loopback-feed")

    init(routes: [String: String]) throws {
        let parameters = NWParameters.tcp
        parameters.requiredInterfaceType = .loopback
        listener = try NWListener(using: parameters)
        self.routes = routes.mapValues { Data($0.utf8) }
    }

    /// Starts listening; the base URL to give the updater.
    func start() async throws -> String {
        let routes = routes
        listener.newConnectionHandler = { connection in
            connection.start(queue: DispatchQueue.global())
            connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) { data, _, _, _ in
                let request = data.flatMap { String(data: $0, encoding: .utf8) } ?? ""
                let path = request.split(separator: " ").dropFirst().first.map(String.init) ?? ""
                let body = routes[path]
                let head = "HTTP/1.1 \(body == nil ? "404 Not Found" : "200 OK")\r\nContent-Length: \(body?.count ?? 0)\r\nConnection: close\r\n\r\n"
                connection.send(content: Data(head.utf8) + (body ?? Data()), completion: .contentProcessed { _ in connection.cancel() })
            }
        }
        return try await withCheckedThrowingContinuation { continuation in
            listener.stateUpdateHandler = { [listener] state in
                switch state {
                case .ready:
                    listener.stateUpdateHandler = nil
                    continuation.resume(returning: "http://127.0.0.1:\(listener.port?.rawValue ?? 0)")
                case .failed(let error): continuation.resume(throwing: error)
                default: break
                }
            }
            listener.start(queue: queue)
        }
    }

    func stop() { listener.cancel() }
}
