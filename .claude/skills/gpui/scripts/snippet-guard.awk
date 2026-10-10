# Used by `gpui-docs check` (run with LC_ALL=C, after control characters were refused): prints "ok"
# for a snippet that can't read a file or an environment variable while it compiles, or why not.
#
# `check` is pre-approved, and a compiler error carries back whatever a snippet reads at compile time:
# `compile_error!(include_str!("…"))`, `env!`, a `mod` loaded from a #[path], gpui-component's
# `icon_named!` (lists a directory). Paths and renames (`use core::include_str as inc`) defeat a
# pattern on `name!(`, so the names are refused as words, anywhere outside comments: even in a string,
# which costs a rare false alarm, never a miss. Comments are dropped so prose can say anything.
#
# Dropping comments is safe only where rustc sees one too, so this lexes like rustc_lexer (strings and
# their escapes, raw and guarded strings, char literals and lifetimes, numbers, literal suffixes, which
# decide whether an `r"` starts a raw string). Outside strings and comments it accepts ASCII only: that
# is all it models, and rustc recovers from look-alike punctuation (a Greek question mark for `;`)
# instead of stopping. An unterminated literal or comment is refused, not guessed at.

function at(k) { return k <= n ? c[k] : "" }
function id_start(ch) { return ch ~ /^[A-Za-z_]$/ }
function id_continue(ch) { return ch ~ /^[A-Za-z0-9_]$/ }
function refuse(why) { print why; exit }
function ascii(ch) {
  if (ch != "" && ch !~ /^[ -~]$/ && ch != "\t" && ch != "\n" && ch != "\r")
    refuse("has a non-ASCII character outside strings and comments")
}
function ident(j) { while (id_continue(at(j))) j++; return j }
function suffix(j) { return id_start(at(j)) ? ident(j + 1) : j }
function digits(j) { while (at(j) ~ /^[0-9_]$/) j++; return j }
function exponent(j) { if (at(j) == "+" || at(j) == "-") j++; return digits(j) }

# From just after the opening quote; returns the index after the closing one.
function dq(j,   ch) {
  while (j <= n) {
    ch = c[j++]
    if (ch == "\"") return j
    if (ch == "\\" && (at(j) == "\\" || at(j) == "\"")) j++
  }
  refuse("has an unterminated string")
}

# From the first `#` or the quote after r, br or cr.
function raw(j,   h, e) {
  for (h = 0; at(j) == "#"; j++) h++
  if (at(j) != "\"") refuse("has a malformed raw string")
  for (j++; ; ) {
    while (j <= n && c[j] != "\"") j++
    if (j > n) refuse("has an unterminated raw string")
    j++
    for (e = 0; e < h && at(j) == "#"; j++) e++
    if (e == h) return j
  }
}

# From just after the opening quote of a char or byte literal.
function sq(j,   ch) {
  if (at(j + 1) == "'" && at(j) != "\\") { ascii(at(j)); return j + 2 }
  for (;;) {
    ch = at(j)
    if (ch == "'") return j + 1
    if (ch == "" || ch == "/" || (ch == "\n" && at(j + 1) != "'")) refuse("has an unterminated character literal")
    ascii(ch)
    if (ch == "\\") { ascii(at(j + 1)); j += 2 } else j++
  }
}

# At a `'`: a lifetime, a raw lifetime, or a char literal.
function quote(i,   a, b, j) {
  a = at(i + 1); b = at(i + 2)
  if (a == "") refuse("has an unterminated character literal")
  ascii(a)
  if (b != "'" && (id_start(a) || a ~ /^[0-9]$/)) {
    if (a == "r" && b == "#" && id_start(at(i + 3))) return ident(i + 3)
    j = ident(i + 2)
    return at(j) == "'" ? j + 1 : j
  }
  return suffix(sq(i + 1))
}

# At `#"` or `##`: reserved guarded strings, consumed whole by the 2024 lexer.
function guarded(i,   h, j, e) {
  for (j = i; at(j) == "#"; j++) h++
  if (at(j) != "\"") return i + 2
  j = dq(j + 1)
  for (e = 0; e < h && at(j) == "#"; j++) e++
  return suffix(j)
}

