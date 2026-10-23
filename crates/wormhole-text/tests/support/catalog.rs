//! The `.ftl` files as the rules see them: per message (and attribute, and term) the variables it
//! uses, what it refers to, how its selects are made.

use std::collections::{BTreeMap, BTreeSet};

use fluent_syntax::ast::{self, Entry, Expression, InlineExpression, PatternElement, VariantKey};
use fluent_syntax::{parser, serializer};

/// The words CLDR calls plural categories.
pub(crate) const CATEGORIES: [&str; 6] = ["zero", "one", "two", "few", "many", "other"];

/// A message's value or one of its attributes (an id `name` or `name.attribute`), or a term's
/// (`-name`, `-name.attribute`).
#[derive(Default, Clone, Debug)]
pub(crate) struct Item {
    pub id: String,
    pub file: String,
    /// Variables the pattern itself uses, in order of first use.
    pub vars: Vec<String>,
    /// Variables a plural select is made on: whole numbers, not text.
    pub counted: BTreeSet<String>,
    /// Messages it refers to: `name` or `name.attribute`.
    pub messages: BTreeSet<String>,
    /// Terms it refers to: `-name` or `-name.attribute`.
    pub terms: BTreeSet<String>,
    pub functions: BTreeSet<String>,
    pub selects: Vec<Select>,
    /// The pattern as Fluent writes it, for the doc comment of a function.
    pub source: String,
    /// Whether the message it belongs to has a comment for the translator.
    pub commented: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Select {
    pub on_variable: Option<String>,
    pub keys: Vec<VariantKeyText>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum VariantKeyText {
    Name(String),
    Number(String),
}

impl Select {
    /// A select whose keys are plural categories or numbers (`[one]`, `[0]`), not words like `[male]`.
    pub(crate) fn is_plural(&self) -> bool {
        self.keys.iter().any(|key| match key {
            VariantKeyText::Number(_) => true,
            VariantKeyText::Name(name) => CATEGORIES.contains(&name.as_str()),
        })
    }
}

#[derive(Default)]
pub(crate) struct Catalog {
    /// Messages' values and attributes, in file order.
    pub messages: Vec<Item>,
    pub terms: Vec<Item>,
    /// Ids defined more than once (across the files too).
    pub duplicates: Vec<String>,
    /// Text the parser gave up on, with its file.
    pub junk: Vec<String>,
}

impl Catalog {
    pub(crate) fn parse(files: &[(String, String)]) -> Catalog {
        let mut catalog = Catalog::default();
        let mut seen = BTreeSet::new();
        for (file, source) in files {
            let resource = match parser::parse(source.as_str()) {
                Ok(resource) => resource,
                Err((resource, errors)) => {
                    for error in errors {
                        let slice = error.slice.clone().unwrap_or(error.pos.clone());
                        catalog.junk.push(format!("{file}: {:?}: {}", error.kind, &source[slice]));
                    }
                    resource
                }
            };
            for entry in &resource.body {
                match entry {
                    Entry::Message(message) => {
                        let name = message.id.name;
                        if !seen.insert(name.to_owned()) {
                            catalog.duplicates.push(name.to_owned());
                        }
                        let commented = message.comment.is_some();
                        if let Some(pattern) = &message.value {
                            catalog.messages.push(item(name, file, pattern, commented));
                        }
                        for attribute in &message.attributes {
                            let id = format!("{name}.{}", attribute.id.name);
                            catalog.messages.push(item(&id, file, &attribute.value, commented));
                        }
                    }
                    Entry::Term(term) => {
                        let name = format!("-{}", term.id.name);
                        if !seen.insert(name.clone()) {
                            catalog.duplicates.push(name.clone());
                        }
                        catalog.terms.push(item(&name, file, &term.value, true));
                        for attribute in &term.attributes {
                            let id = format!("{name}.{}", attribute.id.name);
                            catalog.terms.push(item(&id, file, &attribute.value, true));
                        }
                    }
                    Entry::Junk { content } => catalog.junk.push(format!("{file}: {content}")),
                    Entry::Comment(_) | Entry::GroupComment(_) | Entry::ResourceComment(_) => {}
                }
            }
        }
        catalog
    }

    pub(crate) fn message(&self, id: &str) -> Option<&Item> {
        self.messages.iter().find(|item| item.id == id)
    }

    pub(crate) fn has_term(&self, id: &str) -> bool {
        self.terms.iter().any(|item| item.id == id)
    }

    /// The variables a message needs from its caller: its own, and those of every message it
    /// refers to (a reference passes the caller's arguments on). A term does not: it has its own.
    pub(crate) fn variables(&self, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut visited = BTreeSet::new();
        self.collect(id, &mut visited, &mut out);
        out
    }

    fn collect(&self, id: &str, visited: &mut BTreeSet<String>, out: &mut Vec<String>) {
        if !visited.insert(id.to_owned()) {
            return;
        }
        let Some(item) = self.message(id) else { return };
        for var in &item.vars {
            if !out.contains(var) {
                out.push(var.clone());
            }
        }
        for reference in &item.messages {
            self.collect(reference, visited, out);
        }
    }

    /// Whether the message needs a whole number for `var`, here or in a message it refers to.
    pub(crate) fn counted(&self, id: &str, var: &str) -> bool {
        let mut visited = BTreeSet::new();
        self.counted_in(id, var, &mut visited)
    }

    fn counted_in(&self, id: &str, var: &str, visited: &mut BTreeSet<String>) -> bool {
        if !visited.insert(id.to_owned()) {
            return false;
        }
        let Some(item) = self.message(id) else { return false };
        item.counted.contains(var) || item.messages.iter().any(|m| self.counted_in(m, var, visited))
    }

    /// Ids by item, for comparisons.
    pub(crate) fn by_id(&self) -> BTreeMap<&str, &Item> {
        self.messages.iter().map(|item| (item.id.as_str(), item)).collect()
    }
}

fn item(id: &str, file: &str, pattern: &ast::Pattern<&str>, commented: bool) -> Item {
    let mut item = Item { id: id.to_owned(), file: file.to_owned(), commented, ..Item::default() };
    walk_pattern(&mut item, pattern);
    item.source = source_of(pattern);
    item
}

fn walk_pattern(item: &mut Item, pattern: &ast::Pattern<&str>) {
    for element in &pattern.elements {
        if let PatternElement::Placeable { expression } = element {
            walk_expression(item, expression);
        }
    }
}

fn walk_expression(item: &mut Item, expression: &Expression<&str>) {
    match expression {
        Expression::Inline(inline) => walk_inline(item, inline),
        Expression::Select { selector, variants } => {
            walk_inline(item, selector);
            let keys: Vec<VariantKeyText> = variants
                .iter()
                .map(|variant| match &variant.key {
                    VariantKey::Identifier { name } => VariantKeyText::Name((*name).to_owned()),
                    VariantKey::NumberLiteral { value } => VariantKeyText::Number((*value).to_owned()),
                })
                .collect();
            let on_variable = match selector {
                InlineExpression::VariableReference { id } => Some(id.name.to_owned()),
                _ => None,
            };
            let select = Select { on_variable, keys };
            if let (Some(var), true) = (&select.on_variable, select.is_plural()) {
                item.counted.insert(var.clone());
            }
            item.selects.push(select);
            for variant in variants {
                walk_pattern(item, &variant.value);
            }
        }
    }
}

fn walk_inline(item: &mut Item, inline: &InlineExpression<&str>) {
    match inline {
        InlineExpression::StringLiteral { .. } | InlineExpression::NumberLiteral { .. } => {}
        InlineExpression::FunctionReference { id, arguments } => {
            item.functions.insert(id.name.to_owned());
            walk_arguments(item, arguments);
        }
        InlineExpression::MessageReference { id, attribute } => {
            item.messages.insert(reference(id.name, attribute.as_ref()));
        }
        InlineExpression::TermReference { id, attribute, arguments } => {
            item.terms.insert(format!("-{}", reference(id.name, attribute.as_ref())));
            if let Some(arguments) = arguments {
                walk_arguments(item, arguments);
            }
        }
        InlineExpression::VariableReference { id } => {
            if !item.vars.iter().any(|v| v == id.name) {
                item.vars.push(id.name.to_owned());
            }
        }
        InlineExpression::Placeable { expression } => walk_expression(item, expression),
    }
}

fn walk_arguments(item: &mut Item, arguments: &ast::CallArguments<&str>) {
    for positional in &arguments.positional {
        walk_inline(item, positional);
    }
    for named in &arguments.named {
        walk_inline(item, &named.value);
    }
}

fn reference(name: &str, attribute: Option<&ast::Identifier<&str>>) -> String {
    match attribute {
        Some(attribute) => format!("{name}.{}", attribute.name),
        None => name.to_owned(),
    }
}

/// A pattern as Fluent writes it: serialised as a message `x`, the `x =` taken off, and a select that
/// the serialiser moved to the lines below taken back out to the left.
fn source_of(pattern: &ast::Pattern<&str>) -> String {
    let message = ast::Message {
        id: ast::Identifier { name: "x" },
        value: Some(pattern.clone()),
        attributes: Vec::new(),
        comment: None,
    };
    let text = serializer::serialize(&ast::Resource { body: vec![Entry::Message(message)] });
    let body = text.trim_end().strip_prefix("x =").unwrap_or(&text);
    let lines: Vec<&str> = body.lines().filter(|line| !line.trim().is_empty()).collect();
    let indent = lines.iter().skip(1).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    let mut out: Vec<&str> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        out.push(if i == 0 { line.trim_start() } else { &line[indent.min(line.len())..] });
    }
    out.join("\n")
}
