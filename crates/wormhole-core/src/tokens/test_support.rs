//! Helpers shared by the token tests.

use std::collections::BTreeMap;

use super::expr::Expr;
use super::{Color, ThemeFile, ThemeTokens, builtin_tokens, parse_theme_file, resolve};

pub(super) fn hex(s: &str) -> Color {
    Color::from_hex(s).unwrap()
}

pub(super) fn builtin(name: &str) -> ThemeTokens {
    builtin_tokens().into_iter().find(|t| t.name == name).unwrap()
}

pub(super) fn no_lookup(_: &str) -> Option<ThemeFile> {
    None
}

pub(super) fn resolve_str(src: &str) -> Result<ThemeTokens, String> {
    resolve(&parse_theme_file(src)?, &no_lookup)
}

/// Every color of `t` as hex, for comparisons that ignore sub-8-bit noise.
pub(super) fn snapshot(t: &ThemeTokens) -> Vec<(String, String)> {
    let mut v: Vec<_> = t.terminal.named().into_iter().map(|(n, c)| (n, c.to_hex())).collect();
    v.extend(t.ui_tokens().map(|(n, c)| (n.to_string(), c.to_hex())));
    v.extend(
        t.syntax.iter().map(|(k, s)| (k.clone(), format!("{} {} {}", s.color.to_hex(), s.bold, s.italic))),
    );
    v.push(("appearance".into(), format!("{:?}", t.appearance)));
    v.push(("metrics".into(), format!("{:?}", t.metrics)));
    v
}

pub(super) fn ratio(t: &ThemeTokens, fg: &str, bg: &str) -> f32 {
    let bg = t.opaque(bg, "surface.window").unwrap();
    Color::contrast_ratio(t.get(fg).unwrap().over(bg), bg)
}

/// Runs `f` on a thread with a 256 KiB stack, far less than any thread the app
/// resolves themes on, so unbounded recursion shows up as a crash in tests.
pub(super) fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(256 << 10).spawn(f).unwrap().join().unwrap()
}

/// Rule nodes on the longest reference chain starting at `name`.
pub(super) fn chain_len(
    name: &str,
    rules: &BTreeMap<String, Expr>,
    memo: &mut BTreeMap<String, usize>,
) -> usize {
    if let Some(n) = memo.get(name) {
        return *n;
    }
    let Some(e) = rules.get(name) else { return 0 };
    let mut deps = Vec::new();
    e.refs(&mut deps);
    let n = 1 + deps.iter().map(|d| chain_len(d, rules, memo)).max().unwrap_or(0);
    memo.insert(name.to_string(), n);
    n
}
