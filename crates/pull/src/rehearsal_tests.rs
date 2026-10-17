//! Tests of the rehearse-then-run gate (AUT-05). Every list is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use super::*;

/// A list with one package and one kind step, `fields` spliced into the step.
fn list(merged: bool, fields: &str) -> String {
    format!(
        "format = 1\nleftovers = [\n  \"XX-01\", # a comment\n]\n\n[[package]]\nname = \"R9-01\"\nmerged = {merged}\ntitle = \"a = b # not a comment\"\n\n[[step]]\nid = \"S-90\"\nkind = \"tick-day-import\"\nprerequisites = [\"R9-01\", \"operator: a yes\"]\n{fields}\n\n[[step]]\nid = \"S-91\"\ntitle = \"a hand step, no kind\"\n"
    )
}

/// Every part of a rehearsal, present and current.
const WHOLE: &str = "check = [\n  \"GET /imports.json: done (0 today)\",\n]\nrehearsed_ms = 1200\nrehearsed_bytes = 4096\nrehearsed_commit = \"abc1234 200\"\nwriter_changed = \"def5678 150\"\nsnapshot = \"brutex-before-ticks 300\"";

const QUIET: Facts = Facts {
    last_bulk_write: LastBulkWrite::At(250),
};

fn gate(merged: bool, fields: &str) -> Gate {
    Gate::from_list(&list(merged, fields)).unwrap()
}

/// `WHOLE` without the line whose key is `key`.
fn without(key: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in WHOLE.lines() {
        if line.starts_with(&format!("{key} =")) {
            skipping = line.trim_end().ends_with('[');
            continue;
        }
        if skipping {
            skipping = line.trim() != "]";
            continue;
        }
        out.push(line);
    }
    out.join("\n")
}

#[test]
fn an_unrehearsed_kind_refuses_to_start_by_itself() {
    // A WHOLE REHEARSAL IS ADMITTED, and carries what its run must record.
    let admitted = gate(true, WHOLE).admit("tick-day-import", &QUIET).unwrap();
    assert_eq!(admitted.step, "S-90");
    assert_eq!(admitted.check, ["GET /imports.json: done (0 today)"]);
    assert_eq!(
        (admitted.rehearsed_ms, admitted.rehearsed_bytes),
        (1200, 4096)
    );
    // A KIND NO STEP NAMES.
    let refused = gate(true, WHOLE).admit("gap-fill", &QUIET).unwrap_err();
    assert_eq!((refused.code(), refused.part()), ("not_rehearsed", "entry"));
    assert!(
        refused.to_string().contains("not rehearsed: gap-fill"),
        "{refused}"
    );
    // EACH MISSING PART, BY NAME.
    for (key, part) in [
        ("rehearsed_ms", "dry_run"),
        ("rehearsed_bytes", "dry_run"),
        ("rehearsed_commit", "rehearsed_commit"),
        ("writer_changed", "writer_changed"),
        ("check", "check"),
        ("snapshot", "snapshot"),
    ] {
        let refused = gate(true, &without(key))
            .admit("tick-day-import", &QUIET)
            .unwrap_err();
        assert_eq!(refused.code(), "not_rehearsed", "{key}: {refused}");
        assert_eq!(refused.part(), part, "{key}: {refused}");
        assert!(refused.to_string().contains("S-90"), "{key}: {refused}");
    }
    // A STAMP THAT DOES NOT READ is the same missing part, said.
    let garbled = WHOLE.replace("abc1234 200", "abc1234 yesterday");
    let refused = gate(true, &garbled)
        .admit("tick-day-import", &QUIET)
        .unwrap_err();
    assert_eq!(refused.part(), "rehearsed_commit");
    assert!(
        refused.to_string().contains("not unix seconds"),
        "{refused}"
    );
    // AN EMPTY CHECK is no check.
    let blank = WHOLE.replace("\"GET /imports.json: done (0 today)\"", "\" \"");
    assert_eq!(
        gate(true, &blank)
            .admit("tick-day-import", &QUIET)
            .unwrap_err()
            .part(),
        "check"
    );
}

#[test]
fn a_rehearsal_older_than_the_writer_change_is_stale() {
    let older = WHOLE.replace("abc1234 200", "abc1234 149");
    let refused = gate(true, &older)
        .admit("tick-day-import", &QUIET)
        .unwrap_err();
    assert_eq!(refused.code(), "stale");
    assert!(
        refused
            .to_string()
            .contains("rehearsed on abc1234 at 149, before its writer changed in def5678 at 150"),
        "{refused}"
    );
    // AT the writer's change is current: "at or after".
    let same = WHOLE.replace("abc1234 200", "def5678 150");
    assert!(gate(true, &same).admit("tick-day-import", &QUIET).is_ok());
}

