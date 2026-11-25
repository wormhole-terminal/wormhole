# Standard license texts

`scripts/gen-licenses.py --ffi` puts one of these into the Licenses list for a crate that declares exactly that
license (`Apache-2.0` or `MPL-2.0`) and whose package ships no license file. Both licenses have a fixed text with
nothing to fill in, so it is the crate's license text, not an invented one. Licenses that need the copyright
holder's own line (MIT, BSD, ISC) are never filled in this way.

Both files are copies of license files from crates already in the local cargo registry, with no change; no text was
typed or fetched.

- `Apache-2.0.txt`: `LICENSE-APACHE` of `quinn` 0.11.12, byte for byte the same file as in 30 crates. Its words
  (whitespace aside) equal the Apache-2.0 text of `kurbo` 0.13.1 and of two third-party notices in an installed
  application, one wrapping the lines differently. (The copy in `float-ord` writes `{}` for `[]` in the appendix and is
  not used.)
- `MPL-2.0.txt`: `LICENSE` of `cbindgen` 0.28.0, byte for byte the same file as in `dtoa-short` 0.3.5 and
  `nucleo-matcher` 0.3.1, and word for word the same as `cssparser`'s. The copies of `option-ext` and an installed
  application's notice differ only in the URL scheme of Exhibit A (`https` for `http`), which is a notice for
  people applying the license to their own files.
