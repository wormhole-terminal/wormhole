import Foundation
import Synchronization

/// An update feed on this machine for one scenario: a listener on `127.0.0.1` at a port the system picks, which answers a GET
/// with the file of that path inside one folder of the scenario's scratch (`<base>/stable/latest.json`), and 404 otherwise.
/// The core lets a feed be plain HTTP only on loopback, so a check against it never leaves the Mac. It keeps what was asked
/// (the path and the request's headers) for the scenario to read.
final class FeedServer: Sendable {
    struct Request: Sendable {
        let path: String
        /// The header names in lower case.
        let headers: [String: String]
    }

    private struct State {
        var folder: URL?
        var requests: [Request] = []
    }

    private let state = Mutex(State())
    private let source: any DispatchSourceRead
    let port: UInt16

    /// `--update-feed`'s value: the feeds are `<base>/<channel>/latest.json`.
    var base: String { "http://127.0.0.1:\(port)" }

    init() throws {
        let listener = socket(AF_INET, SOCK_STREAM, 0)
        guard listener >= 0 else { throw ScratchError(description: "no socket for the feed") }
        var address = sockaddr_in()
        address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
        address.sin_family = sa_family_t(AF_INET)
        address.sin_addr = in_addr(s_addr: inet_addr("127.0.0.1"))
        var length = socklen_t(MemoryLayout<sockaddr_in>.size)
        let bound = withUnsafeMutablePointer(to: &address) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { bind(listener, $0, length) == 0 && listen(listener, 8) == 0 && getsockname(listener, $0, &length) == 0 }
        }
        guard bound else {
            close(listener)
            throw ScratchError(description: "the feed cannot listen on 127.0.0.1")
        }
        port = UInt16(bigEndian: address.sin_port)
        source = DispatchSource.makeReadSource(fileDescriptor: listener, queue: DispatchQueue(label: "feed"))
        source.setEventHandler { [weak self] in
            let connection = accept(listener, nil, nil)
            guard connection >= 0 else { return }
            self?.answer(connection)
            close(connection)
        }
        source.setCancelHandler { close(listener) }
        source.resume()
    }

    deinit { source.cancel() }

    /// The folder whose files are served from now on.
    func serve(_ folder: URL) { state.withLock { $0.folder = folder } }

    /// Every request so far, oldest first.
    var requests: [Request] { state.withLock { $0.requests } }

    private func answer(_ connection: Int32) {
        // A request that does not arrive within two seconds is dropped: the listener never waits on a silent client.
        var patience = timeval(tv_sec: 2, tv_usec: 0)
        setsockopt(connection, SOL_SOCKET, SO_RCVTIMEO, &patience, socklen_t(MemoryLayout<timeval>.size))
        var received = Data()
        var buffer = [UInt8](repeating: 0, count: 4096)
        while received.range(of: Data("\r\n\r\n".utf8)) == nil, received.count < 65536 {
            let count = read(connection, &buffer, buffer.count)
            guard count > 0 else { break }
            received.append(contentsOf: buffer[..<count])
        }
        let lines = String(decoding: received, as: UTF8.self).components(separatedBy: "\r\n")
        let first = lines.first?.split(separator: " ").map(String.init) ?? []
        guard first.count >= 2, first[0] == "GET" else { return send(connection, status: "400 Bad Request", body: Data()) }
        var headers: [String: String] = [:]
        for line in lines.dropFirst() {
            guard let colon = line.firstIndex(of: ":") else { continue }
            headers[line[..<colon].lowercased()] = line[line.index(after: colon)...].trimmingCharacters(in: .whitespaces)
        }
        let path = String(first[1].drop { $0 == "/" })
        let folder = state.withLock { state -> URL? in
            state.requests.append(Request(path: path, headers: headers))
            return state.folder
        }
        // Only a plain name under the folder: nothing a path could climb out with.
        guard let folder, !path.contains(".."), let body = try? Data(contentsOf: folder.appendingPathComponent(path)) else {
            return send(connection, status: "404 Not Found", body: Data())
        }
        send(connection, status: "200 OK", body: body)
    }

    private func send(_ connection: Int32, status: String, body: Data) {
        var reply = Data("HTTP/1.1 \(status)\r\nContent-Type: application/json\r\nContent-Length: \(body.count)\r\nConnection: close\r\n\r\n".utf8)
        reply.append(body)
        reply.withUnsafeBytes { bytes in
            guard let start = bytes.baseAddress else { return }
            var sent = 0
            while sent < bytes.count {
                let count = write(connection, start + sent, bytes.count - sent)
                guard count > 0 else { return }
                sent += count
            }
        }
    }
}
