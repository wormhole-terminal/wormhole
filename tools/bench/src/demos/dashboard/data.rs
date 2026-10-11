//! What the dashboard shows: noise-driven metrics, the made-up processes and the log generator.

use crate::canvas::{Rgb, Rng};

pub const CORES: usize = 16;

pub fn hash(n: i64, seed: u32) -> f64 {
    let mut x = (n as u32 ^ seed.wrapping_mul(0x9E37_79B1)).wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0xC2B2_AE3D);
    x ^= x >> 13;
    x as f64 / u32::MAX as f64
}

/// Smooth value noise in 0..1, one lattice point per unit of `x`.
pub fn noise(x: f64, seed: u32) -> f64 {
    let i = x.floor();
    let f = x - i;
    let s = f * f * (3.0 - 2.0 * f);
    let (a, b) = (hash(i as i64, seed), hash(i as i64 + 1, seed));
    a + (b - a) * s
}

pub fn fbm(x: f64, seed: u32) -> f64 {
    0.65 * noise(x, seed) + 0.35 * noise(x * 2.3 + 17.0, seed.wrapping_add(99))
}

/// Green to yellow to red.
pub fn heat(p: f64) -> Rgb {
    let (g, y, r) = (Rgb(70, 214, 124), Rgb(244, 200, 72), Rgb(246, 88, 82));
    if p < 0.55 { g.lerp(y, p / 0.55) } else { y.lerp(r, (p - 0.55) / 0.45) }
}

/// How hard the machine is working (a build going), 0..1, shared by every metric.
pub fn busy(t: f64) -> f64 {
    ((noise(t * 0.09, 999) - 0.4) * 4.0).clamp(0.0, 1.0)
}

pub fn core_load(i: usize, t: f64) -> f64 {
    let base = 0.06 + 0.5 * fbm(t * 0.45 + i as f64 * 1.7, i as u32);
    (base + 0.45 * busy(t) * noise(t * 0.8 + i as f64 * 9.0, i as u32 + 50)).clamp(0.0, 1.0)
}

pub fn net(t: f64) -> (f64, f64) {
    let rx = (fbm(t * 0.6, 11).powi(3) * 3.0 + busy(t) * 0.25).clamp(0.0, 1.0);
    let tx = (fbm(t * 0.5 + 40.0, 12).powi(3) * 3.5).clamp(0.0, 1.0);
    (rx, tx)
}

pub fn mem(t: f64) -> f64 {
    (0.5 + 0.18 * fbm(t * 0.05, 21) + 0.1 * busy(t)).clamp(0.0, 1.0)
}

pub fn avg_cpu(t: f64) -> f64 {
    (0..CORES).map(|i| core_load(i, t)).sum::<f64>() / CORES as f64
}

pub struct Proc {
    pub pid: u32,
    pub user: &'static str,
    pub cmd: &'static str,
    pub weight: f64,
    pub mem: f64,
    pub seconds: f64,
    pub seed: u32,
}

impl Proc {
    pub fn cpu(&self, t: f64) -> f64 {
        let k = if self.weight > 0.5 { 0.25 + 1.4 * busy(t) } else { 1.0 };
        (self.weight * k * (0.15 + 1.7 * fbm(t * 0.3 + self.seed as f64, self.seed)) * 100.0).min(780.0)
    }
}

#[rustfmt::skip]
pub const PROCS: [(&str, &str, f64); 24] = [
    ("rustc --crate-name wormhole --edition=2024 src/main.rs", "dev", 0.9),
    ("cargo build --release -p wormhole", "dev", 0.12),
    ("claude --resume", "dev", 0.35),
    ("/Applications/Wormhole.app/Contents/MacOS/Wormhole", "dev", 0.3),
    ("node /opt/homebrew/bin/codex", "dev", 0.28),
    ("clang -c -O2 sqlite3.c", "dev", 0.6),
    ("ld64 -dynamic -arch arm64 -o target/release/deps", "dev", 0.5),
    ("kernel_task", "root", 0.18),
    ("/System/Library/PrivateFrameworks/SkyLight/WindowServer", "_windowserver", 0.22),
    ("mds_stores", "root", 0.14),
    ("zsh -il", "dev", 0.02),
    ("git status --porcelain=v2", "dev", 0.08),
    ("rust-analyzer", "dev", 0.4),
    ("ttystorm dashboard --seconds 60", "dev", 0.2),
    ("Finder", "dev", 0.05),
    ("com.apple.WebKit.WebContent", "dev", 0.16),
    ("ssh-agent -l", "dev", 0.01),
    ("docker-desktop-backend", "root", 0.15),
    ("syslogd", "root", 0.03),
    ("coreaudiod", "_coreaudiod", 0.07),
    ("tmux: server", "dev", 0.03),
    ("cargo clippy --workspace", "dev", 0.45),
    ("cargo test -p wormhole-core", "dev", 0.55),
    ("distnoted agent", "dev", 0.02),
];

