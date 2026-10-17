//! No store path and no census key is built from a vendor's instrument token.
//! R4-17, CON-13.
//!
//! # Why a source check
//!
//! A broker's history is asked for by an instrument token, and the token is
//! today's: NSE reuses one across an expiry boundary (`CLAUDE.md` §1), and the
//! identity audit found minutes of another instrument under a key whose token
//! had moved (`pull::series_identity`). The store and the census are keyed by
//! exchange, segment, symbol and contract, so a token can name nothing on disk
//! — as long as no key type grows a token field and no key builder takes one.
//! Nothing about a type can state "and never a token", so this file reads the
//! source, in the style of `crates/core/tests/lint.rs`.
//!
//! # What it checks
//!
//! In the six modules [`MODULES`] names — where the store path, the census
//! entry key, the contract and the instrument key are defined and where the
//! census and the facets build their keys — no identifier in:
//!
//! - the body of `PathParts`, `StorePath`, `EntryKey`, `Contract` or
//!   `InstrumentKey`;
//! - the parameters of `StorePath::new`, `StorePath::for_key`,
//!   `gdfl_lineage::option_key` or `gdfl_lineage::parse_option_key`;
//! - any `EntryKey { .. }` or `PathParts { .. }` literal those modules build;
//!
//! names [`FORBIDDEN`] (or holds `token` at all). Comments are not code and
//! are not read.
//!
//! # Closed over its module list
//!
//! Each named item must be found in exactly one listed module, every listed
//! module must hold something checked, and no other non-test source file of
//! `core`, `store`, `pull` or `api` may define a type or function of those names: a
//! new definition elsewhere fails this test until it is listed and read.
//!
//! # And it can fail
//!
//! [`a_token_field_in_a_fixture_key_builder_is_refused`] runs the same checker
//! over an in-memory module whose `EntryKey` holds an `instrument_token`: a
//! checker that refuses nothing fails it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

/// The modules read, each by its path from the workspace root.
const MODULES: [(&str, &str); 6] = [
    (
        "crates/store/src/path.rs",
        include_str!("../../store/src/path.rs"),
    ),
    (
        "crates/pull/src/manifest.rs",
        include_str!("../src/manifest.rs"),
    ),
    (
        "crates/core/src/instrument.rs",
        include_str!("../../core/src/instrument.rs"),
    ),
    (
        "crates/pull/src/census_store.rs",
        include_str!("../src/census_store.rs"),
    ),
    (
        "crates/pull/src/gdfl_lineage.rs",
        include_str!("../src/gdfl_lineage.rs"),
    ),
    (
        "crates/api/src/facets.rs",
        include_str!("../../api/src/facets.rs"),
    ),
];

/// The key types whose bodies are read.
const TYPES: [&str; 5] = [
    "PathParts",
    "StorePath",
    "EntryKey",
    "Contract",
    "InstrumentKey",
];

/// The functions whose parameters are read, each with the impl it sits in
/// (`""` for a free function).
const FUNCTIONS: [(&str, &str); 4] = [
    ("StorePath", "new"),
    ("StorePath", "for_key"),
    ("", "option_key"),
    ("", "parse_option_key"),
];

/// The literals whose fields are read wherever a listed module builds one.
const LITERALS: [&str; 2] = ["EntryKey", "PathParts"];

/// Identifiers a key may never name. Any identifier holding `token` is refused
/// besides these.
const FORBIDDEN: [&str; 5] = [
    "token",
    "instrument_token",
    "exchange_token",
    "security_id",
    "fininstrmid",
];

/// `text` with every `//` comment and `/* */` comment blanked, so prose about
/// a token is not read as a field. String literals are kept as they are.
fn code_only(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut in_string = false;
    while let Some(&byte) = bytes.get(at) {
        if in_string {
            out.push(char::from(byte));
            if byte == b'\\' {
                if let Some(&next) = bytes.get(at + 1) {
                    out.push(char::from(next));
                }
                at += 2;
                continue;
            }
            in_string = byte != b'"';
            at += 1;
            continue;
        }
        match (byte, bytes.get(at + 1)) {
            (b'/', Some(b'/')) => {
                while bytes.get(at).is_some_and(|b| *b != b'\n') {
                    at += 1;
                }
            }
            (b'/', Some(b'*')) => {
                at += 2;
                while bytes.get(at).is_some()
                    && !(bytes[at] == b'*' && bytes.get(at + 1) == Some(&b'/'))
                {
                    at += 1;
                }
                at += 2;
            }
            _ => {
                in_string = byte == b'"';
                // Only ASCII is read: a non-ASCII byte names no identifier.
                out.push(if byte.is_ascii() {
                    char::from(byte)
                } else {
                    ' '
                });
                at += 1;
            }
        }
    }
    out
}

