//! Keeps the methods written inside `define_class!` trivially simple.
//!
//! The CRAP gate scores every function it can see, but it cannot look inside macros, so
//! the Objective-C callback methods would escape it. This check closes that gap: those
//! methods may not contain any decision at all, only a call into ordinary code.

use crate::runner::Runner;
use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, TokenStream, TokenTree};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub line: usize,
    pub what: String,
}

const KEYWORDS: [&str; 5] = ["if", "match", "while", "for", "loop"];

/// Method bodies start two braces deep: `impl … { fn … { body } }`.
const BODY_DEPTH: usize = 2;

fn braces(group: &Group) -> usize {
    usize::from(group.delimiter() == Delimiter::Brace)
}

fn keyword(ident: &Ident, found: &mut Vec<Finding>) {
    let word = ident.to_string();
    if KEYWORDS.contains(&word.as_str()) {
        found.push(Finding {
            line: ident.span().start().line,
            what: word,
        });
    }
}

/// `?`, `&&` and `||`. (`previous` is the punctuation directly before, if any.)
fn operator(punct: &Punct, previous: Option<&Punct>, found: &mut Vec<Finding>) {
    let symbol = punct.as_char();
    let doubled = matches!(symbol, '&' | '|')
        && previous.is_some_and(|p| p.as_char() == symbol && p.spacing() == Spacing::Joint);
    let what = match (symbol, doubled) {
        ('?', _) => "?".to_string(),
        (_, true) => format!("{symbol}{symbol}"),
        _ => return,
    };
    found.push(Finding {
        line: punct.span().start().line,
        what,
    });
}

fn check_body(tokens: TokenStream, depth: usize, found: &mut Vec<Finding>) {
    let mut previous: Option<Punct> = None;
    for tree in tokens {
        let before = previous.take();
        match tree {
            TokenTree::Group(group) => {
                check_body(group.stream(), depth + braces(&group), found);
            }
            TokenTree::Ident(ident) if depth >= BODY_DEPTH => keyword(&ident, found),
            TokenTree::Punct(punct) if depth >= BODY_DEPTH => {
                operator(&punct, before.as_ref(), found);
                previous = Some(punct);
            }
            _ => {}
        }
    }
}

fn follows_class_macro(before: &[TokenTree]) -> bool {
    matches!(
        before,
        [.., TokenTree::Ident(name), TokenTree::Punct(bang)]
            if name == "define_class" && bang.as_char() == '!'
    )
}

/// Finds `define_class!( … )` anywhere, including inside modules and functions.
fn scan(tokens: TokenStream, found: &mut Vec<Finding>) {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, tree) in trees.iter().enumerate() {
        let TokenTree::Group(group) = tree else {
            continue;
        };
        if follows_class_macro(&trees[..index]) {
            check_body(group.stream(), 0, found);
        } else {
            scan(group.stream(), found);
        }
    }
}

/// Every `if`, `match`, `while`, `for`, `loop`, `?`, `&&` and `||` inside a method body
/// of a `define_class!` block.
pub fn decisions_in_class_macros(source: &str) -> Vec<Finding> {
    let mut found = Vec::new();
    match source.parse::<TokenStream>() {
        Ok(tokens) => scan(tokens, &mut found),
        Err(_) => found.push(Finding {
            line: 0,
            what: "unreadable source".into(),
        }),
    }
    found
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn problems_in(file: &Path) -> Vec<String> {
    let source = fs::read_to_string(file).unwrap_or_default();
    decisions_in_class_macros(&source)
        .into_iter()
        .map(|finding| {
            format!(
                "{}:{}: `{}` inside define_class!",
                file.display(),
                finding.line,
                finding.what
            )
        })
        .collect()
}

/// Checks every `.rs` file under `dirs`. Lists what it finds; true if there is nothing.
pub fn check(runner: &dyn Runner, dirs: &[PathBuf]) -> bool {
    runner.say("▶ lint-shell: methods inside define_class! must be single calls");
    let problems: Vec<String> = dirs
        .iter()
        .flat_map(|dir| rust_files(dir))
        .flat_map(|file| problems_in(&file))
        .collect();
    problems
        .iter()
        .for_each(|problem| runner.say(&format!("✗ {problem}")));
    problems.is_empty()
}

#[cfg(test)]
mod tests;
