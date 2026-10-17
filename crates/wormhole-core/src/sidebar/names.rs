//! What a terminal is called: the title the program in it set (OSC 0/2: "Fix the login
//! bug", "vim main.rs"), else after what runs in it, the agent or the program in front ("claude",
//! "cargo watch") and else the shell ("zsh"). The sidebar, the title row's breadcrumb, a split's
//! pane headers and the menus all use the one name; a terminal a project named keeps that.

/// What runs in a terminal, for its name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NameFacts<'a> {
    /// The title the program set, as [`title_name`] cleaned it.
    pub title: Option<&'a str>,
    /// The command of the agent in it ("claude"), when there is one.
    pub agent: Option<&'a str>,
    /// The program in front as a row spells it ("cargo test -p core", `agents::command_label`),
    /// or the one that just finished while its result is still unseen.
    pub command: Option<&'a str>,
    /// The program the terminal started ("zsh").
    pub shell: &'a str,
}

/// The name of what runs: the title its program set, else the agent, else the program in front,
/// else the shell.
pub fn process_name(facts: &NameFacts) -> String {
    match facts.title.filter(|t| !t.is_empty()) {
        Some(title) => title.to_string(),
        None => program_name(facts),
    }
}

/// The program in front, without the title it set: the agent, else the running command, else the
/// shell. What the status bar's size item names ("claude · 90×40").
pub fn program_name(facts: &NameFacts) -> String {
    if let Some(agent) = facts.agent.filter(|a| !a.is_empty()) {
        return agent.to_string();
    }
    match facts.command.map(command_name).filter(|c| !c.is_empty()) {
        Some(command) => command,
        None => facts.shell.to_string(),
    }
}

/// Longest title a terminal keeps; the row and the title row cut it shorter still.
const TITLE_MAX_CHARS: usize = 120;

/// A title a program set (OSC 0/2) as a terminal shows it: on one line, without the spinner or
/// status mark some put in front ("⠐ Fix the bug", "✳ Claude Code"; the row's own glyph says that),
/// and cut to [`TITLE_MAX_CHARS`]. `None` when nothing is left, so the terminal falls back to what
/// runs in it. A spinner that only turns gives the same name every time, so it renames nothing.
pub fn title_name(raw: &str) -> Option<String> {
    // Paths, home and prompts keep their first character ("~/src", "/etc", "[ssh] host").
    const KEEP: &[char] = &['~', '/', '.', '(', '[', '{', '<', '"', '\'', '@', '#', '$', '_'];
    let start = raw.find(|c: char| c.is_alphanumeric() || KEEP.contains(&c))?;
    let mut name = String::new();
    for word in raw[start..].split(|c: char| c.is_whitespace() || c.is_control()).filter(|w| !w.is_empty()) {
        if !name.is_empty() {
            name.push(' ');
        }
        name.push_str(word);
    }
    if let Some((at, _)) = name.char_indices().nth(TITLE_MAX_CHARS) {
        name.truncate(at);
        name.push('…');
    }
    (!name.is_empty()).then_some(name)
}

