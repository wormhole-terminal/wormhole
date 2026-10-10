//! What the program in a plain terminal's foreground means for its row: whether the
//! clock beside the command says anything, and when output counts as "new".

/// Whether `command` (as `agents::command_label` spells it: "cargo test -p core", "npm run dev")
/// ends by itself, like a build, a test run or a script: then the time it has run says how far
/// along it is. Programs you work in or leave running are not like that, and their clock would only
/// count up for as long as they stay open: editors, pagers, remote shells, multiplexers,
/// monitors, a REPL without a script, a dev server or a watcher. Only what is listed here stays
/// open; a program we don't know counts as one that ends, so it keeps its clock.
pub fn ends_by_itself(command: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    !stays_open(&words)
}

/// Programs you sit in until you quit them, and programs that run until you stop them.
const STAYS_OPEN: &[&str] = &[
    // Editors.
    "vi",
    "vim",
    "nvim",
    "view",
    "vimdiff",
    "nano",
    "pico",
    "emacs",
    "hx",
    "helix",
    "micro",
    "kak",
    "joe",
    "ed",
    // Pagers and readers.
    "less",
    "more",
    "most",
    "man",
    "info",
    // Remote shells.
    "ssh",
    "mosh",
    "mosh-client",
    "telnet",
    "sftp",
    "ftp",
    // Multiplexers.
    "tmux",
    "screen",
    "zellij",
    "byobu",
    // Monitors and full-screen tools.
    "top",
    "htop",
    "btop",
    "btm",
    "glances",
    "atop",
    "nvtop",
    "iotop",
    "watch",
    "lazygit",
    "lazydocker",
    "tig",
    "k9s",
    "ranger",
    "yazi",
    "nnn",
    "mc",
    "ncdu",
    // Servers and watchers.
    "uvicorn",
    "gunicorn",
    "hypercorn",
    "daphne",
    "nodemon",
    "ts-node-dev",
    "live-server",
    "http-server",
    "browser-sync",
    "webpack-dev-server",
    "ngrok",
    "redis-server",
    "mongod",
    "watchexec",
    "entr",
    "bacon",
];

/// Language runtimes that open a prompt when they are given nothing to run (`python`, `node`,
/// `irb`). `python3.12` and `pypy3` are matched by [`is_python`].
const REPLS: &[&str] = &[
    "node", "nodejs", "deno", "ruby", "perl", "php", "lua", "luajit", "julia", "ghci", "irb", "pry",
    "ipython", "bpython", "ptpython", "iex", "erl", "clj", "scala", "sbcl", "racket", "guile", "ocaml",
    "utop", "tclsh", "r",
];

/// Database shells: a prompt, unless a command or a file is given with one of [`ONE_SHOT`].
const DB_SHELLS: &[&str] = &["sqlite3", "psql", "mysql", "mariadb", "mongosh", "duckdb"];
const ONE_SHOT: &[&str] = &["-c", "-e", "-f", "--command", "--execute", "--file", "--eval"];

/// Modules that `python -m` runs as a server.
const SERVER_MODULES: &[&str] =
    &["http.server", "SimpleHTTPServer", "uvicorn", "gunicorn", "hypercorn", "flask", "jupyter", "notebook"];

/// Tools that take a script from `package.json` (and the like) by name.
const PACKAGE_MANAGERS: &[&str] = &["npm", "pnpm", "yarn", "bun"];

/// Scripts a package manager runs until stopped, by their first part (`dev`, `start:prod`,
/// `dev:css`) or, for a watcher, their last (`test:watch`, `build:watch`).
const SERVING_SCRIPTS: &[&str] =
    &["dev", "develop", "start", "serve", "server", "watch", "preview", "storybook"];

/// What most tools call the subcommand that serves or watches: `hugo server`, `ng serve`,
/// `cargo watch`, `make dev`, `python manage.py runserver`, `vite preview`.
const SERVING_SUBCOMMANDS: &[&str] = &["dev", "serve", "server", "runserver", "watch", "preview"];

/// Subcommands that serve for one tool only (`rails s`, `flask run`).
const SERVING_FOR: &[(&str, &[&str])] = &[
    ("rails", &["s"]),
    ("ng", &["s"]),
    ("jekyll", &["s"]),
    ("flask", &["run"]),
    ("next", &["start"]),
    ("expo", &["start"]),
    ("docusaurus", &["start"]),
    ("jupyter", &["notebook", "lab", "server"]),
    ("kubectl", &["port-forward", "proxy"]),
];

/// Tools that serve or watch when given no subcommand.
const SERVING_BARE: &[&str] = &["vite", "vitest"];

/// Programs that run another one, which is the one that counts: `sudo nvim`, `npx vite`.
const WRAPPERS: &[&str] = &[
    "sudo",
    "doas",
    "time",
    "nohup",
    "exec",
    "env",
    "nice",
    "caffeinate",
    "command",
    "rlwrap",
    "unbuffer",
    "npx",
    "bunx",
];

