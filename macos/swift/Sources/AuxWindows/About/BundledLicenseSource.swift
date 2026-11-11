// The license list the app ships: `licenses.json` in the app bundle's resources (`scripts/gen-licenses.py` writes it
// from `Cargo.lock`), or the file `WORMHOLE_LICENSES_JSON` names while developing.

import AuxModel
import Foundation

public struct BundledLicenseSource: LicenseSource {
    public init() {}

    public func load() throws -> LicenseList {
        let override = ProcessInfo.processInfo.environment["WORMHOLE_LICENSES_JSON"].map(URL.init(fileURLWithPath:)) // text: an environment variable
        guard let url = override ?? Bundle.main.url(forResource: "licenses", withExtension: "json") else {
            throw CocoaError(.fileNoSuchFile)
        }
        return try FileLicenseSource(url: url).load()
    }
}