/// The words of a command line that say what it is: the program and its subcommands, up to the
/// first option, path or value ("cargo test -p core" → "cargo test", "npm run dev" → "npm run
/// dev", "vim src/main.rs" → "vim"). The rest is for the row's second line, not a name.
pub fn command_name(command: &str) -> String {
    const MAX_WORDS: usize = 3;
    let mut words = command.split_whitespace();
    let Some(program) = words.next() else { return String::new() };
    let subcommand = |word: &&str| {
        word.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    };
    std::iter::once(program).chain(words.take_while(subcommand)).take(MAX_WORDS).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{NameFacts, command_name, process_name, program_name, title_name};

    fn shell() -> NameFacts<'static> {
        NameFacts { shell: "zsh", ..NameFacts::default() }
    }

    #[test]
    fn a_terminal_is_named_after_what_runs_in_it() {
        // The agent, the program in front, the shell.
        let agent = NameFacts { agent: Some("claude"), command: Some("claude --resume"), ..shell() };
        assert_eq!(process_name(&agent), "claude");
        let watch = NameFacts { command: Some("cargo watch -x test"), ..shell() };
        assert_eq!(process_name(&watch), "cargo watch");
        assert_eq!(process_name(&shell()), "zsh");
        // A command that names nothing falls back to the shell.
        assert_eq!(process_name(&NameFacts { command: Some("   "), ..shell() }), "zsh");
    }

    #[test]
    fn the_program_in_front_ignores_the_title_it_set() {
        let agent = NameFacts { title: Some("Fix the login bug"), agent: Some("claude"), ..shell() };
        assert_eq!(program_name(&agent), "claude");
        let vim = NameFacts { title: Some("main.rs - VIM"), command: Some("vim main.rs"), ..shell() };
        assert_eq!(program_name(&vim), "vim");
        assert_eq!(program_name(&NameFacts { title: Some("~"), ..shell() }), "zsh");
    }

    #[test]
    fn the_title_a_program_set_names_the_terminal() {
        let agent = NameFacts { title: Some("Fix the login bug"), agent: Some("claude"), ..shell() };
        assert_eq!(process_name(&agent), "Fix the login bug");
        let vim = NameFacts { title: Some("main.rs (~/src) - VIM"), command: Some("vim main.rs"), ..shell() };
        assert_eq!(process_name(&vim), "main.rs (~/src) - VIM");
        // An empty title names nothing: what runs does.
        assert_eq!(process_name(&NameFacts { title: Some(""), ..shell() }), "zsh");
    }

    #[test]
    fn a_title_loses_its_spinner_and_stays_on_one_line() {
        // A spinner that turns reads the same every time, so the terminal isn't renamed per frame.
        assert_eq!(title_name("⠂ Fix the bug").as_deref(), Some("Fix the bug"));
        assert_eq!(title_name("⠐ Fix the bug").as_deref(), Some("Fix the bug"));
        assert_eq!(title_name("✳ Claude Code").as_deref(), Some("Claude Code"));
        assert_eq!(title_name("* build").as_deref(), Some("build"));
        // Paths and prompts keep their first character.
        assert_eq!(title_name("~/Projects/terminal").as_deref(), Some("~/Projects/terminal"));
        assert_eq!(title_name("/etc").as_deref(), Some("/etc"));
        assert_eq!(title_name("[ssh] host").as_deref(), Some("[ssh] host"));
        assert_eq!(title_name("me@mac: ~/src").as_deref(), Some("me@mac: ~/src"));
        // One line, single spaces.
        assert_eq!(title_name("  two\n\tlines  ").as_deref(), Some("two lines"));
        assert_eq!(title_name(""), None);
        assert_eq!(title_name(" ⠂ "), None);
        let long = "x".repeat(500);
        assert_eq!(title_name(&long).map(|t| t.chars().count()), Some(121));
    }

    #[test]
    fn a_command_is_named_by_its_program_and_subcommands() {
        assert_eq!(command_name("cargo test -p wormhole-core"), "cargo test");
        assert_eq!(command_name("npm run dev"), "npm run dev");
        assert_eq!(command_name("docker compose up -d"), "docker compose up");
        assert_eq!(command_name("kubectl get pods --watch"), "kubectl get pods");
        assert_eq!(command_name("vim src/main.rs"), "vim");
        assert_eq!(command_name("tail -f build.log"), "tail");
        assert_eq!(command_name("ssh me@host"), "ssh");
        assert_eq!(command_name("sleep 100"), "sleep");
        assert_eq!(command_name("git log --oneline"), "git log");
        // A script run by its interpreter is named by the script (`command_label` did that).
        assert_eq!(command_name("manage.py runserver"), "manage.py runserver");
        assert_eq!(command_name("htop"), "htop");
        assert_eq!(command_name(""), "");
    }
}
