//! Plans bounded mutation jobs and reconciles their exact assigned case sets.
//! This CI-only executable has no dependencies and is built directly by rustc.
//! It never excludes a mutation or credits a timeout as a caught mutation.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

const JOB_MINUTES: usize = 240;
// Calibrated from PR #19's run 36318672418 (D-0697). Setup and the required
// clean baseline took about 80 to 91 minutes of each 240-minute job before a
// mutant ran, and the shards then spent about 202 to 259 worker-seconds per
// case. The old 40 minutes and 120 seconds packed 165 cases into each of three
// jobs, and all three hit the 240-minute limit with 256 of 495 cases untested.
// These are still estimates: the worst measured case cost plus a margin.
//
// Recalibrated from PR #74's run 37092404876 (D-1453). At 270s, 62 cases a
// job: of the first 12 shards to stop, the 8 that finished spent about 205
// to 290 worker-seconds a case after a 27 to 40 minute baseline, and 4 were
// cancelled at the 240-minute limit with no outcome recorded, so their
// survivors and timeouts went unnamed. 540s is twice the worst finished
// cost, and leaves room in each job for one mutant that runs to cargo-mutants'
// own test timeout, so that case is named as timed out instead of being lost.
const RESERVE_MINUTES: usize = 100;
const ESTIMATED_CASE_SECONDS: usize = 540;
const WORKERS: usize = 2;
const MAX_JOBS: usize = 256;
const CASES_PER_JOB: usize =
    (JOB_MINUTES - RESERVE_MINUTES) * 60 * WORKERS / ESTIMATED_CASE_SECONDS;
const MAX_LIST_BYTES: u64 = 64 * 1024 * 1024;

fn cases(text: &str) -> Result<Vec<String>, String> {
    let mut unique = BTreeSet::new();
    let mut values = Vec::new();
    for line in text.lines() {
        if line.is_empty()
            || !line.starts_with("crates/")
            || !line.contains(".rs:")
            || line.chars().any(char::is_control)
        {
            return Err(format!("noncanonical mutation-list line: {line:?}"));
        }
        if !unique.insert(line.to_owned()) {
            return Err(format!("duplicate mutation case: {line}"));
        }
        values.push(line.to_owned());
    }
    Ok(values)
}

