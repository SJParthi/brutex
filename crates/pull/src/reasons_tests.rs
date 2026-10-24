//! The READ-02 gate (R4-05): every code a source names has a row in
//! [`TABLE`], and every row's code is named by a source.
//!
//! The codes are collected FROM THEIR SOURCES, never from a copy kept here: a
//! type with its own list is called (`RunRefusal::CODES`, `Code::ALL`,
//! `flags::ALL`, `verdict_words::REASONS`, `HistoryEnds::CODE`); a type with
//! none has its `code()` match read out of its own file through
//! `include_str!`. The reader refuses, by name, an arm it cannot read and a
//! string literal it did not take as a code, so it can never pass by reading
//! fewer codes than the file holds.
//!
//! Every fixture is invented: the codes `zz_*` exist in no source.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use super::*;

/// The files whose code matches the gate reads.
struct Texts {
    causes: String,
    failkind: String,
    fetch: String,
    listing: String,
    run: String,
}

impl Texts {
    /// The files as this build compiled them.
    fn compiled() -> Self {
        Self {
            causes: include_str!("gdfl_import_causes.rs").to_owned(),
            failkind: include_str!("ingest_failkind.rs").to_owned(),
            fetch: include_str!("fetch.rs").to_owned(),
            listing: include_str!("listing.rs").to_owned(),
            run: include_str!("gdfl_run.rs").to_owned(),
        }
    }
}

const CAUSES_AT: &str = "crates/pull/src/gdfl_import_causes.rs";
const FAILKIND_AT: &str = "crates/pull/src/ingest_failkind.rs";
const RUN_AT: &str = "crates/pull/src/gdfl_run.rs";
const VERDICT_AT: &str = "crates/store/src/verdict.rs";
const FETCH_AT: &str = "crates/pull/src/fetch.rs";
const WORDS_AT: &str = "crates/pull/src/verdict_words.rs";

/// The store steps a row may name.
const STEPS: &str = include_str!("../../../config/store-steps.toml");

/// One source of codes: where its codes are written, and the codes.
#[derive(Debug, Clone)]
struct Source {
    at: String,
    codes: Vec<String>,
}

fn source(at: &str, codes: Vec<String>) -> Source {
    Source {
        at: at.to_owned(),
        codes,
    }
}

fn owned(codes: impl IntoIterator<Item = &'static str>) -> Vec<String> {
    codes.into_iter().map(str::to_owned).collect()
}

/// A line with its `//` comment cut off; a `//` inside a string literal is
/// not a comment.
fn uncommented(line: &str) -> &str {
    let mut quoted = false;
    let bytes = line.as_bytes();
    for (at, byte) in bytes.iter().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'/' if !quoted && bytes.get(at + 1) == Some(&b'/') => {
                return line.get(..at).unwrap_or("");
            }
            _ => {}
        }
    }
    line
}

/// The body of the function `header` under `owner` (an `impl` line, or the
/// header alone where `owner` is empty), its comments cut off.
fn body_of(text: &str, owner: &str, header: &str) -> Result<String, String> {
    let from = text
        .find(owner)
        .ok_or_else(|| format!("no `{owner}` in the file"))?;
    let rest = text.get(from..).unwrap_or("");
    let start = rest
        .find(header)
        .ok_or_else(|| format!("no `{header}` after `{owner}`"))?;
    let rest = rest.get(start + header.len()..).unwrap_or("");
    let mut depth = 1_i32;
    let mut out = String::new();
    for line in rest.lines() {
        let code = uncommented(line);
        for c in code.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return Ok(out);
            }
        }
        out.push_str(code);
        out.push('\n');
    }
    Err(format!("`{header}` never closes"))
}

