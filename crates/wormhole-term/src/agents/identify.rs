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
    use super::{AgentId, identify};

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