fn read_cases(path: &Path) -> Result<Vec<String>, String> {
    let metadata = std::fs::metadata(path).map_err(|why| format!("{}: {why}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_LIST_BYTES {
        return Err(format!("{} is not a bounded regular list", path.display()));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|why| format!("cannot read {}: {why}", path.display()))?;
    cases(&text)
}

fn job_count(count: usize) -> Result<usize, String> {
    let jobs = count.div_ceil(CASES_PER_JOB).max(1);
    if jobs > MAX_JOBS {
        return Err(format!(
            "{count} mutation cases need {jobs} jobs at {CASES_PER_JOB} cases/job; the matrix limit is {MAX_JOBS}. No case was excluded."
        ));
    }
    Ok(jobs)
}

fn matrix(count: usize) -> Result<String, String> {
    let jobs = job_count(count)?;
    let mut text = String::from("{\"include\":[");
    for index in 0..jobs {
        if index > 0 {
            text.push(',');
        }
        let expected = count / jobs + usize::from(index < count % jobs);
        text.push_str(&format!(
            "{{\"index\":{index},\"shard\":\"{index}/{jobs}\",\"expected\":{expected}}}"
        ));
    }
    text.push_str("]}");
    Ok(text)
}

fn shard_parts(shard: &str) -> Result<(usize, usize), String> {
    let (index, total) = shard
        .split_once('/')
        .ok_or_else(|| "shard must be zero-based index/total".to_owned())?;
    let index = index.parse::<usize>().map_err(|why| why.to_string())?;
    let total = total.parse::<usize>().map_err(|why| why.to_string())?;
    if total == 0 || total > MAX_JOBS || index >= total {
        return Err("shard is outside its bounded matrix".to_owned());
    }
    Ok((index, total))
}

fn require_partition(all: &[String], assigned: &[String], shard: &str) -> Result<(), String> {
    let (index, total) = shard_parts(shard)?;
    if total != job_count(all.len())? {
        return Err("shard denominator differs from the complete plan".to_owned());
    }
    let expected: Vec<_> = all.iter().skip(index).step_by(total).collect();
    if assigned.len() > CASES_PER_JOB || expected != assigned.iter().collect::<Vec<_>>() {
        return Err(format!(
            "shard {shard} does not exactly reproduce its round-robin assignment"
        ));
    }
    Ok(())
}

fn require_results(
    assigned: &[String],
    caught: &[String],
    unviable: &[String],
    missed: &[String],
    timeout: &[String],
    status: &str,
) -> Result<(), String> {
    if status != "0" || !missed.is_empty() || !timeout.is_empty() {
        return Err(format!(
            "mutation execution failed: exit={status}, surviving={}, timed-out={}",
            missed.len(),
            timeout.len()
        ));
    }
    let completed: BTreeSet<_> = caught.iter().chain(unviable).collect();
    if completed.len() != caught.len() + unviable.len() {
        return Err("a mutation has more than one terminal outcome".to_owned());
    }
    let expected: BTreeSet<_> = assigned.iter().collect();
    if expected.len() != assigned.len() || completed != expected {
        let absent = expected.difference(&completed).count();
        let foreign = completed.difference(&expected).count();
        return Err(format!(
            "mutation outcomes do not reconcile: missing={absent}, foreign={foreign}"
        ));
    }
    Ok(())
}

/// `a/b/../c` is `a/c`; `.` segments vanish. A path that climbs above its
/// root keeps the `..` it cannot resolve, so it never equals a tracked path.
fn normalized(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." if out.last().is_some_and(|last| *last != "..") => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.join("/")
}

/// The path a unified-diff header line names: `--- a/X`, `+++ b/X` or
/// `diff --git a/X b/Y` (whose `b/` side is taken, as git writes it).
fn header_path(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("diff --git a/") {
        return rest.split_once(" b/").map(|(_, b)| b);
    }
    line.strip_prefix("--- a/")
        .or_else(|| line.strip_prefix("+++ b/"))
}

/// cargo-mutants names a file by the path its `#[path]` mount spells, so
/// `crates/cli/src/../build_provenance.rs` is never the
/// `crates/cli/build_provenance.rs` git's diff names, and `--in-diff` matched
/// nothing in such a file (GAP14-56, D-1119). Every diff section whose
/// normalized path cargo-mutants walks under another spelling is emitted once
/// per spelling with its three header paths replaced; every other section is
/// emitted unchanged.
fn respell(diff: &str, walked: &str) -> String {
    let mut spellings: std::collections::BTreeMap<String, Vec<&str>> =
        std::collections::BTreeMap::new();
    for spelled in walked.lines().filter(|line| !line.is_empty()) {
        let plain = normalized(spelled);
        if plain != spelled {
            spellings.entry(plain).or_default().push(spelled);
        }
    }
    let mut sections: Vec<Vec<&str>> = Vec::new();
    for line in diff.split_inclusive('\n') {
        if line.starts_with("diff --git ") || sections.is_empty() {
            sections.push(Vec::new());
        }
        if let Some(section) = sections.last_mut() {
            section.push(line);
        }
    }
    let mut out = String::new();
    for section in sections {
        let path = section
            .first()
            .and_then(|line| header_path(line.trim_end_matches('\n')))
            .map(normalized);
        let Some(names) = path.as_ref().and_then(|p| spellings.get(p)) else {
            out.extend(section);
            continue;
        };
        for name in names {
            for line in &section {
                let bare = line.trim_end_matches('\n');
                let ending = &line[bare.len()..];
                if bare.starts_with("diff --git ") {
                    out.push_str(&format!("diff --git a/{name} b/{name}{ending}"));
                } else if bare.starts_with("--- a/") {
                    out.push_str(&format!("--- a/{name}{ending}"));
                } else if bare.starts_with("+++ b/") {
                    out.push_str(&format!("+++ b/{name}{ending}"));
                } else {
                    out.push_str(line);
                }
            }
        }
    }
    out
}

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [mode, diff, walked] if mode == "respell" => {
            let read = |p: &str| {
                std::fs::read_to_string(p).map_err(|why| format!("cannot read {p}: {why}"))
            };
            print!("{}", respell(&read(diff)?, &read(walked)?));
        }
        [mode, list] if mode == "plan" => {
            let all = read_cases(Path::new(list))?;
            eprintln!(
                "{} cases; {} jobs; at most {CASES_PER_JOB} cases/job; {WORKERS} workers/job; {JOB_MINUTES} minutes/job with {RESERVE_MINUTES} reserved. The {ESTIMATED_CASE_SECONDS}s/case figure is an estimate, not a measured runtime.",
                all.len(),
                job_count(all.len())?
            );
            println!("{}", matrix(all.len())?);
        }
        [mode, all, assigned, shard] if mode == "partition" => {
            require_partition(
                &read_cases(Path::new(all))?,
                &read_cases(Path::new(assigned))?,
                shard,
            )?;
            println!("shard {shard}: exact assignment verified");
        }
        [mode, assigned, directory, status] if mode == "verify" => {
            let assigned = read_cases(Path::new(assigned))?;
            let directory = Path::new(directory);
            let caught = read_cases(&directory.join("caught.txt"))?;
            let unviable = read_cases(&directory.join("unviable.txt"))?;
            require_results(
                &assigned,
                &caught,
                &unviable,
                &read_cases(&directory.join("missed.txt"))?,
                &read_cases(&directory.join("timeout.txt"))?,
                status,
            )?;
            println!(
                "{} assigned cases reconciled: {} caught, {} unviable; no survivors, timeouts or missing outcomes",
                assigned.len(), caught.len(), unviable.len()
            );
        }
        _ => return Err("use respell DIFF WALKED_FILES, plan LIST, partition ALL ASSIGNED INDEX/TOTAL, or verify ASSIGNED RESULTS EXIT_STATUS".to_owned()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("GATE 18 REFUSED: {why}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generated(count: usize) -> Vec<String> {
        (0..count)
            .map(|index| format!("crates/example/src/lib.rs:{index}:1: replace value with false"))
            .collect()
    }

    #[test]
    fn capacity_and_empty_matrix_are_explicit() {
        assert_eq!(CASES_PER_JOB, 31);
        assert_eq!(job_count(0), Ok(1));
        assert_eq!(job_count(31), Ok(1));
        assert_eq!(job_count(32), Ok(2));
        assert_eq!(job_count(495), Ok(16));
        assert_eq!(job_count(2_405), Ok(78));
        assert_eq!(job_count(7_905), Ok(255));
        assert_eq!(job_count(7_936), Ok(256));
        assert!(job_count(7_937).is_err());
        assert_eq!(
            matrix(0).unwrap(),
            "{\"include\":[{\"index\":0,\"shard\":\"0/1\",\"expected\":0}]}"
        );
    }

    #[test]
    fn large_plan_partitions_every_case_once_without_sampling() {
        let all = generated(7_905);
        let total = job_count(all.len()).unwrap();
        let mut seen = BTreeSet::new();
        for index in 0..total {
            let assigned: Vec<_> = all.iter().skip(index).step_by(total).cloned().collect();
            require_partition(&all, &assigned, &format!("{index}/{total}")).unwrap();
            assert!(assigned.len() <= CASES_PER_JOB);
            for name in assigned {
                assert!(seen.insert(name));
            }
        }
        assert_eq!(seen, all.into_iter().collect());
    }

    #[test]
    fn wrong_missing_reordered_and_foreign_assignments_refuse() {
        let all = generated(CASES_PER_JOB + 1);
        let assigned: Vec<_> = all.iter().step_by(2).cloned().collect();
        assert!(require_partition(&all, &assigned, "0/2").is_ok());
        for bad in ["0/0", "2/2", "0/257", "one/two", "0/1"] {
            assert!(require_partition(&all, &assigned, bad).is_err());
        }
        assert!(require_partition(&all, &assigned, "1/2").is_err());
        assert!(require_partition(&all, &assigned[1..], "0/2").is_err());
        let mut reordered = assigned.clone();
        reordered.swap(0, 1);
        assert!(require_partition(&all, &reordered, "0/2").is_err());
        let mut foreign = assigned;
        foreign[0] = "crates/foreign/src/lib.rs:1:1: replace true with false".to_owned();
        assert!(require_partition(&all, &foreign, "0/2").is_err());
    }

    #[test]
    fn only_complete_disjoint_caught_and_unviable_results_pass() {
        let assigned = generated(3);
        assert!(require_results(&assigned, &assigned[..2], &assigned[2..], &[], &[], "0").is_ok());
        assert!(require_results(&assigned, &assigned[..2], &[], &[], &[], "0").is_err());
        assert!(require_results(&assigned, &assigned, &assigned[..1], &[], &[], "0").is_err());
        let foreign = generated(4);
        assert!(require_results(&assigned, &foreign, &[], &[], &[], "0").is_err());
        assert!(require_results(&assigned, &assigned, &[], &assigned[..1], &[], "0").is_err());
        assert!(require_results(&assigned, &assigned, &[], &[], &assigned[..1], "0").is_err());
        assert!(require_results(&assigned, &assigned, &[], &[], &[], "4").is_err());
    }

    #[test]
    fn a_path_mounted_file_is_named_as_cargo_mutants_walks_it() {
        let diff = "diff --git a/crates/cli/build_provenance.rs b/crates/cli/build_provenance.rs\n\
                    index 1..2 100644\n\
                    --- a/crates/cli/build_provenance.rs\n\
                    +++ b/crates/cli/build_provenance.rs\n\
                    @@ -1 +1 @@\n\
                    -a\n\
                    +b\n\
                    diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs\n\
                    --- a/crates/cli/src/lib.rs\n\
                    +++ b/crates/cli/src/lib.rs\n\
                    @@ -1 +1 @@\n\
                    -c\n\
                    +d\n";
        let walked = "crates/cli/src/lib.rs\ncrates/cli/src/../build_provenance.rs\n\
                      crates/cli/src/./sub/../../build_provenance.rs\n";
        let out = respell(diff, walked);
        // Both spellings get the section, with every header renamed.
        for name in [
            "crates/cli/src/../build_provenance.rs",
            "crates/cli/src/./sub/../../build_provenance.rs",
        ] {
            assert!(
                out.contains(&format!("diff --git a/{name} b/{name}\n")),
                "{out}"
            );
            assert!(out.contains(&format!("--- a/{name}\n")), "{out}");
            assert!(out.contains(&format!("+++ b/{name}\n")), "{out}");
        }
        assert!(!out.contains("a/crates/cli/build_provenance.rs"), "{out}");
        assert_eq!(out.matches("+b\n").count(), 2);
        // A file walked under its own name passes through untouched.
        assert!(out.ends_with(
            "diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs\n\
             --- a/crates/cli/src/lib.rs\n+++ b/crates/cli/src/lib.rs\n@@ -1 +1 @@\n-c\n+d\n"
        ));
        // Nothing walked under another name: the diff is returned byte for byte.
        assert_eq!(respell(diff, "crates/cli/src/lib.rs\n"), diff);
        assert_eq!(normalized("crates/cli/src/../x.rs"), "crates/cli/x.rs");
        assert_eq!(normalized("../x.rs"), "../x.rs");
    }

    #[test]
    fn malformed_or_duplicate_lists_are_not_silently_counted() {
        assert_eq!(cases(""), Ok(Vec::new()));
        let one = generated(1).join("\n");
        assert_eq!(cases(&format!("{one}\n")).unwrap().len(), 1);
        for bad in [
            format!("{one}\n{one}"),
            format!("{one}\n\n"),
            "warning".to_owned(),
            format!("{one}\u{1b}"),
        ] {
            assert!(cases(&bad).is_err());
        }
    }
}