/// Every string literal of `text`, in order.
fn literals(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The value of `pub const NAME: &str = "...";` in `text`.
fn constant(text: &str, name: &str) -> Option<String> {
    let needle = format!("pub const {name}: &str = \"");
    let at = text.find(&needle)? + needle.len();
    let rest = text.get(at..)?;
    Some(rest.get(..rest.find('"')?)?.to_owned())
}

/// What one `code()` match reads to: its codes, and the arms that hand the
/// code to another type's `code()`.
struct Arms {
    codes: Vec<String>,
    delegated: Vec<String>,
}

/// The codes of a `code()` match. An arm is a string literal, a constant of
/// `crate::listing` (read from that file), or another value's `code()`
/// (named in `delegated`); any other arm, a wildcard arm, and a string the
/// arms did not take, are refused by name.
fn arms(body: &str, listing: &str) -> Result<Arms, String> {
    let mut codes = Vec::new();
    let mut delegated = Vec::new();
    for line in body.lines() {
        let Some((lhs, rhs)) = line.split_once("=>") else {
            continue;
        };
        if lhs.trim() == "_" || lhs.trim().starts_with("_ ") {
            return Err(format!("a wildcard arm: `{}`", line.trim()));
        }
        let rhs = rhs.trim().trim_end_matches(',').trim();
        if let Some(literal) = rhs.strip_prefix('"') {
            let end = literal
                .find('"')
                .ok_or_else(|| format!("an unclosed string: `{}`", line.trim()))?;
            codes.push(literal.get(..end).unwrap_or("").to_owned());
        } else if let Some(name) = rhs.strip_prefix("crate::listing::") {
            codes.push(
                constant(listing, name)
                    .ok_or_else(|| format!("no `listing::{name}` constant: `{}`", line.trim()))?,
            );
        } else if rhs.ends_with(".code()") {
            delegated.push(rhs.to_owned());
        } else {
            return Err(format!("an arm this gate cannot read: `{}`", line.trim()));
        }
    }
    for literal in literals(body) {
        if !codes.contains(&literal) {
            return Err(format!(
                "a string the arms did not take as a code: `{literal}`"
            ));
        }
    }
    if codes.is_empty() {
        return Err("no arm read".to_owned());
    }
    Ok(Arms { codes, delegated })
}

/// The codes the cause functions hand to `Cause::vendor` and `Cause::open`:
/// each call's last argument is a string literal, or the variable `code`
/// (the import refusal's own code, collected from `ImportRefusal::code`).
/// Any other argument, and a string literal no call took, are refused.
fn cause_codes(body: &str) -> Result<Vec<String>, String> {
    let mut codes = Vec::new();
    for call in ["Cause::vendor(", "Cause::open("] {
        for (at, _) in body.match_indices(call) {
            let rest = body.get(at + call.len()..).unwrap_or("");
            let end = rest
                .find(')')
                .ok_or_else(|| format!("an unclosed `{call}`"))?;
            let args = rest.get(..end).unwrap_or("");
            let last = args.rsplit(',').next().unwrap_or("").trim();
            if let Some(literal) = last.strip_prefix('"') {
                codes.push(literal.trim_end_matches('"').to_owned());
            } else if last != "code" {
                return Err(format!("a cause this gate cannot read: `{call}{args})`"));
            }
        }
    }
    for literal in literals(body) {
        if !codes.contains(&literal) {
            return Err(format!("a string no cause took as a code: `{literal}`"));
        }
    }
    Ok(codes)
}

/// Every source of codes, read from `texts`.
fn sources_from(texts: &Texts) -> Result<Vec<Source>, String> {
    let refusal = arms(
        &body_of(
            &texts.causes,
            "impl ImportRefusal {",
            "pub const fn code(&self) -> &'static str {",
        )?,
        &texts.listing,
    )?;
    // `Store { kind, .. } => kind.code()`: the failure kind's code, which the
    // FailKind source below collects. Any other hand-off is unread.
    if refusal.delegated != ["kind.code()"] {
        return Err(format!(
            "ImportRefusal::code hands off to {:?}; only `kind.code()` (FailKind) is read",
            refusal.delegated
        ));
    }
    let mut causes = Vec::new();
    for header in [
        "pub const fn cm_cause(why: &CmRefusal) -> Cause {",
        "pub const fn nfo_cause(why: &NfoRefusal) -> Cause {",
        "pub const fn import_cause(why: &ImportRefusal) -> Cause {",
    ] {
        causes.extend(cause_codes(&body_of(&texts.causes, "", header)?)?);
    }
    let failkind = arms(
        &body_of(
            &texts.failkind,
            "impl FailKind {",
            "pub const fn code(self) -> &'static str {",
        )?,
        &texts.listing,
    )?;
    let run = arms(
        &body_of(
            &texts.run,
            "impl RunRefusal {",
            "pub const fn code(&self) -> &'static str {",
        )?,
        &texts.listing,
    )?;
    let listed: BTreeSet<&str> = crate::gdfl_run::RunRefusal::CODES.into_iter().collect();
    let matched: BTreeSet<&str> = run.codes.iter().map(String::as_str).collect();
    if listed != matched {
        return Err(format!(
            "RunRefusal::CODES and RunRefusal::code disagree: listed only {:?}, matched only {:?}",
            listed.difference(&matched).collect::<Vec<_>>(),
            matched.difference(&listed).collect::<Vec<_>>()
        ));
    }
    let mut row_causes = arms(
        &body_of(
            &texts.fetch,
            "impl RowCause {",
            "pub const fn code(self) -> &'static str {",
        )?,
        &texts.listing,
    )?
    .codes;
    row_causes.push(crate::fetch::HistoryEnds::CODE.to_owned());
    Ok(vec![
        source(&format!("{CAUSES_AT}: ImportRefusal::code"), refusal.codes),
        source(
            &format!("{CAUSES_AT}: cm_cause, nfo_cause, import_cause"),
            causes,
        ),
        source(&format!("{FAILKIND_AT}: FailKind::code"), failkind.codes),
        source(
            &format!("{RUN_AT}: RunRefusal::CODES"),
            owned(crate::gdfl_run::RunRefusal::CODES),
        ),
        source(
            &format!("{VERDICT_AT}: Code::ALL"),
            owned(store::verdict::Code::ALL.map(store::verdict::Code::word)),
        ),
        source(
            &format!("{VERDICT_AT}: flags::ALL"),
            owned(store::verdict::flags::ALL.map(|(_, word)| word)),
        ),
        source(
            &format!("{FETCH_AT}: RowCause::code, HistoryEnds::CODE"),
            row_causes,
        ),
        source(
            &format!("{WORDS_AT}: REASONS"),
            owned(crate::verdict_words::REASONS.map(|(_, word)| word)),
        ),
    ])
}

/// Every source, as this build compiled them.
fn sources() -> Vec<Source> {
    sources_from(&Texts::compiled()).expect("every source reads")
}

/// The ids of `config/store-steps.toml`, with whether each is withdrawn.
fn steps() -> BTreeMap<String, bool> {
    let mut out = BTreeMap::new();
    let mut id: Option<String> = None;
    for line in STEPS.lines() {
        if line.starts_with("[[") {
            id = None;
        } else if let Some(rest) = line.strip_prefix("id = \"") {
            let found = rest.trim_end_matches('"').to_owned();
            out.insert(found.clone(), false);
            id = Some(found);
        } else if line.trim() == "state = \"withdrawn\""
            && let Some(at) = &id
        {
            out.insert(at.clone(), true);
        }
    }
    out
}

/// Why a text is not plain words, or `None`.
fn not_plain(text: &str) -> Option<&'static str> {
    if text.trim().is_empty() {
        Some("is empty")
    } else if text != text.trim() {
        Some("has space at an end")
    } else if text.ends_with(['.', ';', ':']) {
        Some("ends with a stop; a page adds its own")
    } else if text.contains(';') && !text.contains("; ") {
        Some("holds a ';' a title would read as the control's")
    } else if text.contains('`') || text.contains("::") {
        Some("holds code punctuation")
    } else if text
        .split_whitespace()
        .any(|word| word.contains('_') && word.chars().any(|c| c.is_ascii_lowercase()))
    {
        Some("holds an identifier")
    } else {
        None
    }
}

