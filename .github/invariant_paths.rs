//! Checks the document's named tests against their explicitly recorded files.
//! The source declaration index is supplied by Gate 10's existing scanner.

use std::collections::HashSet;
use std::process::ExitCode;

#[derive(Debug)]
struct Span<'a> {
    start: usize,
    end: usize,
    value: &'a str,
}

fn code_spans(line: &str) -> Result<Vec<Span<'_>>, String> {
    let mut ticks = line.match_indices('`');
    let mut spans = Vec::new();
    while let Some((start, _)) = ticks.next() {
        let (end, _) = ticks
            .next()
            .ok_or_else(|| "unclosed code span in a path-qualified proof".to_owned())?;
        spans.push(Span {
            start,
            end: end + 1,
            value: &line[start + 1..end],
        });
    }
    Ok(spans)
}

fn symbol(value: &str) -> Option<&str> {
    let value = value.strip_suffix("()").unwrap_or(value);
    if value.split("::").all(|part| {
        let mut bytes = part.bytes();
        bytes
            .next()
            .is_some_and(|first| first == b'_' || first.is_ascii_lowercase())
            && bytes.all(|byte| byte == b'_' || byte.is_ascii_lowercase() || byte.is_ascii_digit())
    }) {
        value.rsplit("::").next()
    } else {
        None
    }
}

fn joins_names(gap: &str) -> bool {
    let words: String = gap
        .chars()
        .map(|character| match character {
            ',' | ';' | '&' => ' ',
            other => other,
        })
        .collect();
    words
        .split_whitespace()
        .all(|word| matches!(word, "and" | "or"))
}

/// A tracked source root a proof may name. `.github/` holds the CI tools, whose
/// tests prove the CIG rows (D-1114); before it was accepted here those rows were
/// read by nothing.
fn is_source_path(path: &str) -> bool {
    (path.starts_with("crates/") || path.starts_with(".github/")) && path.ends_with(".rs")
}

fn references(line: &str) -> Result<Vec<(String, String)>, String> {
    if !line.starts_with('|') || !(line.contains(" in `crates/") || line.contains(" in `.github/"))
    {
        return Ok(Vec::new());
    }
    let spans = code_spans(line)?;
    let mut references = Vec::new();
    for (index, path) in spans.iter().enumerate() {
        if !is_source_path(path.value) || index == 0 {
            continue;
        }
        let mut name_index = index - 1;
        let before = &spans[name_index];
        if line[before.end..path.start].trim() != "in" {
            continue;
        }
        if path
            .value
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(format!("noncanonical source path: {}", path.value));
        }
        loop {
            let name = &spans[name_index];
            let function = symbol(name.value)
                .ok_or_else(|| format!("noncanonical test name: {}", name.value))?;
            references.push((path.value.to_owned(), function.to_owned()));
            if name_index == 0 {
                break;
            }
            let earlier = &spans[name_index - 1];
            let gap = &line[earlier.end..name.start];
            if !joins_names(gap)
                || symbol(earlier.value).is_none()
                || (gap.contains(';') && earlier.value.contains("::"))
            {
                break;
            }
            name_index -= 1;
        }
    }
    Ok(references)
}

/// Every backticked `a::b::c`-shaped token in a line, read the way gate 10's
/// `grep -oE` reads them: leftmost first, and a closing backtick around content of
/// another shape may open the next token.
fn qualified_tokens(line: &str) -> Vec<&str> {
    let segment = |part: &str| {
        let mut bytes = part.bytes();
        bytes
            .next()
            .is_some_and(|first| first == b'_' || first.is_ascii_lowercase())
            && bytes.all(|byte| byte == b'_' || byte.is_ascii_lowercase() || byte.is_ascii_digit())
    };
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(open) = line[from..].find('`').map(|at| from + at) {
        let Some(close) = line[open + 1..].find('`').map(|at| open + 1 + at) else {
            break;
        };
        let value = &line[open + 1..close];
        if value.split("::").count() >= 3 && value.split("::").all(segment) {
            out.push(value);
            from = close + 1;
        } else {
            from = close;
        }
    }
    out
}

/// The crate directories the declarations come from: `crates/<name>/...`.
fn crate_names<'a>(declarations: &[(&'a str, &'a str)]) -> HashSet<&'a str> {
    declarations
        .iter()
        .filter_map(|(path, _)| path.strip_prefix("crates/")?.split('/').next())
        .collect()
}

