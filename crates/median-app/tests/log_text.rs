use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// `tracing` macros that write a log line.
const LEVELS: [&str; 5] = ["trace", "debug", "info", "warn", "error"];

/// Words in a message that mean nothing to someone who has not read the code.
const BANNED_WORDS: [&str; 9] = [
    "mirror",
    "wall",
    "slate",
    "landed",
    "stands",
    "unjudged",
    "stretch",
    "gave back",
    "hands over",
];

/// Field names that mean nothing without the code. An error goes in `error`, not `why` or `e`.
const BANNED_FIELDS: [&str; 8] = [
    "why",
    "e",
    "stands",
    "landed",
    "strangers",
    "micros",
    "front",
    "wanted",
];

/// Every `.rs` file under the `src` of each crate in the workspace.
fn sources() -> Vec<PathBuf> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for entry in fs::read_dir(&crates).expect("the crates directory reads") {
        let src = entry.expect("a crate directory reads").path().join("src");
        if src.is_dir() {
            walk(&src, &mut found);
        }
    }
    found
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a source directory reads") {
        let path = entry.expect("a source entry reads").path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The line and the arguments of every log macro call in `stream`, nested ones included.
fn calls(stream: TokenStream, found: &mut Vec<(usize, TokenStream)>) {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    for (at, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            calls(group.stream(), found);
        }
        let (TokenTree::Ident(name), Some(TokenTree::Punct(bang)), Some(TokenTree::Group(args))) =
            (token, tokens.get(at + 1), tokens.get(at + 2))
        else {
            continue;
        };
        if LEVELS.contains(&name.to_string().as_str())
            && bang.as_char() == '!'
            && args.delimiter() == Delimiter::Parenthesis
        {
            found.push((name.span().start().line, args.stream()));
        }
    }
}

/// Rule violations in one call's arguments; empty when there are none.
fn problems(args: TokenStream) -> Vec<String> {
    let mut parts: Vec<Vec<TokenTree>> = vec![Vec::new()];
    for token in args {
        match &token {
            TokenTree::Punct(comma) if comma.as_char() == ',' => parts.push(Vec::new()),
            _ => parts.last_mut().expect("parts start with one").push(token),
        }
    }
    if parts.last().is_some_and(Vec::is_empty) {
        parts.pop();
    }
    let Some((message, fields)) = parts.split_last() else {
        return vec!["no message".to_owned()];
    };

    let mut found = Vec::new();
    match message.as_slice() {
        [TokenTree::Literal(literal)] if literal.to_string().starts_with('"') => {
            let quoted = literal.to_string();
            found.extend(message_problems(&quoted[1..quoted.len() - 1]));
        }
        _ => found.push("the message is not a plain string literal after the fields".to_owned()),
    }
    for field in fields {
        if let Some(name) = field_name(field)
            && BANNED_FIELDS.contains(&name.as_str())
        {
            found.push(format!("field `{name}` says nothing without the code"));
        }
    }
    found
}

fn message_problems(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    if text.contains(['{', '}']) {
        found.push(format!("`{text}`: values go in fields, not in the message"));
    }
    if !text.starts_with(|first: char| first.is_ascii_lowercase()) {
        found.push(format!(
            "`{text}`: the message starts with a lowercase subsystem name"
        ));
    }
    let lower = text.to_lowercase();
    if ["the ", "a ", "an "]
        .iter()
        .any(|article| lower.starts_with(article))
    {
        found.push(format!(
            "`{text}`: the message names the subsystem, it does not narrate"
        ));
    }
    let spaced = format!(" {} ", lower.replace([',', ';', ':', '.'], " "));
    for word in BANNED_WORDS {
        if spaced.contains(&format!(" {word} ")) {
            found.push(format!("`{text}`: `{word}` is a banned word"));
        }
    }
    found
}

/// Field name of one argument: `name = …`, `%name`, `?name` or `name`; `None` for `parent:`
/// and `target:`, which are not fields.
fn field_name(field: &[TokenTree]) -> Option<String> {
    let mut tokens = field.iter().skip_while(
        |token| matches!(token, TokenTree::Punct(sigil) if matches!(sigil.as_char(), '%' | '?')),
    );
    let TokenTree::Ident(name) = tokens.next()? else {
        return None;
    };
    match tokens.next() {
        Some(TokenTree::Punct(colon)) if colon.as_char() == ':' => None,
        _ => Some(name.to_string()),
    }
}

#[test]
fn every_log_call_keeps_the_writing_rule() {
    let mut broken = Vec::new();
    let mut seen = 0;
    for path in sources() {
        let source = fs::read_to_string(&path).expect("a source file reads");
        let stream = TokenStream::from_str(&source)
            .unwrap_or_else(|why| panic!("{} does not tokenize: {why}", path.display()));
        let mut found = Vec::new();
        calls(stream, &mut found);
        seen += found.len();
        for (line, args) in found {
            for problem in problems(args) {
                broken.push(format!("{}:{line}: {problem}", path.display()));
            }
        }
    }
    assert!(
        seen > 0,
        "no log calls were found; the scan itself is broken"
    );
    assert!(
        broken.is_empty(),
        "log calls that break the writing rule:\n{}",
        broken.join("\n")
    );
}

#[test]
fn a_narrated_message_with_values_in_it_is_caught() {
    let mut found = Vec::new();
    calls(
        TokenStream::from_str(r#"fn f() { warn!(%why, "the mirror said {x}"); }"#).unwrap(),
        &mut found,
    );
    let said = problems(found.remove(0).1).join("\n");
    assert!(said.contains("values go in fields"), "{said}");
    assert!(said.contains("does not narrate"), "{said}");
    assert!(said.contains("`mirror` is a banned word"), "{said}");
    assert!(said.contains("field `why`"), "{said}");
}

#[test]
fn a_message_with_format_arguments_is_caught() {
    let said = problems(TokenStream::from_str(r#"slug, "price read failed: {}", e"#).unwrap());
    assert_eq!(
        said,
        ["the message is not a plain string literal after the fields"]
    );
}

#[test]
fn a_call_that_keeps_the_rule_passes() {
    let said = problems(
        TokenStream::from_str(r#"parent: &span, error = %why, took_ms = 3, "catalog installed""#)
            .unwrap(),
    );
    assert!(said.is_empty(), "{said:?}");
}