/// Wrappers that name the program after `run` or `exec`: `uv run vite`, `bundle exec rails s`.
const RUNNERS: &[&str] = &["bundle", "poetry", "pipenv", "uv", "pdm"];

fn stays_open(words: &[&str]) -> bool {
    let Some((program, args)) = words.split_first() else { return false };
    let program = program.to_ascii_lowercase();
    let program = program.as_str();
    // Watch mode, wherever it is asked for: `jest --watch`, `tsc --watch`.
    if args.iter().any(|arg| arg.starts_with("--watch")) {
        return true;
    }
    if WRAPPERS.contains(&program) || RUNNERS.contains(&program) {
        let mut rest: Vec<&str> =
            args.iter().copied().skip_while(|arg| arg.starts_with('-') || arg.contains('=')).collect();
        if RUNNERS.contains(&program) && matches!(rest.first(), Some(&("run" | "exec"))) {
            rest.remove(0);
        }
        return stays_open(&rest);
    }
    if STAYS_OPEN.contains(&program) {
        return true;
    }
    if program == "deno"
        && let Some(("task", scripts)) = args.split_first().map(|(task, scripts)| (*task, scripts))
    {
        return script_stays_open(scripts);
    }
    if is_python(program) || REPLS.contains(&program) {
        return interpreter_stays_open(args);
    }
    if DB_SHELLS.contains(&program) {
        return !args.iter().any(|arg| ONE_SHOT.contains(arg));
    }
    if PACKAGE_MANAGERS.contains(&program) {
        return script_stays_open(args);
    }
    if matches!(program, "tail" | "journalctl") {
        return args.iter().any(|arg| matches!(*arg, "-f" | "-F" | "--follow"));
    }
    if matches!(program, "docker" | "docker-compose" | "podman" | "podman-compose") {
        return container_stays_open(args);
    }
    // The subcommand: the first word that is not a flag (or `cargo +nightly`).
    let subcommand = args.iter().copied().find(|arg| !arg.starts_with(['-', '+']));
    match subcommand {
        None => SERVING_BARE.contains(&program),
        Some(sub) => {
            SERVING_SUBCOMMANDS.contains(&sub)
                || SERVING_FOR.iter().any(|(tool, subs)| *tool == program && subs.contains(&sub))
        }
    }
}