#[test]
fn an_unmerged_prerequisite_is_not_ready() {
    let refused = gate(false, WHOLE)
        .admit("tick-day-import", &QUIET)
        .unwrap_err();
    assert_eq!(refused.code(), "not_ready");
    assert_eq!(
        refused,
        Refusal::NotReady {
            kind: "tick-day-import".to_owned(),
            waits_for: vec!["R9-01".to_owned()],
        }
    );
    // An undeclared package is unmerged too; an operator item is the
    // person's and does not hold an automatic start.
    let undeclared = list(true, WHOLE).replace("[\"R9-01\", \"operator: a yes\"]", "[\"R9-77\"]");
    let refused = Gate::from_list(&undeclared)
        .unwrap()
        .admit("tick-day-import", &QUIET)
        .unwrap_err();
    assert_eq!(refused.code(), "not_ready");
}

#[test]
fn a_deletion_or_a_credential_change_is_never_automatic() {
    for kind in [
        "prune-logs",
        "retire-raw-months",
        "DELETE-old",
        "token-refresh",
        "credential-rotate",
    ] {
        let named = list(true, WHOLE).replace("tick-day-import", kind);
        let refused = Gate::from_list(&named)
            .unwrap()
            .admit(kind, &QUIET)
            .unwrap_err();
        assert_eq!(refused.code(), "never_automatic", "{kind}");
        assert_eq!(refused.kind(), kind);
    }
}

#[test]
fn a_snapshot_must_be_newer_than_the_last_bulk_write() {
    let g = gate(true, WHOLE);
    for (facts, admitted) in [
        (LastBulkWrite::At(299), true),
        (LastBulkWrite::At(300), false),
        (LastBulkWrite::At(301), false),
        (LastBulkWrite::NoneRecorded, true),
        (
            LastBulkWrite::Unknown("the runs ledger is torn".to_owned()),
            false,
        ),
    ] {
        let answer = g.admit(
            "tick-day-import",
            &Facts {
                last_bulk_write: facts.clone(),
            },
        );
        assert_eq!(answer.is_ok(), admitted, "{facts:?}: {answer:?}");
    }
    let unknown = g
        .admit(
            "tick-day-import",
            &Facts {
                last_bulk_write: LastBulkWrite::Unknown("torn".to_owned()),
            },
        )
        .unwrap_err();
    assert_eq!(unknown.part(), "last_bulk_write_unknown");
    assert!(unknown.to_string().contains("torn"));
    let older = g
        .admit(
            "tick-day-import",
            &Facts {
                last_bulk_write: LastBulkWrite::At(300),
            },
        )
        .unwrap_err();
    assert_eq!(older.part(), "snapshot_older_than_last_bulk_write");
}

/// A new ledger path in the temporary folder.
fn ledger(name: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "brutex-rehearsal-{name}-{}-{}.log",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn the_check_result_is_stored_after_an_automatic_run() {
    let admitted = gate(true, WHOLE).admit("tick-day-import", &QUIET).unwrap();
    let path = ledger("checks");
    let mut asked = Vec::new();
    let outcome = after_run(
        &admitted,
        "run-17",
        1_800_000_000,
        |check| {
            asked = check.to_vec();
            CheckOutcome::Passed
        },
        &path,
    )
    .unwrap();
    assert_eq!(outcome, CheckOutcome::Passed);
    assert_eq!(asked, admitted.check, "the step's own check was read");
    let failed = after_run(
        &admitted,
        "run-18",
        1_800_000_060,
        |_| CheckOutcome::Failed("held 3 of 4 days\nday 4 missing".to_owned()),
        &path,
    )
    .unwrap();
    assert_eq!(failed.word(), "failed");
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        text,
        "v=1 at=1800000000 run=run-17 kind=tick-day-import step=S-90 outcome=passed why=\n\
         v=1 at=1800000060 run=run-18 kind=tick-day-import step=S-90 outcome=failed why=held 3 of 4 days day 4 missing\n"
    );
    // A RESULT THAT CANNOT BE STORED IS AN ERROR, never a quiet success.
    let nowhere = ledger("no-such-dir").join("checks.log");
    let stored = after_run(&admitted, "run-19", 1, |_| CheckOutcome::Passed, &nowhere);
    assert!(stored.is_err());
    // A LONG WHY IS BOUNDED on a character boundary.
    let long = CheckOutcome::NotEvaluated("é".repeat(MAX_WHY_BYTES));
    let line = check_line(&CheckRecord {
        run: "r",
        at: 0,
        kind: "k",
        step: "S-90",
        outcome: &long,
    });
    let why = line.trim_end().split_once("why=").unwrap().1;
    assert!(
        why.len() <= MAX_WHY_BYTES && why.len() >= MAX_WHY_BYTES - 1,
        "{}",
        why.len()
    );
    assert!(line.contains("outcome=not_evaluated"));
}

