// A short license list, for previews and the Theme Studio's gallery: three packages, two of them sharing one text, in
// the shape of core's `assets/licenses.json`.

/// A small license list for previews and specimens.
public enum SampleLicenses {
    /// Three packages over two license texts.
    public static let list = LicenseList(
        texts: [
            "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \u{201c}Software\u{201d}), to deal in the Software without restriction\u{2026}",
            "Apache License\nVersion 2.0, January 2004\nhttp://www.apache.org/licenses/",
        ],
        packages: [
            LicenseList.Package(name: "anyhow", version: "1.0.98", license: "MIT OR Apache-2.0", repository: "https://github.com/dtolnay/anyhow", texts: [0, 1]),
            LicenseList.Package(name: "bitflags", version: "2.9.1", license: "MIT OR Apache-2.0", repository: nil, texts: [0]),
            LicenseList.Package(name: "unicode-ident", version: "1.0.18", license: "(MIT OR Apache-2.0) AND Unicode-3.0", repository: nil, texts: [1]),
        ])
}