/// The text between the delimiter `open` at `from` and its matching `close`.
fn delimited(code: &str, from: usize, open: u8, close: u8) -> Option<&str> {
    let bytes = code.as_bytes();
    let start = from + code.get(from..)?.find(char::from(open))?;
    let mut depth = 0usize;
    for (offset, byte) in bytes.get(start..)?.iter().enumerate() {
        if *byte == open {
            depth += 1;
        } else if *byte == close {
            depth -= 1;
            if depth == 0 {
                return code.get(start + 1..start + offset);
            }
        }
    }
    None
}

/// Every position at which `needle` occurs as whole words in `code`.
fn word_positions(code: &str, needle: &str) -> Vec<usize> {
    let bytes = code.as_bytes();
    let word = |at: Option<&u8>| at.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_');
    code.match_indices(needle)
        .map(|(at, _)| at)
        .filter(|at| {
            !word(at.checked_sub(1).and_then(|before| bytes.get(before)))
                && !word(bytes.get(at + needle.len()))
        })
        .collect()
}

/// Every identifier of `text` that a key may not name.
fn forbidden_in(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map(str::to_ascii_lowercase)
        .filter(|ident| FORBIDDEN.contains(&ident.as_str()) || ident.contains("token"))
        .collect()
}

/// One finding: where, what was read, and the forbidden names in it.
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    module: String,
    item: String,
    names: Vec<String>,
}

/// What the checker read in one module, and what it refused.
#[derive(Debug, Default)]
struct Read {
    /// Each item found, by kind and name.
    found: Vec<String>,
    findings: Vec<Finding>,
}

/// Read one module: every listed type's body, every listed function's
/// parameters and every listed literal's fields.
fn check(module: &str, text: &str) -> Read {
    let code = code_only(text);
    let mut read = Read::default();
    let mut judge = |item: String, body: &str| {
        let names = forbidden_in(body);
        if !names.is_empty() {
            read.findings.push(Finding {
                module: module.to_owned(),
                item: item.clone(),
                names,
            });
        }
        read.found.push(item);
    };
    for name in TYPES {
        for keyword in ["struct", "enum"] {
            for at in word_positions(&code, &format!("{keyword} {name}")) {
                if let Some(body) = delimited(&code, at, b'{', b'}') {
                    judge(format!("type {name}"), body);
                }
            }
        }
    }
    for (owner, name) in FUNCTIONS {
        let scope = if owner.is_empty() {
            Some(code.as_str())
        } else {
            word_positions(&code, &format!("impl<'a> {owner}<'a>"))
                .first()
                .and_then(|at| delimited(&code, *at, b'{', b'}'))
        };
        let Some(scope) = scope else { continue };
        for at in word_positions(scope, &format!("fn {name}")) {
            if let Some(parameters) = delimited(scope, at, b'(', b')') {
                judge(format!("fn {owner}::{name}"), parameters);
            }
        }
    }
    for name in LITERALS {
        for at in word_positions(&code, &format!("{name} {{")) {
            // A definition is read above, as a type; an impl block or a return
            // type followed by its body is not a literal.
            let before = code.get(..at).unwrap_or("").trim_end();
            if ["struct", "enum", "impl", "for", "->"]
                .iter()
                .any(|word| before.ends_with(word))
            {
                continue;
            }
            if let Some(fields) = delimited(&code, at, b'{', b'}') {
                judge(format!("literal {name}"), fields);
            }
        }
    }
    read
}

