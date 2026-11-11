// `LicenseSource` over core's bundled license list (`licenses()`): the same data the GPUI app's Licenses window
// reads (`assets/licenses.json`), handed over as the JSON `LicenseList` already parses, so the list keeps one
// reader and its check that every package points at a text that exists.

import AuxModel
import Foundation
import WormholeBindings

public struct FFILicenseSource: LicenseSource {
    public init() {}

    public func load() throws -> LicenseList {
        let record = licenses()
        let packages: [[String: Any]] = record.packages.map { package in
            var entry: [String: Any] = [
                "name": package.name, "version": package.version, "texts": package.texts.map(Int.init),
                "kind": package.isFont ? "font" : "crate",
            ]
            if let license = package.license { entry["license"] = license }
            if let repository = package.repository { entry["repository"] = repository }
            return entry
        }
        let data = try JSONSerialization.data(withJSONObject: ["texts": record.texts, "crates": packages])
        return try LicenseList.parse(data)
    }
}