/// The credential field names a text must not carry the shape of, as this
/// crate names them: the token field every feed reads, and the second field of
/// each two-secret feed (`vendor::Auth::key_field`). The charter's other field
/// names are `api`'s, which passes them to the same detector.
fn credential_fields() -> Vec<&'static str> {
    let mut fields = vec!["access-token"];
    for feed in crate::vendor::Feed::ALL {
        if let crate::vendor::Transport::Http(spec) = &feed.descriptor().transport
            && let Some(field) = spec.auth.key_field
            && !fields.contains(&field)
        {
            fields.push(field);
        }
    }
    fields
}

/// Every way `table` fails the gate against `sources`, each naming the code
/// and, for a missing row, every source file that names the code.
fn problems(table: &[Reason], sources: &[Source]) -> Vec<String> {
    let mut out = Vec::new();
    let mut named: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for source in sources {
        for code in &source.codes {
            let at = named.entry(code.as_str()).or_default();
            if !at.contains(&source.at.as_str()) {
                at.push(source.at.as_str());
            }
        }
    }
    let mut rows: BTreeMap<&str, usize> = BTreeMap::new();
    for reason in table {
        *rows.entry(reason.code).or_default() += 1;
    }
    for (code, at) in &named {
        if !rows.contains_key(code) {
            out.push(format!("no row for `{code}` (named by {})", at.join("; ")));
        }
    }
    let known = steps();
    for reason in table {
        let code = reason.code;
        if !named.contains_key(code) {
            out.push(format!("row `{code}` names a code no source names"));
        }
        if rows.get(code).copied().unwrap_or(0) > 1
            && !out
                .iter()
                .any(|p| p.starts_with(&format!("two rows for `{code}`")))
        {
            out.push(format!("two rows for `{code}`"));
        }
        for (what, text) in [("sentence", reason.sentence), ("action", reason.action)] {
            if let Some(why) = not_plain(text) {
                out.push(format!("row `{code}`: its {what} {why}"));
            }
            if brutex_core::secret_shape::shape_of(text, &credential_fields()).is_some() {
                out.push(format!("row `{code}`: its {what} has a credential's shape"));
            }
        }
        match reason.control {
            Control::StoreStep(id) => match known.get(id) {
                None => out.push(format!(
                    "row `{code}`: store step {id} is not in config/store-steps.toml"
                )),
                Some(true) => out.push(format!("row `{code}`: store step {id} is withdrawn")),
                Some(false) => {}
            },
            Control::Operator(what) => {
                if let Some(why) = not_plain(what) {
                    out.push(format!("row `{code}`: its operator step {why}"));
                }
            }
            Control::Repair | Control::RetryRefused | Control::Nothing => {}
        }
    }
    out
}