function number(j,   d, k) {
  if (c[j++] == "0") {
    d = at(j)
    if (d == "b" || d == "o" || d == "x") {
      k = ++j
      if (d == "x") { while (at(j) ~ /^[0-9a-fA-F_]$/) j++ } else j = digits(j)
      if (substr(src, k, j - k) !~ /[0-9a-fA-F]/) return j
    } else if (d ~ /^[0-9_]$/) j = digits(j)
    else if (d != "." && d != "e" && d != "E") return j
  } else j = digits(j)
  d = at(j)
  if (d == "." && at(j + 1) != "." && !id_start(at(j + 1))) {
    j++
    if (at(j) ~ /^[0-9]$/) {
      j = digits(j)
      if (at(j) == "e" || at(j) == "E") j = exponent(j + 1)
    }
    return j
  }
  return (d == "e" || d == "E") ? exponent(j + 1) : j
}

{ src = src $0 "\n" }

END {
  n = split(src, c, "")
  # The code without comments, each comment replaced by a space.
  for (i = 1; i <= n; ) {
    ch = c[i]; nx = at(i + 1)
    if (ch == "/" && (nx == "/" || nx == "*")) {
      code = code substr(src, last + 1, i - last - 1) " "
      if (nx == "/") {
        while (i <= n && c[i] != "\n") i++
      } else {
        for (i += 2; depth >= 0; ) {
          if (i > n) refuse("has an unterminated comment")
          if (c[i] == "/" && at(i + 1) == "*") { depth++; i += 2 }
          else if (c[i] == "*" && at(i + 1) == "/") { depth--; i += 2 }
          else i++
        }
        depth = 0
      }
      last = i - 1
      continue
    }
    if (ch == "\"") i = suffix(dq(i + 1))
    else if (ch == "'") i = quote(i)
    else if (ch == "#" && (nx == "\"" || nx == "#")) i = guarded(i)
    else if (ch == "r" && nx == "#" && id_start(at(i + 2))) i = ident(i + 2)
    else if (ch == "r" && (nx == "#" || nx == "\"")) i = suffix(raw(i + 1))
    else if ((ch == "b" || ch == "c") && nx == "\"") i = suffix(dq(i + 2))
    else if (ch == "b" && nx == "'") i = suffix(sq(i + 2))
    else if ((ch == "b" || ch == "c") && nx == "r" && (at(i + 2) == "\"" || at(i + 2) == "#")) i = suffix(raw(i + 2))
    else if (id_start(ch)) i = ident(i + 1)
    else if (ch ~ /^[0-9]$/) i = suffix(number(i))
    else { ascii(ch); i++ }
  }
  code = " " code substr(src, last + 1) " "
  gsub(/[\t\n\r]/, " ", code)

  if (match(code, /[^A-Za-z0-9_](include|include_str|include_bytes|env|option_env|icon_named|debugger_visualizer)[^A-Za-z0-9_]/))
    refuse("uses `" substr(code, RSTART + 1, RLENGTH - 2) "` (refused anywhere, even as a name or in a string)")
  # Only inline modules: `mod name;` loads a file, from anywhere with #[path] or #[cfg_attr(…, path)].
  for (rest = code; match(rest, /[^A-Za-z0-9_]mod[^A-Za-z0-9_]/); ) {
    rest = substr(rest, RSTART + 4)
    if (!match(rest, /^ +(r#)?[A-Za-z_][A-Za-z0-9_]* *[{]/))
      refuse("declares a module without a body (`mod name;` loads a file): write it inline, `mod name { … }`")
  }
  # Crate-level attributes and other crates: nothing a snippet needs, and more ways to load things.
  if (code ~ /#[ ]*!/) refuse("has an inner attribute (`#![…]`): the check crate already allows unused code")
  if (code ~ /[^A-Za-z0-9_]extern +crate[^A-Za-z0-9_]/) refuse("uses `extern crate`: reach everything through `gpui_kit`")
  print "ok"
}
