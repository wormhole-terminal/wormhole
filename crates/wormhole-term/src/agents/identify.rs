//! Which agent a process is, from its executable path and argv. Pure string
//! work, so it is tested without spawning anything.

use super::{AGENTS, AgentId};

/// Programs that run agent scripts; for these the script path decides.
fn is_interpreter(name: &str) -> bool {
    matches!(name, "node" | "bun" | "deno" | "ruby" | "sh" | "bash" | "zsh" | "dash")
        || name.to_ascii_lowercase().starts_with("python")
}

/// `codex-aarch64-apple-darwin` → `codex` (release binaries carry the target).
fn strip_target(name: &str) -> &str {
    for suffix in ["-aarch64-apple-darwin", "-x86_64-apple-darwin", "-darwin-arm64", "-darwin-x64"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            return stripped;
        }
    }
    name
}

/// File name without login-shell dash, script/`.exe` extension or target
/// suffix (npm installs name native binaries `bin/<name>.exe`, even on macOS).
fn base_name(path: &str) -> &str {
    let mut name = path.rsplit('/').next().unwrap_or(path);
    name = name.strip_prefix('-').unwrap_or(name);
    for ext in [".exe", ".js", ".mjs", ".cjs"] {
        name = name.strip_suffix(ext).unwrap_or(name);
    }
    strip_target(name)
}

fn by_executable(name: &str) -> Option<AgentId> {
    AGENTS.iter().find(|a| a.executables.contains(&name)).map(|a| a.id)
}

fn by_package(path: &str) -> Option<AgentId> {
    AGENTS.iter().find(|a| a.packages.iter().any(|p| path.contains(p))).map(|a| a.id)
}

/// Agents whose command does more than run the agent: only this subcommand, its first argument, is a
/// session someone works in. `fm` also answers one prompt for a script (`fm respond`), runs a server
/// (`fm serve`) and builds schemas; none of those is an agent in the terminal.
const SUBCOMMANDS: &[(AgentId, &str)] = &[(AgentId::Fm, "chat")];

/// How a program that may be an agent was started, for deciding whether it is one that the user can approve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// A native program, named by its executable (never by what it says its name is).
    Native(AgentId),
    /// A script on an interpreter, started plainly: `interpreter script args`, with nothing in between
    /// that runs or loads other code. `script` is the argument as given.
    Script { agent: AgentId, script: String },
}

/// Flags an interpreter may be started with and still be "that script": none of them runs code of its own,
/// loads any, or takes a value that could name some. Anything else (`-e`, `-p`, `-r`, `--require`, `--import`,
/// `--loader`, `-m`, `-c`, `--inspect…`, or any flag with a value) makes the process not an agent that can be
/// approved by its script: it is whatever the user's own program is, which is "Scripts".
const HARMLESS_FLAGS: &[&str] = &[
    "--no-warnings",
    "--no-deprecation",
    "--trace-warnings",
    "--trace-deprecation",
    "--enable-source-maps",
    "--no-experimental-fetch",
    "--no-network-family-autoselection",
    "-u",
    "-B",
    "-O",
    "-OO",
    "-s",
    "-E",
];