#[test]
fn the_list_reader_refuses_by_line_and_two_steps_cannot_share_a_kind() {
    let twice = format!(
        "{}\n[[step]]\nid = \"S-92\"\nkind = \"tick-day-import\"\n",
        list(true, WHOLE)
    );
    let why = Gate::from_list(&twice).unwrap_err();
    assert!(
        why.contains("\"tick-day-import\" is named by two steps"),
        "{why}"
    );
    for (text, says) in [
        (
            "[[step]]\nid = \"S-1\nkind = \"x\"\n",
            "line 2: a string is not closed",
        ),
        (
            "[[step]]\ncheck = [\n  \"a\",\n",
            "line 2: an array is not closed",
        ),
        ("[[step]]\nid = maybe\n", "line 2: \"maybe\" is not"),
        ("[[step]]\nid\n", "line 2: no `key = value`"),
        ("[[step]]\ncheck = [\"a\" \"b\"] x\n", "line 2: text after"),
        ("[[step]]\nid = \"a\" b\n", "line 2: text after a string"),
        ("[[step]]\nid = \"a\\q\"\n", "line 2: escape"),
        (
            "[[step]]\ncheck = [7]\n",
            "line 2: \"7]\" is not a string item",
        ),
    ] {
        let why = Gate::from_list(text).unwrap_err();
        assert!(why.contains(says), "{text:?}: {why}");
    }
    // Escapes and an integer, a flag and a comment are read.
    let fine = "a = 1 # n\nb = true\n[[step]]\nid = \"S-\\\"9\\\\\\n\\t\"\nkind = \"k\"\n";
    let g = Gate::from_list(fine).unwrap();
    assert_eq!(g.kinds(), 1);
    let refused = g.admit("k", &QUIET).unwrap_err();
    assert!(refused.to_string().contains("S-\"9\\\n\t"), "{refused}");
}

#[test]
fn the_tracked_list_reads_and_names_no_kind_yet() {
    let gate = Gate::embedded().expect("config/store-steps.toml reads");
    // At this merge no automatic starter exists, so no step names a kind:
    // every automatic start is refused as not rehearsed (entry), loudly.
    assert_eq!(gate.kinds(), 0);
    let refused = may_start_by_itself(
        "tick-day-import",
        &Facts {
            last_bulk_write: LastBulkWrite::NoneRecorded,
        },
    )
    .unwrap_err();
    assert_eq!((refused.code(), refused.part()), ("not_rehearsed", "entry"));
}

/// **THE GATE IS ONE LOOKUP PER START, MEASURED**: n = 1,000 admits over a
/// list of 1 kind and one of 10,000 kinds. Printed, the ratio of the p50s
/// bounded loosely (a hash probe does not grow with the map); the timing is a
/// measurement, not a claim about another machine.
#[test]
fn the_gate_is_one_lookup_per_start() {
    let mut big = list(true, WHOLE);
    for n in 0..10_000 {
        use std::fmt::Write as _;
        write!(
            big,
            "\n[[step]]\nid = \"S-{n:05}\"\nkind = \"kind-{n}\"\n{WHOLE}\n"
        )
        .unwrap();
    }
    let small = gate(true, WHOLE);
    let large = Gate::from_list(&big).unwrap();
    assert_eq!(large.kinds(), 10_001);
    let time = |g: &Gate, kind: &str| {
        let mut ns: Vec<u128> = (0..1_000)
            .map(|_| {
                let started = std::time::Instant::now();
                let answer = g.admit(kind, &QUIET);
                let took = started.elapsed().as_nanos();
                assert!(answer.is_ok());
                took
            })
            .collect();
        ns.sort_unstable();
        (ns[500], ns[990], ns[999])
    };
    let (s50, s99, smax) = time(&small, "tick-day-import");
    let (l50, l99, lmax) = time(&large, "kind-9999");
    println!("rehearsal admit 1 kind: n=1000 p50={s50}ns p99={s99}ns max={smax}ns");
    println!("rehearsal admit 10001 kinds: n=1000 p50={l50}ns p99={l99}ns max={lmax}ns");
    assert!(
        l50 < s50.max(1) * 20,
        "p50 {l50}ns at 10,001 kinds against {s50}ns at 1"
    );
}