/// Does a MODULE-FIRST token resolve? D-1114.
///
/// Gate 10 reads the first segment of `a::b::name` as a crate. When it is not one,
/// the token used to be counted as "pending" and checked by nothing -- 52 rows'
/// worth, every one a module path such as `server::tests::x` or
/// `vwap::tests::y`, so any of those tests could be renamed or deleted with the
/// gate green. Such a token now resolves only when a tracked source file whose path
/// names the module -- `.../server.rs`, or a file under `.../server/` -- declares
/// `name`.
///
/// A module mounted with `#[path]` -- `server::store_wire` living in
/// `store_wire.rs` -- is not named by its file's path, so the MODULES table,
/// `FILE<TAB>a::b` per mounting as `source_scan modules` prints it, is read as
/// well: the token resolves when a file declaring `name` is mounted at a module
/// path some tail of which begins the token's own module segments.
fn module_first_resolves(
    token: &str,
    declarations: &[(&str, &str)],
    modules: &[(&str, &str)],
) -> bool {
    let segments: Vec<&str> = token.split("::").collect();
    let (Some(name), Some(module)) = (segments.last(), segments.first()) else {
        return false;
    };
    let wanted = &segments[..segments.len() - 1];
    declarations.iter().any(|(path, function)| {
        function == name
            && (path
                .strip_prefix("crates/")
                .into_iter()
                .flat_map(|rest| rest.split('/').skip(1))
                .any(|component| component.strip_suffix(".rs").unwrap_or(component) == *module)
                || modules.iter().any(|(file, mounted)| {
                    file == path && {
                        let mounted: Vec<&str> =
                            mounted.split("::").filter(|s| !s.is_empty()).collect();
                        (0..mounted.len()).any(|from| wanted.starts_with(&mounted[from..]))
                    }
                }))
    })
}

#[cfg(test)]
fn verify(document: &str, declarations: &str) -> Result<usize, String> {
    verify_with(document, declarations, "")
}

fn verify_with(document: &str, declarations: &str, modules: &str) -> Result<usize, String> {
    let modules: Vec<(&str, &str)> = modules
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let pairs: Vec<(&str, &str)> = declarations
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let crates = crate_names(&pairs);
    let declarations: HashSet<&str> = declarations
        .lines()
        .filter(|line| !line.is_empty())
        .collect();
    let mut checked = 0;
    let mut refusals = Vec::new();
    for (index, line) in document.lines().enumerate() {
        if line.starts_with('|') {
            for token in qualified_tokens(line) {
                let first = token.split("::").next().unwrap_or("");
                if crates.contains(first) {
                    continue; // crate-first: gate 10's own table checks it
                }
                checked += 1;
                if !module_first_resolves(token, &pairs, &modules) {
                    refusals.push(format!(
                        "line {}: {token} names no crate, and no tracked file of a module `{first}` declares its function",
                        index + 1
                    ));
                }
            }
        }
        match references(line) {
            Ok(references) => {
                for (path, function) in references {
                    checked += 1;
                    if !declarations.contains(format!("{path}\t{function}").as_str()) {
                        refusals.push(format!(
                            "line {}: {function} is not declared in tracked {path}",
                            index + 1
                        ));
                    }
                }
            }
            Err(why) => refusals.push(format!("line {}: {why}", index + 1)),
        }
    }
    if refusals.is_empty() {
        Ok(checked)
    } else {
        Err(refusals.join("\n"))
    }
}

fn run() -> Result<usize, String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let (document, declarations, modules) = match arguments.as_slice() {
        [document, declarations] => (document, declarations, None),
        [document, declarations, modules] => (document, declarations, Some(modules)),
        _ => {
            return Err(
                "usage: invariant-paths DOCUMENT PATH_DECLARATIONS [MODULE_PATHS]".to_owned(),
            );
        }
    };
    let document = std::fs::read_to_string(document).map_err(|why| why.to_string())?;
    let declarations = std::fs::read_to_string(declarations).map_err(|why| why.to_string())?;
    let modules = match modules {
        Some(path) => std::fs::read_to_string(path).map_err(|why| why.to_string())?,
        None => String::new(),
    };
    let checked = verify_with(&document, &declarations, &modules)?;
    if checked == 0 {
        return Err("no path-qualified invariant proofs were checked".to_owned());
    }
    Ok(checked)
}