fn without(code: &str) -> Vec<Reason> {
    TABLE.iter().copied().filter(|r| r.code != code).collect()
}

fn with(extra: Reason) -> Vec<Reason> {
    let mut table = TABLE.to_vec();
    table.push(extra);
    table
}

fn invented(code: &'static str) -> Reason {
    row(code, "an invented code", "nothing to do", Control::Nothing)
}

/// The distinct codes every source names.
fn every_code(sources: &[Source]) -> BTreeSet<String> {
    sources
        .iter()
        .flat_map(|s| s.codes.iter().cloned())
        .collect()
}

// ── The gate ───────────────────────────────────────────────────────────────

#[test]
fn every_code_has_a_row_and_every_row_a_code() {
    let sources = sources();
    // A reader that stopped reading would pass on nothing: each source holds
    // at least the codes it held when this gate was written.
    let floors = [28, 46, 12, 24, 18, 22, 11, 37];
    assert_eq!(sources.len(), floors.len());
    for (source, floor) in sources.iter().zip(floors) {
        assert!(
            source.codes.len() >= floor,
            "{} gave {} code(s), fewer than its {floor}",
            source.at,
            source.codes.len()
        );
    }
    let found = problems(TABLE, &sources);
    assert!(found.is_empty(), "the reasons gate:\n{}", found.join("\n"));
    assert_eq!(
        every_code(&sources).len(),
        TABLE.len(),
        "one row per distinct code"
    );
}

#[test]
fn the_gate_reads_codes_from_their_sources_not_a_copy() {
    // A code a merged package adds to a source with no row: named, with the
    // file it was read from. Each read source is changed in memory only.
    let header = "pub const fn code(&self) -> &'static str {\n        match self {";
    let mut texts = Texts::compiled();
    texts.causes = texts.causes.replacen(
        header,
        &format!("{header}\n            Self::Invented {{ .. }} => \"zz_invented_refusal\","),
        1,
    );
    let found = problems(TABLE, &sources_from(&texts).unwrap());
    assert_eq!(
        found,
        [format!(
            "no row for `zz_invented_refusal` (named by {CAUSES_AT}: ImportRefusal::code)"
        )]
    );

    let mut texts = Texts::compiled();
    texts.causes = texts.causes.replacen(
        "CmRefusal::FolderUnknown => Cause::vendor(\"folder_unknown\"),",
        "CmRefusal::FolderUnknown => Cause::vendor(\"folder_unknown\"),\n        CmRefusal::Invented => Cause::open(Stage::File, \"zz_invented_cause\"),",
        1,
    );
    let found = problems(TABLE, &sources_from(&texts).unwrap());
    assert_eq!(
        found,
        [format!(
            "no row for `zz_invented_cause` (named by {CAUSES_AT}: cm_cause, nfo_cause, import_cause)"
        )]
    );

    let mut texts = Texts::compiled();
    texts.failkind = texts.failkind.replacen(
        "Self::MonthBusy => \"month_busy\",",
        "Self::MonthBusy => \"month_busy\",\n            Self::Invented => \"zz_invented_kind\",",
        1,
    );
    assert_eq!(
        problems(TABLE, &sources_from(&texts).unwrap()),
        [format!(
            "no row for `zz_invented_kind` (named by {FAILKIND_AT}: FailKind::code)"
        )]
    );

    let mut texts = Texts::compiled();
    texts.fetch = texts.fetch.replacen(
        "Self::Timestamp => \"timestamp_refused\",",
        "Self::Timestamp => \"timestamp_refused\",\n            Self::Invented => \"zz_invented_row_cause\",",
        1,
    );
    assert_eq!(
        problems(TABLE, &sources_from(&texts).unwrap()),
        [format!(
            "no row for `zz_invented_row_cause` (named by {FETCH_AT}: RowCause::code, HistoryEnds::CODE)"
        )]
    );

    // The listing constant a RowCause arm names is read from its own file:
    // changed there, the code the gate looks for changes with it.
    let mut texts = Texts::compiled();
    texts.listing = texts.listing.replacen(
        "pub const BEFORE_LISTING: &str = \"before_listing\";",
        "pub const BEFORE_LISTING: &str = \"zz_before_listing_renamed\";",
        1,
    );
    let found = problems(TABLE, &sources_from(&texts).unwrap());
    assert!(
        found.contains(&format!(
            "no row for `zz_before_listing_renamed` (named by {FETCH_AT}: RowCause::code, HistoryEnds::CODE)"
        )),
        "{found:?}"
    );

    // A row nobody names, and a row taken away.
    assert_eq!(
        problems(&with(invented("zz_orphan")), &sources()),
        ["row `zz_orphan` names a code no source names"]
    );
    assert_eq!(
        problems(&without("needs_month_generation"), &sources()),
        [format!(
            "no row for `needs_month_generation` (named by {FAILKIND_AT}: FailKind::code)"
        )]
    );
}

