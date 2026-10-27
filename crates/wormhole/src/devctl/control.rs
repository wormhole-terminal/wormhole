//! `control [--from <tab>] <words…>`: a control request as `Wormhole <words…>` sends it over the
//! socket, here handed straight to the app's handler, from the tab `--from` names (none: a shell
//! outside the app). Answered after this update (the handler reads every window, this one
//! included) and appended to the results as the reply's one JSON line.

use anyhow::{Result, anyhow};
use wormhole_core::ipc::Peer;
use wormhole_core::ipc::control::{Parsed, Request, help, parse};
use wormhole_core::platform::paths;

use super::Ctx;
use super::ambient::later;
use crate::app::control;

pub(super) fn control(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = split_words(arg);
    let mut from = None;
    if words.first().map(String::as_str) == Some("--from") {
        words.remove(0);
        let tab = words.first().ok_or_else(|| anyhow!("--from needs a tab id"))?;
        from = Some(tab.clone());
        words.remove(0);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let home = paths::home();
    let cli = match parse(&words, &cwd, home.as_deref()).map_err(|mistake| anyhow!(mistake))? {
        Parsed::Run(cli) => cli,
        Parsed::Help => return Ok(help()),
        Parsed::HelpJson => return Ok(wormhole_core::ipc::control::help_json()),
    };
    let request = Request::new(cli.action, from);
    later(ctx, move |cx| {
        format!(
            "{}\n",
            serde_json::to_string(&control::handle(request, &Peer::default(), cx)).unwrap_or_default()
        )
    });
    Ok(String::new())
}

/// The argument as words, as a shell would read them: split on blanks, with double quotes keeping
/// a run with blanks together (`send-text --tab 3 "run the tests"`).
fn split_words(arg: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut in_word = false;
    for c in arg.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                in_word = true;
            }
            c if c.is_whitespace() && !quoted => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            c => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::split_words;

    #[test]
    fn words_split_on_blanks_and_quotes_keep_them_together() {
        assert_eq!(split_words("list --json"), ["list", "--json"]);
        assert_eq!(
            split_words(r#"send-text --tab 3 "run the tests" --submit"#),
            ["send-text", "--tab", "3", "run the tests", "--submit"]
        );
        assert_eq!(
            split_words(r#"notify --title "" done"#),
            ["notify", "--title", "", "done"],
            "an empty word"
        );
        assert_eq!(split_words("  "), Vec::<String>::new());
    }
}