pub const CRATES: [&str; 12] = [
    "serde",
    "tokio",
    "wormhole-core",
    "gpui",
    "regex",
    "anyhow",
    "tracing",
    "rusqlite",
    "ropey",
    "smallvec",
    "unicode-width",
    "parking_lot",
];
pub const TESTS: [&str; 8] = [
    "terminal::snapshot::wide_cells",
    "git::diff::renames",
    "keymap::table_has_no_clash",
    "sidebar::grouping::pins_first",
    "ipc::control::roundtrip",
    "store::migrations::matches_fresh",
    "tokens::rules::derives_all",
    "fsutil::links::stay_within",
];
pub const FILES: [&str; 8] = [
    "src/main.rs",
    "src/workspace/session.rs",
    "src/terminal_view/render/rows.rs",
    "core/src/git/diff.rs",
    "src/style/scale.rs",
    "core/src/agents/hooks.rs",
    "src/git_panel/changes.rs",
    "docs/cli.md",
];
pub const COMMITS: [&str; 6] = [
    "perf(release): build the release faster",
    "fix(sidebar): no stuck band flickering",
    "feat(bench): ttystorm demos",
    "docs(releasing): download host cost",
    "refactor(panes): one split per layout",
    "chore: version 0.1.4",
];
pub const SOURCES: [(&str, Rgb); 6] = [
    ("cargo", Rgb(240, 170, 80)),
    ("test", Rgb(110, 210, 140)),
    ("claude", Rgb(200, 144, 255)),
    ("git", Rgb(240, 132, 112)),
    ("clippy", Rgb(92, 192, 242)),
    ("agent", Rgb(242, 212, 100)),
];

pub struct Line {
    pub born: f64,
    pub src: usize,
    /// 0 info, 1 warning, 2 error.
    pub level: u8,
    pub msg: String,
}

pub fn log_line(rng: &mut Rng, born: f64) -> Line {
    let pick = |rng: &mut Rng, n: usize| rng.below(n);
    let (src, level, msg) = match rng.below(15) {
        0..=3 => {
            let c = CRATES[pick(rng, CRATES.len())];
            (0, 0, format!("Compiling {c} v{}.{}.{}", rng.below(2), rng.below(30), rng.below(20)))
        }
        4 => (
            0,
            0,
            format!("Finished `release` profile [optimized] in {}.{}s", 12 + rng.below(70), rng.below(10)),
        ),
        5 => (1, 0, format!("test result: ok. {} passed; 0 failed; 0 ignored", 380 + rng.below(80))),
        6 | 7 => (1, 0, format!("{} ... ok", TESTS[pick(rng, TESTS.len())])),
        8 => (2, 0, format!("editing {}", FILES[pick(rng, FILES.len())])),
        9 => (2, 0, "running cargo test -p wormhole-core".to_string()),
        10 => {
            (2, 0, format!("✓ {} files changed, +{} -{}", 1 + rng.below(9), rng.below(300), rng.below(120)))
        }
        11 => (
            3,
            0,
            format!("commit {:07x} {}", rng.next_u64() & 0xfff_ffff, COMMITS[pick(rng, COMMITS.len())]),
        ),
        12 => (4, 0, "no warnings".to_string()),
        13 => (5, 1, "waiting for approval: Bash(cargo fmt --all)".to_string()),
        _ => {
            if rng.f() < 0.3 {
                (0, 2, "error[E0308]: mismatched types in src/workspace/panes.rs".to_string())
            } else {
                (4, 1, format!("warning: unused variable `{}`", ["x", "cols", "row", "ctx"][pick(rng, 4)]))
            }
        }
    };
    Line { born, src, level, msg }
}