/// `python3.12`, `pypy3`: a Python by any version.
fn is_python(program: &str) -> bool {
    program
        .strip_prefix("python")
        .or_else(|| program.strip_prefix("pypy"))
        .is_some_and(|version| version.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// An interpreter's arguments: a prompt when nothing is given to run (flags alone, or `-i`), a
/// server when `-m` names one, otherwise a script or a one-liner that ends.
fn interpreter_stays_open(args: &[&str]) -> bool {
    if let Some(module) = args.iter().position(|arg| *arg == "-m").and_then(|at| args.get(at + 1)) {
        return SERVER_MODULES.iter().any(|served| module.eq_ignore_ascii_case(served));
    }
    args.iter().all(|arg| arg.starts_with('-'))
        || args.iter().any(|arg| matches!(*arg, "-i" | "--interactive"))
}

/// `npm run dev`, `yarn start`, `pnpm --filter web dev`, `pnpm exec vite`: any word may name the
/// script, since flags such as `--filter` take values.
fn script_stays_open(args: &[&str]) -> bool {
    let words: Vec<&str> = args.iter().copied().filter(|arg| !arg.starts_with('-')).collect();
    if let Some((&("exec" | "dlx" | "x"), wrapped)) = words.split_first() {
        return stays_open(wrapped);
    }
    words.iter().any(|word| {
        let first = word.split(':').next().unwrap_or(word);
        let last = word.rsplit(':').next().unwrap_or(word);
        SERVING_SCRIPTS.contains(&first) || matches!(last, "watch" | "serve")
    })
}

/// `docker compose up` (not detached) and `docker logs -f`.
fn container_stays_open(args: &[&str]) -> bool {
    let has = |wanted: &[&str]| args.iter().any(|arg| wanted.contains(arg));
    (has(&["up"]) && !has(&["-d", "--detach"])) || (has(&["logs"]) && has(&["-f", "--follow"]))
}

/// What decides whether a terminal without an agent has "new output" on its row.
#[derive(Clone, Copy, Debug)]
pub struct OutputFacts {
    /// A coding agent runs in the terminal: it reports through its status instead.
    pub agent: bool,
    /// It is the terminal you are looking at.
    pub selected: bool,
    /// A program other than the shell holds the terminal.
    pub program_running: bool,
    /// Output arrived (or is still arriving) that nobody has looked at since.
    pub unseen: bool,
}

/// Whether the row says "new output". Only once the program has finished: a build, a dev server
/// or a tail keeps printing for as long as it runs, and the row already says what it runs. What
/// is news is that the command is done and left output you haven't seen. An agent's terminal
/// never says it (its status does), nor does the one in front.
pub fn has_new_output(facts: OutputFacts) -> bool {
    let OutputFacts { agent, selected, program_running, unseen } = facts;
    unseen && !agent && !selected && !program_running
}

#[cfg(test)]
mod tests {
    use super::{OutputFacts, ends_by_itself, has_new_output};

    /// Commands as `command_label` spells them.
    fn check(commands: &[&str], ends: bool) {
        for command in commands {
            assert_eq!(ends_by_itself(command), ends, "{command:?}");
        }
    }

    #[test]
    fn builds_tests_and_scripts_end_by_themselves() {
        check(
            &[
                "cargo test -p wormhole-core",
                "cargo build --release",
                "cargo +nightly fmt",
                "make",
                "make test",
                "npm test",
                "npm run build",
                "npm install",
                "pnpm install --frozen-lockfile",
                "yarn build",
                "bun test",
                "bun run build",
                "pytest -x",
                "python -m pytest tests",
                "python -m pip install rich",
                "python3 -c print(1)",
                "node -e console.log(1)",
                "deploy.sh prod",
                "seed.ts",
                "git pull --rebase",
                "rsync -a src/ dst/",
                "scp file host:",
                "brew upgrade",
                "docker build .",
                "docker compose up -d",
                "docker compose -f dev.yml up --detach",
                "vite build",
                "vitest run",
                "next build",
                "psql -c select",
                "sudo brew install x",
                "time cargo test",
                "uv run pytest",
                "bundle exec rake test",
                "tail -n 50 app.log",
            ],
            true,
        );
    }

    #[test]
    fn a_program_we_do_not_know_keeps_its_clock() {
        check(&["frobnicate --all", "./a.out", "a-very-long-job", ""], true);
    }

    #[test]
    fn editors_pagers_and_remote_shells_stay_open() {
        check(
            &[
                "vim src/sidebar.rs",
                "nvim",
                "nano notes.txt",
                "emacs -nw",
                "hx .",
                "micro main.go",
                "less log.txt",
                "more",
                "man ls",
                "ssh deploy@prod-1",
                "mosh host",
                "telnet host 25",
                "sudo nvim /etc/hosts",
                "Vim",
            ],
            false,
        );
    }

    #[test]
    fn multiplexers_and_monitors_stay_open() {
        check(
            &["tmux", "tmux attach -t main", "screen -r", "zellij", "top", "htop", "btop", "watch -n 1 ls"],
            false,
        );
    }

    #[test]
    fn a_repl_stays_open_until_it_is_given_something_to_run() {
        check(
            &[
                "python",
                "python3",
                "python3.12 -q",
                "Python",
                "python -i setup.py",
                "node",
                "node --inspect",
                "irb",
                "ghci",
                "ipython",
                "ruby",
                "deno",
                "rlwrap ghci",
                "sqlite3 app.db",
                "psql -h localhost app",
            ],
            false,
        );
        // The label names the script instead of the interpreter, so there is no REPL to mistake.
        check(
            &["python -c print(1)", "node -p 1+1", "ruby -e puts", "deno run main.ts", "deno task build"],
            true,
        );
    }

    #[test]
    fn dev_servers_and_watchers_stay_open() {
        check(
            &[
                "npm run dev",
                "npm start",
                "npm run start:dev",
                "npm run test:watch",
                "pnpm dev",
                "pnpm run dev",
                "pnpm --filter web dev",
                "pnpm exec vite",
                "yarn dev",
                "yarn start",
                "bun run dev",
                "bun dev",
                "deno task dev",
                "npx vite",
                "vite",
                "vite --host",
                "vite dev",
                "vite preview",
                "vitest",
                "next dev",
                "next start",
                "nuxt dev",
                "astro dev",
                "ng serve",
                "cargo watch -x test",
                "rails s",
                "rails server",
                "bundle exec rails s",
                "flask run",
                "manage.py runserver",
                "python -m http.server",
                "python3 -m http.server 8000",
                "python -m uvicorn app:app",
                "uvicorn app:app --reload",
                "hugo server",
                "mkdocs serve",
                "make dev",
                "jest --watch",
                "tsc --watch",
                "tail -f app.log",
                "docker compose up",
                "docker-compose up",
                "docker logs -f web",
                "kubectl port-forward svc/db 5432",
                "ollama serve",
            ],
            false,
        );
    }

    #[test]
    fn new_output_waits_for_the_program_to_finish() {
        let facts = OutputFacts { agent: false, selected: false, program_running: false, unseen: true };
        assert!(has_new_output(facts), "the command is done and left output nobody saw");
        let running = OutputFacts { program_running: true, ..facts };
        assert!(!has_new_output(running), "a dev server prints for as long as it runs");
        assert!(!has_new_output(OutputFacts { unseen: false, ..facts }), "nothing arrived");
        assert!(!has_new_output(OutputFacts { selected: true, ..facts }), "you are looking at it");
        assert!(!has_new_output(OutputFacts { agent: true, ..facts }), "an agent says it with its status");
    }
}