#[test]
fn an_arm_the_gate_cannot_read_is_refused_by_name_never_skipped() {
    let header = "pub const fn code(self) -> &'static str {\n        match self {";
    let refused = |edit: &str| {
        let mut texts = Texts::compiled();
        texts.failkind =
            texts
                .failkind
                .replacen(header, &format!("{header}\n            {edit}"), 1);
        sources_from(&texts).err()
    };
    assert_eq!(
        refused("Self::Invented => invented_code(),"),
        Some("an arm this gate cannot read: `Self::Invented => invented_code(),`".to_owned())
    );
    assert_eq!(
        refused("_ => \"zz_wild\","),
        Some("a wildcard arm: `_ => \"zz_wild\",`".to_owned())
    );
    assert_eq!(
        refused("Self::Invented => crate::listing::NO_SUCH,"),
        Some(
            "no `listing::NO_SUCH` constant: `Self::Invented => crate::listing::NO_SUCH,`"
                .to_owned()
        )
    );
    // A code written as a value the arms do not return (a constant built
    // inside the match) is a string the arms did not take.
    assert_eq!(
        refused("Self::Invented => { let _ = \"zz_hidden\"; NAME }"),
        Some(
            "an arm this gate cannot read: `Self::Invented => { let _ = \"zz_hidden\"; NAME }`"
                .to_owned()
        )
    );
    // A commented-out arm is not a code: it names nothing the build returns.
    let mut texts = Texts::compiled();
    texts.failkind = texts.failkind.replacen(
        header,
        &format!("{header}\n            // Self::Retired => \"zz_retired\","),
        1,
    );
    assert!(problems(TABLE, &sources_from(&texts).unwrap()).is_empty());
    // A cause call whose code is neither a literal nor `code`.
    let mut texts = Texts::compiled();
    texts.causes = texts.causes.replacen(
        "Cause::vendor(\"folder_unknown\")",
        "Cause::vendor(FOLDER)",
        1,
    );
    assert_eq!(
        sources_from(&texts).err(),
        Some("a cause this gate cannot read: `Cause::vendor(FOLDER)`".to_owned())
    );
    // A hand-off ImportRefusal::code makes to a type the gate does not read.
    let mut texts = Texts::compiled();
    texts.causes = texts.causes.replacen(
        "Self::Store { kind, .. } => kind.code(),",
        "Self::Store { kind, .. } => kind.code(),\n            Self::Other { other, .. } => other.code(),",
        1,
    );
    assert!(
        sources_from(&texts)
            .err()
            .is_some_and(|why| why.contains("only `kind.code()` (FailKind) is read")),
    );
    // A function the gate reads that is gone is named, never read as empty.
    let mut texts = Texts::compiled();
    texts.fetch = texts
        .fetch
        .replacen("impl RowCause {", "impl RowCauseGone {", 1);
    assert_eq!(
        sources_from(&texts).err(),
        Some("no `impl RowCause {` in the file".to_owned())
    );
}

