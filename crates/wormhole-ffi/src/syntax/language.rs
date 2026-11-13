//! Which grammar a path names, and the queries its captures come from.
//!
//! The choice is GPUI Kit's as the GPUI app builds it (`Language::from_str` on the extension, with the app's sixteen
//! grammars and JSON): the extension as written, so `Makefile`, `a.h` and `a.RS` have none. The queries are each
//! grammar crate's own. GPUI Kit carries queries of its own for Rust, Go, JavaScript, TypeScript, HTML, JSON and
//! Markdown and reads languages inside others (a fenced block, a `<script>`); those are not here, so these seven
//! colour by their grammar's query and an embedded language stays plain. It has no query for Swift; this has.

use tree_sitter::Language;

/// A grammar and the sources of its highlight query, joined in order.
pub(super) struct Grammar {
    /// Names the compiled query in the cache.
    pub(super) name: &'static str,
    pub(super) language: Language,
    pub(super) highlights: Vec<&'static str>,
}

/// The grammar the name of `path` says it is, if there is one.
pub(super) fn of(path: &str) -> Option<Grammar> {
    let extension = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let javascript = tree_sitter_javascript::HIGHLIGHT_QUERY;
    let (name, language, highlights): (_, Language, _) = match extension {
        "json" | "jsonc" => {
            ("json", tree_sitter_json::LANGUAGE.into(), vec![tree_sitter_json::HIGHLIGHTS_QUERY])
        }
        "bash" | "sh" => ("bash", tree_sitter_bash::LANGUAGE.into(), vec![tree_sitter_bash::HIGHLIGHT_QUERY]),
        "c" => ("c", tree_sitter_c::LANGUAGE.into(), vec![tree_sitter_c::HIGHLIGHT_QUERY]),
        "cpp" | "c++" => ("cpp", tree_sitter_cpp::LANGUAGE.into(), vec![tree_sitter_cpp::HIGHLIGHT_QUERY]),
        "css" | "scss" => ("css", tree_sitter_css::LANGUAGE.into(), vec![tree_sitter_css::HIGHLIGHTS_QUERY]),
        "go" => ("go", tree_sitter_go::LANGUAGE.into(), vec![tree_sitter_go::HIGHLIGHTS_QUERY]),
        "html" => ("html", tree_sitter_html::LANGUAGE.into(), vec![tree_sitter_html::HIGHLIGHTS_QUERY]),
        "java" => ("java", tree_sitter_java::LANGUAGE.into(), vec![tree_sitter_java::HIGHLIGHTS_QUERY]),
        "javascript" | "js" => (
            "javascript",
            tree_sitter_javascript::LANGUAGE.into(),
            vec![javascript, tree_sitter_javascript::JSX_HIGHLIGHT_QUERY],
        ),
        "markdown" | "md" | "mdx" => {
            ("markdown", tree_sitter_md::LANGUAGE.into(), vec![tree_sitter_md::HIGHLIGHT_QUERY_BLOCK])
        }
        "python" | "py" => {
            ("python", tree_sitter_python::LANGUAGE.into(), vec![tree_sitter_python::HIGHLIGHTS_QUERY])
        }
        "rust" | "rs" => {
            ("rust", tree_sitter_rust::LANGUAGE.into(), vec![tree_sitter_rust::HIGHLIGHTS_QUERY])
        }
        "swift" => ("swift", tree_sitter_swift::LANGUAGE.into(), vec![tree_sitter_swift::HIGHLIGHTS_QUERY]),
        "toml" => ("toml", tree_sitter_toml_ng::LANGUAGE.into(), vec![tree_sitter_toml_ng::HIGHLIGHTS_QUERY]),
        // TypeScript's query only adds to JavaScript's, which its grammar extends. Should the two not compile as one,
        // the last source alone is used (`engine::compile`).
        "tsx" => (
            "tsx",
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            vec![
                javascript,
                tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
            ],
        ),
        "typescript" | "ts" => (
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            vec![javascript, tree_sitter_typescript::HIGHLIGHTS_QUERY],
        ),
        "yaml" | "yml" => {
            ("yaml", tree_sitter_yaml::LANGUAGE.into(), vec![tree_sitter_yaml::HIGHLIGHTS_QUERY])
        }
        _ => return None,
    };
    Some(Grammar { name, language, highlights })
}
