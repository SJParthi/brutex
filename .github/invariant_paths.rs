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

fn references(line: &str) -> Result<Vec<(String, String)>, String> {
    if !line.starts_with('|') || !line.contains(" in `crates/") {
        return Ok(Vec::new());
    }
    let spans = code_spans(line)?;
    let mut references = Vec::new();
    for (index, path) in spans.iter().enumerate() {
        if !path.value.starts_with("crates/") || !path.value.ends_with(".rs") || index == 0 {
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

fn verify(document: &str, declarations: &str) -> Result<usize, String> {
    let declarations: HashSet<&str> = declarations
        .lines()
        .filter(|line| !line.is_empty())
        .collect();
    let mut checked = 0;
    let mut refusals = Vec::new();
    for (index, line) in document.lines().enumerate() {
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
    let [document, declarations] = arguments.as_slice() else {
        return Err("usage: invariant-paths DOCUMENT PATH_DECLARATIONS".to_owned());
    };
    let document = std::fs::read_to_string(document).map_err(|why| why.to_string())?;
    let declarations = std::fs::read_to_string(declarations).map_err(|why| why.to_string())?;
    let checked = verify(&document, &declarations)?;
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
        assert_eq!(verify(document, DECLARATIONS), Ok(2));
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