#[test]
fn run_refusal_codes_and_its_match_are_one_list() {
    let mut texts = Texts::compiled();
    texts.run = texts.run.replacen(
        "Self::RunBusy { .. } => \"run_busy\",",
        "Self::RunBusy { .. } => \"zz_run_busy_renamed\",",
        1,
    );
    let why = sources_from(&texts).err().unwrap();
    assert!(why.contains("listed only [\"run_busy\"]"), "{why}");
    assert!(
        why.contains("matched only [\"zz_run_busy_renamed\"]"),
        "{why}"
    );
}

// ── Extreme cases of the gate (at least 8; two combinations; one no list
//    names) ────────────────────────────────────────────────────────────────

#[test]
fn an_empty_table_names_every_code_and_no_row() {
    let sources = sources();
    let found = problems(&[], &sources);
    assert_eq!(found.len(), every_code(&sources).len());
    assert!(found.iter().all(|p| p.starts_with("no row for `")));
}

#[test]
fn a_one_row_table_names_every_other_code() {
    let sources = sources();
    let one = [*of("needs_month_generation").unwrap()];
    let found = problems(&one, &sources);
    assert_eq!(found.len(), every_code(&sources).len() - 1);
    assert!(!found.iter().any(|p| p.contains("`needs_month_generation`")));
}

#[test]
fn a_duplicate_row_is_named_once() {
    let twice = with(*of("owed").unwrap());
    assert_eq!(problems(&twice, &sources()), ["two rows for `owed`"]);
}

#[test]
fn a_missing_row_an_orphan_and_a_duplicate_at_once_are_each_named() {
    // COMBINATION: three faults in one table; none hides another.
    let mut table = without("store_io");
    table.push(invented("zz_orphan"));
    table.push(*of("match").unwrap());
    let found: BTreeSet<String> = problems(&table, &sources()).into_iter().collect();
    let want: BTreeSet<String> = [
        format!("no row for `store_io` (named by {FAILKIND_AT}: FailKind::code)"),
        "row `zz_orphan` names a code no source names".to_owned(),
        "two rows for `match`".to_owned(),
    ]
    .into_iter()
    .collect();
    assert_eq!(found, want);
}

#[test]
fn a_code_two_sources_share_names_both_when_its_row_is_gone() {
    // OUT OF THE BOX: one word, two meanings close enough to share a row
    // (`disk_full` from the importer's run and from the store's door). The
    // gate names every file a fixer must read, not the first it met.
    assert_eq!(
        problems(&without("disk_full"), &sources()),
        [format!(
            "no row for `disk_full` (named by {CAUSES_AT}: ImportRefusal::code; {FAILKIND_AT}: FailKind::code)"
        )]
    );
    assert_eq!(
        problems(&without("month_busy"), &sources()),
        [format!(
            "no row for `month_busy` (named by {FAILKIND_AT}: FailKind::code; {WORDS_AT}: REASONS)"
        )]
    );
}

#[test]
fn a_row_in_code_words_or_with_its_own_stop_is_refused() {
    let bad = [
        (
            row("owed", "", "x", Control::Nothing),
            "row `owed`: its sentence is empty",
        ),
        (
            row("owed", "see `owed`", "x", Control::Nothing),
            "row `owed`: its sentence holds code punctuation",
        ),
        (
            row("owed", "the month_superseded day", "x", Control::Nothing),
            "row `owed`: its sentence holds an identifier",
        ),
        (
            row("owed", "a sentence.", "x", Control::Nothing),
            "row `owed`: its sentence ends with a stop; a page adds its own",
        ),
        (
            row("owed", " padded", "x", Control::Nothing),
            "row `owed`: its sentence has space at an end",
        ),
        (
            row("owed", "a;b", "x", Control::Nothing),
            "row `owed`: its sentence holds a ';' a title would read as the control's",
        ),
        (
            row("owed", "words", "pull::fetch it", Control::Nothing),
            "row `owed`: its action holds code punctuation",
        ),
        (
            row("owed", "words", "x", Control::Operator("")),
            "row `owed`: its operator step is empty",
        ),
    ];
    for (reason, want) in bad {
        let mut table = without("owed");
        table.push(reason);
        assert_eq!(problems(&table, &sources()), [want.to_owned()]);
    }
}

#[test]
fn a_store_step_the_file_lacks_or_withdrew_is_refused() {
    let mut table = without("owed");
    table.push(row("owed", "words", "x", Control::StoreStep("S-999")));
    assert_eq!(
        problems(&table, &sources()),
        ["row `owed`: store step S-999 is not in config/store-steps.toml"]
    );
    // S-20 is withdrawn in the file: a row may not send anyone to it.
    assert_eq!(steps().get("S-20"), Some(&true));
    let mut table = without("owed");
    table.push(row("owed", "words", "x", Control::StoreStep("S-20")));
    assert_eq!(
        problems(&table, &sources()),
        ["row `owed`: store step S-20 is withdrawn"]
    );
    assert_eq!(steps().get("S-36"), Some(&false));
}