/// `--max-old-space-size=4096`: a number, which is all a memory flag may say.
fn harmless_flag(flag: &str) -> bool {
    HARMLESS_FLAGS.contains(&flag)
        || ["--max-old-space-size=", "--stack-size="].iter().any(|name| {
            flag.strip_prefix(name).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
}

/// Which agent `exe` is, and how it was started, for approving it (`agent-permissions.md`, 4.3).
/// Stricter than [`identify`], which names what a terminal runs for its sidebar: here only the executable's own
/// name counts, never `argv[0]` (a program can call itself anything), and a script counts only when its
/// interpreter was started plainly and `env_runs_code` (the environment holds `NODE_OPTIONS`, `PYTHONPATH` and the
/// like, which load code before the script) is false. `None` is not an agent the user can approve: it falls to
/// Scripts.
pub fn identify_launch(exe: &str, argv: &[String], env_runs_code: bool) -> Option<Launch> {
    let name = base_name(exe);
    if !is_interpreter(name) {
        let agent = by_executable(name).or_else(|| by_package(exe))?;
        return match SUBCOMMANDS.iter().find(|(id, _)| *id == agent) {
            Some((_, subcommand)) => {
                (argv.get(1).map(String::as_str) == Some(*subcommand)).then_some(Launch::Native(agent))
            }
            None => Some(Launch::Native(agent)),
        };
    }
    if env_runs_code {
        return None;
    }
    let mut args = argv.iter().skip(1);
    let script = loop {
        let arg = args.next()?;
        if arg.starts_with('-') {
            if harmless_flag(arg) {
                continue;
            }
            return None;
        }
        break arg.clone();
    };
    let agent = by_executable(base_name(&script)).or_else(|| by_package(&script))?;
    Some(Launch::Script { agent, script })
}

/// Variables that make an interpreter load code before the script it was given runs.
pub const CODE_LOADING_VARIABLES: &[&str] = &[
    "NODE_OPTIONS",
    "NODE_PATH",
    "NODE_EXTRA_CA_CERTS",
    "PYTHONSTARTUP",
    "PYTHONPATH",
    "PYTHONHOME",
    "RUBYOPT",
    "RUBYLIB",
    "PERL5OPT",
    "PERL5LIB",
    "BUN_OPTIONS",
    "DENO_OPTIONS",
];

/// The script an interpreter runs (`node --flag /path/to/cli.js …`: `/path/to/cli.js`), when the process
/// is an interpreter with one; `None` for a native program. Used by the sidebar's detection, which is
/// not an approval ([`identify_launch`] is).
pub fn interpreted_script(exe: &str, argv: &[String]) -> Option<String> {
    let program = argv.first().map(String::as_str).unwrap_or(exe);
    [base_name(exe), base_name(program)]
        .into_iter()
        .any(is_interpreter)
        .then(|| argv.iter().skip(1).find(|a| !a.starts_with('-')).cloned())
        .flatten()
}

/// Which agent a process is, from its executable path and argv.
pub fn identify(exe: &str, argv: &[String]) -> Option<AgentId> {
    let program = argv.first().map(String::as_str).unwrap_or(exe);
    for name in [base_name(exe), base_name(program)] {
        if is_interpreter(name) {
            // `node --flag /path/to/cli.js …`: the first non-flag argument is the script.
            let script = argv.iter().skip(1).find(|a| !a.starts_with('-'))?;
            return by_executable(base_name(script)).or_else(|| by_package(script));
        }
    }
    let agent = by_executable(base_name(exe))
        .or_else(|| by_executable(base_name(program)))
        .or_else(|| by_package(exe))?;
    match SUBCOMMANDS.iter().find(|(id, _)| *id == agent) {
        Some((_, subcommand)) => (argv.get(1).map(String::as_str) == Some(*subcommand)).then_some(agent),
        None => Some(agent),
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentId, Launch, identify, identify_launch};

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn native_binaries_match_by_name() {
        assert_eq!(identify("/opt/homebrew/bin/codex", &argv(&["codex"])), Some(AgentId::Codex));
        assert_eq!(
            identify("/Users/me/.local/bin/claude", &argv(&["claude", "--resume"])),
            Some(AgentId::Claude)
        );
        assert_eq!(identify("/usr/local/bin/goose", &argv(&["goose", "session"])), Some(AgentId::Goose));
        assert_eq!(
            identify(
                "/x/node_modules/@openai/codex/vendor/aarch64-apple-darwin/codex/codex-aarch64-apple-darwin",
                &argv(&["codex-aarch64-apple-darwin"])
            ),
            Some(AgentId::Codex)
        );
    }

    #[test]
    fn foundation_models_is_an_agent_only_in_its_interactive_mode() {
        assert_eq!(identify("/usr/bin/fm", &argv(&["fm", "chat"])), Some(AgentId::Fm));
        assert_eq!(identify("/usr/bin/fm", &argv(&["fm", "chat", "--resume", "work"])), Some(AgentId::Fm));
        for script in [&["fm", "respond", "hi"][..], &["fm", "serve"], &["fm"], &["fm", "--help"]] {
            assert_eq!(identify("/usr/bin/fm", &argv(script)), None, "{script:?}");
        }
    }

    #[test]
    fn scripts_match_by_script_name_or_package() {
        let node = "/opt/homebrew/bin/node";
        assert_eq!(identify(node, &argv(&["node", "/opt/homebrew/bin/gemini"])), Some(AgentId::Gemini));
        assert_eq!(
            identify(node, &argv(&["node", "--no-warnings", "/x/lib/node_modules/@github/copilot/index.js"])),
            Some(AgentId::Copilot)
        );
        assert_eq!(
            identify(node, &argv(&["node", "/Users/me/.local/share/cursor-agent/versions/1/index.js"])),
            Some(AgentId::Cursor)
        );
        let python =
            "/Library/Frameworks/Python.framework/Versions/3.12/Resources/Python.app/Contents/MacOS/Python";
        assert_eq!(
            identify(python, &argv(&["/usr/local/bin/python3.12", "/Users/me/.local/bin/aider"])),
            Some(AgentId::Aider)
        );
    }

    #[test]
    fn the_script_an_interpreter_runs_is_named_and_a_native_program_has_none() {
        assert_eq!(
            super::interpreted_script(
                "/opt/homebrew/bin/node",
                &argv(&["node", "--no-warnings", "/x/cli.js", "-p"])
            ),
            Some("/x/cli.js".to_string())
        );
        assert_eq!(super::interpreted_script("/usr/bin/python3", &argv(&["python3"])), None);
        assert_eq!(
            super::interpreted_script("/Users/me/.local/bin/claude", &argv(&["claude", "--resume"])),
            None
        );
    }

    fn launch(exe: &str, args: &[&str], env: bool) -> Option<Launch> {
        identify_launch(exe, &argv(args), env)
    }

    #[test]
    fn a_plain_script_on_an_interpreter_is_the_agent_and_a_native_binary_is_named_by_its_executable() {
        assert_eq!(
            launch("/opt/homebrew/bin/node", &["node", "--no-warnings", "/x/bin/gemini"], false),
            Some(Launch::Script { agent: AgentId::Gemini, script: "/x/bin/gemini".into() })
        );
        assert_eq!(
            launch(
                "/usr/bin/node",
                &["node", "--max-old-space-size=8192", "/x/@github/copilot/index.js"],
                false
            ),
            Some(Launch::Script { agent: AgentId::Copilot, script: "/x/@github/copilot/index.js".into() })
        );
        assert_eq!(
            launch("/Users/me/.local/bin/claude", &["claude"], false),
            Some(Launch::Native(AgentId::Claude))
        );
        assert_eq!(launch("/usr/bin/fm", &["fm", "chat"], false), Some(Launch::Native(AgentId::Fm)));
        assert_eq!(launch("/usr/bin/fm", &["fm", "serve"], false), None);
    }

    #[test]
    fn what_a_program_calls_itself_is_never_what_it_is() {
        // `exec -a node /tmp/evil /x/gemini`: the executable is not an interpreter, whatever argv[0] says.
        assert_eq!(launch("/tmp/evil", &["node", "/x/bin/gemini"], false), None);
        assert_eq!(launch("/tmp/evil", &["claude"], false), None);
    }

    #[test]
    fn a_flag_that_runs_or_loads_code_or_takes_a_value_makes_it_no_agent() {
        let node = "/usr/bin/node";
        for args in [
            &["node", "-e", "-x", "/x/bin/gemini"][..],
            &["node", "-p", "1", "/x/bin/gemini"],
            &["node", "--require=/tmp/evil.js", "/x/bin/gemini"],
            &["node", "-r", "/tmp/evil.js", "/x/bin/gemini"],
            &["node", "--import", "/tmp/evil.mjs", "/x/bin/gemini"],
            &["node", "--loader=/tmp/l.mjs", "/x/bin/gemini"],
            &["node", "--eval", "x", "/x/bin/gemini"],
            &["node", "--inspect=0", "/x/bin/gemini"],
            &["node", "-i", "/x/bin/gemini"],
            &["node", "--max-old-space-size=$(x)", "/x/bin/gemini"],
            &["node", "--unknown-flag", "/x/bin/gemini"],
        ] {
            assert_eq!(launch(node, args, false), None, "{args:?}");
        }
        assert_eq!(launch("/usr/bin/python3", &["python3", "-m", "aider"], false), None);
        assert_eq!(launch("/usr/bin/python3", &["python3", "-c", "x", "/x/aider"], false), None);
        assert!(launch("/usr/bin/python3", &["python3", "-u", "/x/.local/bin/aider"], false).is_some());
    }

    #[test]
    fn an_environment_that_loads_code_makes_a_script_no_agent_but_not_a_native_binary() {
        assert_eq!(launch("/usr/bin/node", &["node", "/x/bin/gemini"], true), None);
        assert!(launch("/usr/local/bin/codex", &["codex"], true).is_some());
    }

    #[test]
    fn unrelated_processes_do_not_match() {
        assert_eq!(identify("/bin/zsh", &argv(&["-zsh"])), None);
        assert_eq!(identify("/bin/zsh", &argv(&["/bin/zsh", "-l"])), None);
        assert_eq!(identify("/usr/bin/vim", &argv(&["vim", "codex"])), None);
        assert_eq!(identify("/opt/homebrew/bin/node", &argv(&["node", "server.js"])), None);
        assert_eq!(identify("/opt/homebrew/bin/node", &argv(&["node"])), None);
        // An argument that merely names an agent isn't the agent.
        assert_eq!(identify("/bin/cat", &argv(&["cat", "/tmp/claude"])), None);
    }

    #[test]
    fn install_layouts_seen_in_the_wild() {
        let node = "/opt/homebrew/bin/node";
        // npm installs that swap the shim for a native `bin/<name>.exe`.
        let claude_exe = "/x/lib/node_modules/@anthropic-ai/claude-code/bin/claude.exe";
        assert_eq!(identify(claude_exe, &argv(&["claude"])), Some(AgentId::Claude));
        let opencode_exe = "/x/lib/node_modules/opencode-ai/bin/opencode.exe";
        assert_eq!(identify(opencode_exe, &argv(&[opencode_exe])), Some(AgentId::OpenCode));
        // Pi's script under Bun, by the name it was started as or by its package (old and new scope).
        let pi = "/Users/me/.bun/bin/pi";
        assert_eq!(identify("/Users/me/.bun/bin/bun", &argv(&["bun", pi])), Some(AgentId::Pi));
        for scope in ["@earendil-works", "@mariozechner"] {
            let script = format!("/x/lib/node_modules/{scope}/pi-coding-agent/dist/cli.js");
            assert_eq!(identify("/usr/local/bin/node", &argv(&["node", &script])), Some(AgentId::Pi));
        }
        // Claude's native installer names the binary after its version.
        assert_eq!(
            identify("/Users/me/.local/share/claude/versions/2.1.3", &argv(&["claude"])),
            Some(AgentId::Claude)
        );
        // Node shims that spawn the native binary.
        assert_eq!(
            identify(node, &argv(&["node", "/x/lib/node_modules/@openai/codex/bin/codex.js"])),
            Some(AgentId::Codex)
        );
        assert_eq!(
            identify(node, &argv(&["node", "/x/lib/node_modules/@google/gemini-cli/bundle/gemini.js"])),
            Some(AgentId::Gemini)
        );
        assert_eq!(
            identify(node, &argv(&["node", "/x/lib/node_modules/@qwen-code/qwen-code/cli-entry.js"])),
            Some(AgentId::Qwen)
        );
        // Cursor's launcher: `exec -a "$0" node --use-system-ca index.js`.
        let cursor = "/Users/me/.local/share/cursor-agent/versions/2026.09.01/node";
        assert_eq!(
            identify(
                cursor,
                &argv(&[
                    "/Users/me/.local/bin/agent",
                    "--use-system-ca",
                    "/Users/me/.local/share/cursor-agent/versions/2026.09.01/index.js"
                ])
            ),
            Some(AgentId::Cursor)
        );
    }
}