fn main() -> ExitCode {
    match run() {
        Ok(checked) => {
            println!(
                "{checked} path-qualified test references checked against their exact tracked files"
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("invariant path reference refused:\n{why}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECLARATIONS: &str = "crates/api/src/example.rs\tfirst\ncrates/api/src/example.rs\tsecond\ncrates/api/src/example.rs\tthird\ncrates/cli/tests/example.rs\tother\n";

    #[test]
    fn comma_semicolon_and_multiple_file_references_are_all_checked() {
        let document = "| P-01 | property | `first`, `second`; `third` in `crates/api/src/example.rs`; `other` in `crates/cli/tests/example.rs` |";
        assert_eq!(verify(document, DECLARATIONS), Ok(4));
    }

    #[test]
    fn every_named_list_member_must_exist() {
        for proof in [
            "`missing`, `first`",
            "`first` and `missing`",
            "`first`; `missing`; `second`",
        ] {
            let document = format!("| invariant | {proof} in `crates/api/src/example.rs` |");
            assert!(verify(&document, DECLARATIONS).is_err());
        }
    }

    #[test]
    fn a_same_named_function_in_another_file_does_not_satisfy_the_proof() {
        let document = "| invariant | `first` in `crates/cli/tests/example.rs` |";
        let refusal = verify(document, DECLARATIONS).unwrap_err();
        assert!(refusal.contains("first"));
        assert!(refusal.contains("crates/cli/tests/example.rs"));
    }

    #[test]
    fn unknown_files_and_noncanonical_paths_refuse() {
        for path in ["crates/api/src/absent.rs", "crates/api/../secret.rs"] {
            let document = format!("| invariant | `first` in `{path}` |");
            assert!(verify(&document, DECLARATIONS).is_err());
        }
    }

    #[test]
    fn deep_module_names_parentheses_and_digit_suffixes_are_supported() {
        let declarations = format!("{DECLARATIONS}crates/api/src/example.rs\tversion_3\n");
        let document = "| invariant | `api::server::tests::first()`, `version_3` in `crates/api/src/example.rs` |";
        assert_eq!(verify(document, &declarations), Ok(2));
    }

    #[test]
    fn prose_and_unrelated_code_spans_do_not_become_test_names() {
        let document = "`not_a_test` in `crates/api/src/example.rs`\n| The `constant` is fixed | `first` in `crates/api/src/example.rs` |\n| Earlier `pull.member` tests in `crates/api/src/example.rs` were insufficient | tracked elsewhere |\n| Invariant | Executable proof in `crates/api/src/example.rs` |\n| Source is in `crates/api` | pending |";
        assert_eq!(verify(document, DECLARATIONS), Ok(1));
    }

    #[test]
    fn separate_qualified_proofs_do_not_borrow_a_later_source_path() {
        let document = "| invariant | `runner::bootstrap::tests::foreign`; `runner::other::tests::also_foreign`; `first`, `second` in `crates/api/src/example.rs` |";
        // `runner` is a crate here, so its tokens are gate 10's table's to check.
        let declarations = format!("{DECLARATIONS}crates/runner/src/lib.rs\tunrelated\n");
        assert_eq!(verify(document, &declarations), Ok(2));
    }

    #[test]
    fn a_module_first_token_must_name_a_function_its_module_declares() {
        let declarations = format!(
            "{DECLARATIONS}crates/api/src/server.rs\tbinds\ncrates/cli/src/index_stop/source_context.rs\tsnapshot\n"
        );
        let found = "| invariant | `server::tests::binds`; `source_context::tests::snapshot` |";
        assert_eq!(verify(found, &declarations), Ok(2));
        let deep = "| invariant | `index_stop::source_context::tests::snapshot` |";
        assert_eq!(verify(deep, &declarations), Ok(1));
        for absent in [
            "| invariant | `server::tests::renamed` |",
            "| invariant | `example::tests::binds` |",
            "| invariant | `nowhere::tests::first` |",
        ] {
            let refusal = verify(absent, &declarations).expect_err(absent);
            assert!(refusal.contains("names no crate"), "{refusal}");
        }
        // Prose and a non-row line are not tokens.
        assert_eq!(
            verify("`server::tests::renamed` in prose", &declarations),
            Ok(0)
        );
    }

    #[test]
    fn a_module_mounted_by_path_resolves_through_the_module_table() {
        let declarations = format!(
            "{DECLARATIONS}crates/api/src/store_wire.rs\tshares\ncrates/api/src/server.rs\tbinds\n"
        );
        let modules = "crates/api/src/store_wire.rs\tserver::store_wire\n";
        let token = "| invariant | `server::store_wire::tests::shares` |";
        assert!(
            verify(token, &declarations).is_err(),
            "the path alone cannot see the mount"
        );
        assert_eq!(verify_with(token, &declarations, modules), Ok(1));
        let tail = "| invariant | `store_wire::tests::shares` |";
        assert_eq!(verify_with(tail, &declarations, modules), Ok(1));
        let renamed = "| invariant | `server::store_wire::tests::renamed` |";
        assert!(verify_with(renamed, &declarations, modules).is_err());
        let elsewhere = "| invariant | `server::other::tests::shares` |";
        assert!(
            verify_with(elsewhere, &declarations, modules).is_err(),
            "a function declared under one mount does not satisfy a sibling module"
        );
    }

    #[test]
    fn a_ci_tool_path_is_a_source_path() {
        let declarations = ".github/source_scan.rs\tonly_a_declared_fn_is_a_function\n";
        let document =
            "| CIG | x | `only_a_declared_fn_is_a_function` in `.github/source_scan.rs` |";
        assert_eq!(verify(document, declarations), Ok(1));
        let missing = "| CIG | x | `gone` in `.github/source_scan.rs` |";
        assert!(verify(missing, declarations).is_err());
        let noncanonical = "| CIG | x | `gone` in `.github/../secret.rs` |";
        assert!(verify(noncanonical, declarations).is_err());
    }

    #[test]
    fn malformed_names_and_unclosed_code_spans_refuse() {
        for document in [
            "| invariant | `first second` in `crates/api/src/example.rs` |",
            "| invariant | `first` in `crates/api/src/example.rs |",
        ] {
            assert!(verify(document, DECLARATIONS).is_err());
        }
    }

    #[test]
    fn a_valid_reference_cannot_hide_a_later_missing_reference() {
        let document = "| invariant | `first` in `crates/api/src/example.rs` |\n| invariant | `absent` in `crates/api/src/example.rs` |";
        let refusal = verify(document, DECLARATIONS).unwrap_err();
        assert!(refusal.contains("line 2"));
        assert!(refusal.contains("absent"));
    }
}