#[test]
fn a_secret_shaped_text_is_refused_with_its_row() {
    // COMBINATION: a credential-shaped action on a row that is also an
    // orphan: both named.
    let mut table = TABLE.to_vec();
    table.push(row(
        "zz_orphan",
        "words",
        "paste Zz9Yy8Xx7Ww6Vv5Uu4Tt3 here",
        Control::Nothing,
    ));
    let found: BTreeSet<String> = problems(&table, &sources()).into_iter().collect();
    assert!(found.contains("row `zz_orphan` names a code no source names"));
    assert!(found.contains("row `zz_orphan`: its action has a credential's shape"));
    let mut table = without("owed");
    table.push(row("owed", "see /a/b/c/api-key", "x", Control::Nothing));
    assert_eq!(
        problems(&table, &sources()),
        ["row `owed`: its sentence has a credential's shape"]
    );
}

#[test]
fn every_store_step_a_row_names_is_a_pending_or_done_step() {
    let named: Vec<&str> = TABLE.iter().filter_map(|r| r.control.step()).collect();
    assert!(!named.is_empty());
    for id in named {
        assert_eq!(steps().get(id), Some(&false), "{id}");
    }
}

// ── The rows themselves ────────────────────────────────────────────────────

#[test]
fn the_acceptance_sentence_reads_as_written() {
    let row = of("needs_month_generation").unwrap();
    assert_eq!(
        title(row),
        "needs_month_generation: the month already passed this day; press Repair"
    );
    for code in [
        "repair_in_open_month",
        "month_superseded",
        "bars_differ_held",
        "repair_refused",
        "repair_impossible",
    ] {
        let row = of(code).unwrap();
        assert!(title(row).starts_with(&format!("{code}: ")), "{code}");
    }
    assert_eq!(of("month_superseded").unwrap().control, Control::Repair);
    assert_eq!(
        of("absent_from_listing").unwrap().control.label(),
        "Retry refused"
    );
    assert_eq!(
        title(of("symbol_not_in_use").unwrap()),
        "symbol_not_in_use: the file is named for a symbol the exchange was not using that day; store step S-36"
    );
    assert_eq!(
        title(of("disk_full").unwrap()),
        "disk_full: the store's disk is full, so nothing more could be written; operator: free disk space"
    );
    assert_eq!(
        title(of("match").unwrap()),
        "match: every unit the store holds equals the source's, and none is extra; no action"
    );
}

#[test]
fn each_control_has_its_label_kind_and_step() {
    let cases = [
        (Control::Repair, "press Repair", "repair", None),
        (
            Control::RetryRefused,
            "Retry refused",
            "retry_refused",
            None,
        ),
        (
            Control::StoreStep("S-36"),
            "store step S-36",
            "store_step",
            Some("S-36"),
        ),
        (
            Control::Operator("free disk space"),
            "operator: free disk space",
            "operator",
            None,
        ),
        (Control::Nothing, "no action", "none", None),
    ];
    for (control, label, kind, step) in cases {
        assert_eq!(control.label(), label);
        assert_eq!(control.kind(), kind);
        assert_eq!(control.step(), step);
        assert!(
            TABLE.iter().any(|r| r.control.kind() == kind),
            "{kind} is used"
        );
    }
}

#[test]
fn of_finds_every_row_and_nothing_else_in_one_probe() {
    for (at, reason) in TABLE.iter().enumerate() {
        let found = of(reason.code).unwrap();
        assert!(
            std::ptr::eq(found, TABLE.get(at).unwrap()),
            "{}",
            reason.code
        );
    }
    for unknown in [
        "",
        "zz_unknown",
        "NEEDS_MONTH_GENERATION",
        "needs_month_generation ",
    ] {
        assert_eq!(of(unknown), None, "{unknown:?}");
    }
    // p50/p99/max over n = 200 sequential lookups, half known and half not.
    let mut took = Vec::with_capacity(200);
    for n in 0..200 {
        let code = if n % 2 == 0 {
            TABLE[n % TABLE.len()].code
        } else {
            "zz_unknown_code"
        };
        let start = Instant::now();
        let found = of(code);
        took.push(start.elapsed().as_nanos());
        assert_eq!(found.is_some(), n % 2 == 0);
    }
    took.sort_unstable();
    let (p50, p99, max) = (took[99], took[197], took[199]);
    eprintln!("reasons::of n=200 p50={p50}ns p99={p99}ns max={max}ns");
    assert!(p99 < 1_000_000, "one hash probe took {p99} ns at p99");
}