#[test]
fn no_store_or_census_key_is_built_from_a_token() {
    let mut findings = Vec::new();
    let mut found: Vec<(String, String)> = Vec::new();
    for (module, text) in MODULES {
        let read = check(module, text);
        assert!(
            !read.found.is_empty(),
            "{module} is listed and holds nothing this test reads: the list is stale"
        );
        found.extend(read.found.into_iter().map(|item| (module.to_owned(), item)));
        findings.extend(read.findings);
    }
    assert!(
        findings.is_empty(),
        "a store or census key names a vendor token: {findings:#?}"
    );
    // Every named type and function is found, each in exactly one module.
    for item in TYPES.iter().map(|name| format!("type {name}")).chain(
        FUNCTIONS
            .iter()
            .map(|(owner, name)| format!("fn {owner}::{name}")),
    ) {
        let homes: Vec<&str> = found
            .iter()
            .filter(|(_, held)| *held == item)
            .map(|(module, _)| module.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(homes.len(), 1, "{item} is defined in {homes:?}");
    }
    // The census and the facets each build a key this test reads.
    for module in [
        "crates/pull/src/census_store.rs",
        "crates/api/src/facets.rs",
    ] {
        assert!(
            found
                .iter()
                .any(|(at, item)| at == module && item == "literal EntryKey"),
            "{module} builds no EntryKey any more: re-read the list"
        );
    }
}

/// Every `.rs` file under `dir`, in path order.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_module_list_is_closed_over_every_definition_of_a_key() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let listed: Vec<&str> = MODULES.iter().map(|(module, _)| *module).collect();
    let mut files = Vec::new();
    for krate in ["core", "store", "pull", "api"] {
        sources(&root.join("crates").join(krate).join("src"), &mut files);
    }
    assert!(files.len() > 100, "the walk found {} files", files.len());
    let mut strays = Vec::new();
    for path in files {
        let module = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        // A test module builds fixtures, not the store's keys: `api`'s
        // `saved_response_boundary_tests.rs` holds an `enum Contract` of its own.
        if listed.contains(&module.as_str()) || module.ends_with("tests.rs") {
            continue;
        }
        let code = code_only(&std::fs::read_to_string(&path).unwrap());
        let defines = TYPES.iter().any(|name| {
            ["struct", "enum", "type"]
                .iter()
                .any(|keyword| !word_positions(&code, &format!("{keyword} {name}")).is_empty())
        }) || FUNCTIONS.iter().any(|(_, name)| {
            !word_positions(&code, &format!("fn {name}(")).is_empty() && *name != "new"
        });
        if defines {
            strays.push(module);
        }
    }
    assert!(
        strays.is_empty(),
        "these files define a key type or builder this test does not read; list them: {strays:?}"
    );
}

/// A fixture module whose census key holds a token, and a key builder that
/// takes one: the checker must refuse both by name.
const TOKEN_FIXTURE: &str = r"
/// A census key. // a token in prose is not a field
pub struct EntryKey {
    pub contract: Option<Contract>,
    pub instrument_token: u32,
}
impl<'a> StorePath<'a> {
    pub fn for_key(vendor: Vendor, exchange_token: u64) -> Self { todo!() }
}
fn build(row: &Row) -> EntryKey {
    EntryKey { contract: None, instrument_token: row.security_id }
}
";

#[test]
fn a_token_field_in_a_fixture_key_builder_is_refused() {
    let read = check("fixture.rs", TOKEN_FIXTURE);
    assert_eq!(
        read.findings,
        vec![
            Finding {
                module: "fixture.rs".to_owned(),
                item: "type EntryKey".to_owned(),
                names: vec!["instrument_token".to_owned()],
            },
            Finding {
                module: "fixture.rs".to_owned(),
                item: "fn StorePath::for_key".to_owned(),
                names: vec!["exchange_token".to_owned()],
            },
            Finding {
                module: "fixture.rs".to_owned(),
                item: "literal EntryKey".to_owned(),
                names: vec!["instrument_token".to_owned(), "security_id".to_owned()],
            },
        ]
    );
    // The same module with the token taken out passes, so the checker refuses
    // the token and not the shape; prose about a token is never read.
    let clean = TOKEN_FIXTURE
        .replace("pub instrument_token: u32,", "")
        .replace("exchange_token: u64", "month: YearMonth")
        .replace("instrument_token: row.security_id", "");
    let read = check("fixture.rs", &clean);
    assert!(read.findings.is_empty(), "{:?}", read.findings);
    assert_eq!(read.found.len(), 3);
    // Every forbidden spelling is refused, in any case.
    for name in FORBIDDEN {
        assert_eq!(
            forbidden_in(&format!("a: {}", name.to_ascii_uppercase())),
            vec![name.to_owned()]
        );
    }
    assert_eq!(forbidden_in("kite_token"), vec!["kite_token".to_owned()]);
    assert!(forbidden_in("contract exchange segment symbol").is_empty());
    // A comment, a block comment and a string escape are handled.
    assert_eq!(
        code_only("a // token\nb /* token */ c \"x\\\"//y\""),
        "a \nb  c \"x\\\"//y\""
    );
}
