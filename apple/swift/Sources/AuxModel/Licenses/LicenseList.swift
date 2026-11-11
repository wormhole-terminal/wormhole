// The open-source software Wormhole is built from: each crate's declared license and the license texts shipped
// in its package, plus what is compiled in without being a crate (the bundled font). The data is core's
// `assets/licenses.json` (`scripts/gen-licenses.py`); identical texts are stored once and shared by index.

import Foundation

public struct LicenseList: Decodable, Sendable {
    public struct Package: Decodable, Sendable, Identifiable, Equatable {
        public enum Kind: String, Decodable, Sendable { case crate, font }

        public var name: String
        public var version: String
        /// The SPDX expression from the crate's `Cargo.toml`.
        public var license: String?
        public var repository: String?
        let texts: [Int]
        public var kind: Kind = .crate
        public var id: String { name + " " + version }

        enum CodingKeys: String, CodingKey { case name, version, license, repository, texts, kind }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            name = try c.decode(String.self, forKey: .name)
            version = try c.decode(String.self, forKey: .version)
            license = try c.decodeIfPresent(String.self, forKey: .license)
            repository = try c.decodeIfPresent(String.self, forKey: .repository)
            texts = try c.decode([Int].self, forKey: .texts)
            kind = try c.decodeIfPresent(Kind.self, forKey: .kind) ?? .crate
        }

        init(name: String, version: String, license: String?, repository: String?, texts: [Int]) {
            (self.name, self.version, self.license, self.repository, self.texts) = (name, version, license, repository, texts)
        }
    }

    let texts: [String]
    public let packages: [Package]

    enum CodingKeys: String, CodingKey { case texts, packages = "crates" }

    public init(texts: [String], packages: [Package]) { (self.texts, self.packages) = (texts, packages) }

    /// Parses the list; a package that points at a text that does not exist is an error, as in core.
    public static func parse(_ data: Data) throws -> LicenseList {
        let list = try JSONDecoder().decode(LicenseList.self, from: data)
        for package in list.packages where package.texts.contains(where: { $0 >= list.texts.count }) {
            throw CocoaError(.coderReadCorrupt, userInfo: [NSLocalizedDescriptionKey: "\(package.id) points at a missing license text"]) // text: a parse error for the log; the window says licenses-unreadable
        }
        return list
    }

    /// The license files of `package`, in the order found. Empty when its package ships none.
    public func texts(of package: Package) -> [String] { package.texts.map { texts[$0] } }

    /// Indices into `packages` matching `query`: every word must appear in the name, version or license,
    /// ignoring case. An empty query matches all.
    public func search(_ query: String) -> [Int] {
        let words = query.split(whereSeparator: \.isWhitespace).map { $0.lowercased() }
        return packages.indices.filter { index in
            let package = packages[index]
            let haystack = "\(package.name) \(package.version) \(package.license ?? "")".lowercased() // text: what a search matches against
            return words.allSatisfy { haystack.contains($0) }
        }
    }
}

/// Where the list comes from: the file the app bundles, later the core through the FFI.
public protocol LicenseSource: Sendable {
    func load() throws -> LicenseList
}

public struct FileLicenseSource: LicenseSource {
    public let url: URL
    public init(url: URL) { self.url = url }
    public func load() throws -> LicenseList { try LicenseList.parse(Data(contentsOf: url)) }
}