// ── The served body ────────────────────────────────────────────────────────

#[test]
fn the_body_is_the_whole_table_under_its_build_and_digest() {
    let served = body("0123456789abcdef0123456789abcdef01234567");
    let doc: serde_json::Value = serde_json::from_slice(&served.bytes).unwrap();
    assert_eq!(doc["schema"], SCHEMA);
    assert_eq!(doc["build"], "0123456789abcdef0123456789abcdef01234567");
    assert_eq!(doc["count"], TABLE.len());
    assert_eq!(served.rows, TABLE.len());
    // The cut is the table's own longest title: no title it holds is cut.
    let longest = TABLE
        .iter()
        .map(|r| title(r).chars().count())
        .max()
        .unwrap();
    assert_eq!(doc["title_cut"], longest);
    assert_eq!(title_cut(), longest);
    assert!(
        TABLE
            .iter()
            .all(|r| title(r).chars().count() <= title_cut())
    );
    assert!(
        TABLE
            .iter()
            .any(|r| title(r).chars().count() == title_cut())
    );
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), TABLE.len());
    for (got, reason) in rows.iter().zip(TABLE) {
        assert_eq!(got["code"], reason.code);
        assert_eq!(got["sentence"], reason.sentence, "whole, never cut");
        assert_eq!(got["action"], reason.action);
        assert_eq!(got["control"], reason.control.label());
        assert_eq!(got["control_kind"], reason.control.kind());
        assert_eq!(got["step"], serde_json::json!(reason.control.step()));
        assert_eq!(got["title"], title(reason));
    }
    let digest = brutex_core::blake3::hash(&served.bytes);
    let digits = b"0123456789abcdef";
    let hex: String = digest
        .iter()
        .take(8)
        .flat_map(|b| [digits[usize::from(b >> 4)], digits[usize::from(b & 15)]])
        .map(char::from)
        .collect();
    assert_eq!(
        served.etag,
        format!("\"0123456789abcdef0123456789abcdef01234567.{hex}\"")
    );
    assert_eq!(hex.len(), 16);
}

#[test]
fn the_body_is_the_same_bytes_every_time_and_its_tag_moves_with_its_build() {
    let once = body("unstamped");
    let again = body("unstamped");
    assert_eq!(once, again, "same build, same bytes, same tag");
    assert!(once.etag.starts_with("\"unstamped."));
    let other = body("fedcba9876543210fedcba9876543210fedcba98");
    assert_ne!(once.etag, other.etag);
    assert_ne!(once.bytes, other.bytes);
    // An unstamped build's tag still names its bytes: two unstamped tags are
    // equal only when the bytes are.
    let digest_part = |etag: &str| etag.rsplit('.').next().unwrap().to_owned();
    assert_ne!(digest_part(&once.etag), digest_part(&other.etag));
}

#[test]
fn a_long_sentence_is_served_whole() {
    // The longest sentence in the table, served byte for byte (a page cuts
    // a title; the table never does).
    let longest = TABLE.iter().max_by_key(|r| r.sentence.len()).unwrap();
    assert!(longest.sentence.len() > 100, "{}", longest.sentence.len());
    let served = body("unstamped");
    let doc: serde_json::Value = serde_json::from_slice(&served.bytes).unwrap();
    let found = doc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["code"] == longest.code)
        .unwrap();
    assert_eq!(found["sentence"], longest.sentence);
}

#[test]
fn building_the_body_is_one_pass_over_the_table() {
    // The cold half of the route: what the first request pays, once per
    // process. p50/p99/max over n = 200 sequential builds.
    let mut took = Vec::with_capacity(200);
    let mut bytes = 0;
    for _ in 0..200 {
        let start = Instant::now();
        let served = body("0123456789abcdef0123456789abcdef01234567");
        took.push(start.elapsed().as_nanos());
        bytes = served.bytes.len();
    }
    took.sort_unstable();
    let (p50, p99, max) = (took[99], took[197], took[199]);
    eprintln!(
        "reasons::body n=200 rows={} bytes={bytes} p50={p50}ns p99={p99}ns max={max}ns",
        TABLE.len()
    );
    assert!(p99 < 50_000_000, "building the body took {p99} ns at p99");
}
