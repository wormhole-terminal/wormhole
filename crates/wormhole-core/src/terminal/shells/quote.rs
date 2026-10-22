//! Words for a shell's command line: a bare program name from a profile runs through the login
//! shell (`$SHELL -l -c 'exec claude …'`), whose line is built here, so an argument with a space
//! or a quote arrives as the one word it was.

/// `word` as one word of a POSIX shell command line: as it is when every character is safe, else
/// in single quotes, with a quote inside written as `'\''` (the quoting zsh, bash, fish and sh
/// read alike).
pub fn quote(word: &str) -> String {
    if !word.is_empty() && word.chars().all(is_safe) {
        return word.to_string();
    }
    single_quote(word)
}

/// `word` in single quotes always, even when it needs none (a path typed at a prompt reads as one
/// word whatever it holds): a quote inside is written as `'\''`.
pub fn single_quote(word: &str) -> String {
    let mut out = String::with_capacity(word.len() + 2);
    out.push('\'');
    for c in word.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// `words` as one command line, each word quoted as it needs.
pub fn quote_all<I, S>(words: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    words.into_iter().map(|word| quote(word.as_ref())).collect::<Vec<_>>().join(" ")
}

/// A character no shell reads as anything but itself, outside quotes.
fn is_safe(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@' | '%' | '+' | ',' | '=')
}

#[cfg(test)]
mod tests {
    use super::{quote, quote_all, single_quote};

    #[test]
    fn safe_words_stay_bare() {
        for word in ["claude", "/opt/homebrew/bin/fish", "-l", "--model=opus", "a,b", "v1.2+3", "x@y:z%"] {
            assert_eq!(quote(word), word);
        }
    }

    #[test]
    fn a_single_quoted_word_is_quoted_even_when_it_is_safe() {
        assert_eq!(single_quote("/Users/ben/Projects/tube"), "'/Users/ben/Projects/tube'");
        assert_eq!(single_quote("/tmp/my dir/$HOME"), "'/tmp/my dir/$HOME'");
        assert_eq!(single_quote("/tmp/it's"), r#"'/tmp/it'\''s'"#);
        assert_eq!(single_quote(""), "''");
    }

    #[test]
    fn anything_a_shell_would_read_is_quoted() {
        assert_eq!(quote(""), "''");
        assert_eq!(quote("two words"), "'two words'");
        assert_eq!(quote("$HOME"), "'$HOME'");
        assert_eq!(quote("a;rm -rf"), "'a;rm -rf'");
        assert_eq!(quote("it's"), r#"'it'\''s'"#);
        assert_eq!(quote("*"), "'*'");
        assert_eq!(quote("tab\there"), "'tab\there'");
    }

    #[test]
    fn a_command_line_joins_its_words() {
        assert_eq!(quote_all(["exec", "claude", "--model", "opus 4"]), "exec claude --model 'opus 4'");
        assert_eq!(quote_all(Vec::<String>::new()), "");
        assert_eq!(quote_all([String::from("npm"), String::from("run")]), "npm run");
    }
}
