//! Tests of the rehearse-then-run gate (AUT-05). Every list is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use super::*;

/// The operator's recorded yes for the fixture step's one operator item.
const YES: &str = "operator_yes = [\"a yes | operator, 17 Oct 10:30 IST: 'yes, run it'\"]";

/// A list with one package and one kind step, `fields` spliced into the step.
fn list(merged: bool, fields: &str) -> String {
    format!(
        "format = 1\nleftovers = [\n  \"XX-01\", # a comment\n]\n\n[[package]]\nname = \"R9-01\"\nmerged = {merged}\ntitle = \"a = b # not a comment\"\n\n[[step]]\nid = \"S-90\"\nkind = \"tick-day-import\"\nprerequisites = [\"R9-01\", \"operator: a yes\"]\n{YES}\n{fields}\n\n[[step]]\nid = \"S-91\"\ntitle = \"a hand step, no kind\"\n"
    )
}

/// Every part of a rehearsal, present and current.
const WHOLE: &str = "state = \"pending\"\ncheck = [\n  \"GET /imports.json: done (0 today)\",\n]\nrehearsed_ms = 1200\nrehearsed_bytes = 4096\nrehearsed_commit = \"abc1234 200\"\nwriter_changed = \"def5678 150\"\nsnapshot = \"brutex-before-ticks 300\"";

const QUIET: Facts = Facts {
    last_bulk_write: LastBulkWrite::At(250),
    now: 1_000,
    attested: Vec::new(),
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
    // An undeclared package is unmerged too. (An operator item is judged
    // after the packages: see an_operator_prerequisite_is_never_met_silently.)
    let undeclared = list(true, WHOLE).replace(
        "[\"R9-01\", \"operator: a yes\"]",
        "[\"R9-77\", \"operator: a yes\"]",
    );
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
                ..QUIET
            },
        );
        assert_eq!(answer.is_ok(), admitted, "{facts:?}: {answer:?}");
    }
    let unknown = g
        .admit(
            "tick-day-import",
            &Facts {
                last_bulk_write: LastBulkWrite::Unknown("torn".to_owned()),
                ..QUIET
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
                ..QUIET
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
            ..QUIET
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

/// `list(true, WHOLE)` with the step's prerequisites and yes replaced.
fn with_operator(prerequisites: &str, yes: &str) -> Gate {
    Gate::from_list(
        &list(true, WHOLE)
            .replace("[\"R9-01\", \"operator: a yes\"]", prerequisites)
            .replace(YES, yes),
    )
    .unwrap()
}

/// **AN OPERATOR PREREQUISITE IS NEVER MET SILENTLY** (the second agent's
/// blocking 1, `verify-R4-06-r1.json`). Before this change every
/// `operator: <what>` item was skipped, so each case below that is refused
/// was admitted.
#[test]
fn an_operator_prerequisite_is_never_met_silently() {
    // THE SECOND AGENT'S OWN CASE: an approval the list does not record.
    let window = "approval, and a docs/05-decisions.md entry moving the window";
    let g = with_operator(&format!("[\"R9-01\", \"operator: {window}\"]"), "");
    let refused = g.admit("tick-day-import", &QUIET).unwrap_err();
    assert_eq!(
        refused,
        Refusal::OperatorPending {
            kind: "tick-day-import".to_owned(),
            step: "S-90".to_owned(),
            items: vec![window.to_owned()],
        }
    );
    assert_eq!(
        (refused.code(), refused.part()),
        ("operator_pending", "operator")
    );
    assert!(refused.to_string().contains(window), "{refused}");
    // An operator item alone, with every package merged, is still refused.
    let drill = with_operator("[\"operator: the restore drill (no agent runs it)\"]", "");
    assert_eq!(
        drill.admit("tick-day-import", &QUIET).unwrap_err().code(),
        "operator_pending"
    );
    // A RECORDED YES meets its item, and is handed back to the starter.
    let admitted = gate(true, WHOLE).admit("tick-day-import", &QUIET).unwrap();
    assert_eq!(
        admitted.operator,
        [OperatorMet {
            item: "a yes".to_owned(),
            by: MetBy::Recorded("operator, 17 Oct 10:30 IST: 'yes, run it'".to_owned()),
        }]
    );
    // AN ATTESTED CONDITION meets its item, by its exact words only.
    let token = "a Kite access token valid on the day";
    let g = with_operator(&format!("[\"R9-01\", \"operator: {token}\"]"), "");
    let attest = |words: &str| Facts {
        attested: vec![words.to_owned()],
        ..QUIET
    };
    let admitted = g.admit("tick-day-import", &attest(token)).unwrap();
    assert_eq!(
        admitted.operator,
        [OperatorMet {
            item: token.to_owned(),
            by: MetBy::Attested,
        }]
    );
    for near in [
        format!("operator: {token}"),
        token.to_uppercase(),
        format!("{token} "),
        String::new(),
    ] {
        assert_eq!(
            g.admit("tick-day-import", &attest(&near))
                .unwrap_err()
                .code(),
            "operator_pending",
            "{near:?}"
        );
    }
    // TWO ITEMS: each must be met; the refusal names only the unmet one, and
    // an admission hands both back in list order.
    let two = with_operator(
        &format!("[\"R9-01\", \"operator: a yes\", \"operator: {token}\"]"),
        YES,
    );
    let refused = two.admit("tick-day-import", &QUIET).unwrap_err();
    assert_eq!(
        refused,
        Refusal::OperatorPending {
            kind: "tick-day-import".to_owned(),
            step: "S-90".to_owned(),
            items: vec![token.to_owned()],
        }
    );
    let both = two.admit("tick-day-import", &attest(token)).unwrap();
    assert_eq!(
        both.operator
            .iter()
            .map(|m| m.item.as_str())
            .collect::<Vec<_>>(),
        ["a yes", token]
    );
    assert_eq!(both.operator[1].by, MetBy::Attested);
    // A YES THAT NAMES NO ITEM, or says no one's yes, does not read: the
    // whole list is refused (every kind list_unreadable), never half-read.
    for (yes, says) in [
        (
            "operator_yes = [\"a no | operator, 17 Oct\"]",
            "\"a no | operator, 17 Oct\"",
        ),
        ("operator_yes = [\"a yes | \"]", "\"a yes | \""),
        ("operator_yes = [\"a yes\"]", "\"a yes\""),
        (
            "operator_yes = [\"a yesterday | operator\"]",
            "\"a yesterday | operator\"",
        ),
    ] {
        let why = Gate::from_list(&list(true, WHOLE).replace(YES, yes)).unwrap_err();
        assert!(
            why.contains("S-90: operator_yes") && why.contains(says),
            "{yes}: {why}"
        );
    }
}

#[test]
fn a_done_or_withdrawn_step_never_starts_by_itself() {
    for (state, says) in [
        ("state = \"done\"", "is \"done\""),
        ("state = \"withdrawn\"", "is \"withdrawn\""),
        ("state = \"Pending\"", "is \"Pending\""),
        ("", "is (no state)"),
    ] {
        let fields = WHOLE.replace("state = \"pending\"", state);
        let refused = gate(true, &fields)
            .admit("tick-day-import", &QUIET)
            .unwrap_err();
        assert_eq!(
            (refused.code(), refused.part()),
            ("step_closed", "state"),
            "{state}"
        );
        assert!(refused.to_string().contains(says), "{state}: {refused}");
    }
}

/// The second agent's garbage figures and stamps, each admitted before.
#[test]
fn garbage_figures_and_impossible_stamps_are_refused() {
    for (from, to, part, says) in [
        (
            "rehearsed_ms = 1200",
            "rehearsed_ms = -5",
            "dry_run",
            "above 0",
        ),
        (
            "rehearsed_ms = 1200",
            "rehearsed_ms = 0",
            "dry_run",
            "above 0",
        ),
        (
            "rehearsed_bytes = 4096",
            "rehearsed_bytes = 0",
            "dry_run",
            "above 0",
        ),
        (
            "abc1234 200",
            "abc1234 -9223372036854775808",
            "rehearsed_commit",
            "between 1970 and now",
        ),
        (
            "def5678 150",
            "def5678 0",
            "writer_changed",
            "between 1970 and now",
        ),
        (
            "brutex-before-ticks 300",
            "brutex-before-ticks 99999999999",
            "snapshot",
            "between 1970 and now (1000)",
        ),
        (
            "abc1234 200",
            "not-a-commit 200",
            "rehearsed_commit",
            "not a commit hash",
        ),
        (
            "def5678 150",
            "DEF5678 150",
            "writer_changed",
            "not a commit hash",
        ),
        (
            "abc1234 200",
            "abc12 200",
            "rehearsed_commit",
            "not a commit hash",
        ),
    ] {
        let fields = WHOLE.replace(from, to);
        let refused = gate(true, &fields)
            .admit("tick-day-import", &QUIET)
            .unwrap_err();
        assert_eq!(
            (refused.code(), refused.part()),
            ("not_rehearsed", part),
            "{to}"
        );
        assert!(refused.to_string().contains(says), "{to}: {refused}");
    }
    // A stamp AT the caller's clock is not after it; one past it is.
    let at_now = WHOLE.replace("brutex-before-ticks 300", "brutex-before-ticks 1000");
    assert!(gate(true, &at_now).admit("tick-day-import", &QUIET).is_ok());
    let a_40_hex = WHOLE.replace("abc1234", &"a".repeat(40));
    assert!(
        gate(true, &a_40_hex)
            .admit("tick-day-import", &QUIET)
            .is_ok()
    );
    let a_41_hex = WHOLE.replace("abc1234", &"a".repeat(41));
    assert!(
        gate(true, &a_41_hex)
            .admit("tick-day-import", &QUIET)
            .is_err()
    );
}

#[test]
fn a_key_twice_or_a_package_twice_does_not_read() {
    let twice = list(true, WHOLE).replace(
        "rehearsed_ms = 1200",
        "rehearsed_ms = 1200\nrehearsed_ms = 7",
    );
    let why = Gate::from_list(&twice).unwrap_err();
    assert!(
        why.contains("key \"rehearsed_ms\" is written twice"),
        "{why}"
    );
    let package = list(true, WHOLE).replace(
        "[[step]]\nid = \"S-90\"",
        "[[package]]\nname = \"R9-01\"\nmerged = false\n\n[[step]]\nid = \"S-90\"",
    );
    let why = Gate::from_list(&package).unwrap_err();
    assert!(why.contains("package \"R9-01\" is declared twice"), "{why}");
    // The same key in two tables is two keys.
    assert!(Gate::from_list(&list(true, WHOLE)).is_ok());
}

#[test]
fn a_kind_longer_than_the_bound_is_refused_unread() {
    let long = "k".repeat(1 << 20);
    let g = gate(true, WHOLE);
    let started = std::time::Instant::now();
    let refused = g.admit(&long, &QUIET).unwrap_err();
    let took = started.elapsed();
    assert_eq!((refused.code(), refused.part()), ("not_rehearsed", "entry"));
    assert_eq!(refused.kind().len(), MAX_KIND_BYTES);
    assert!(refused.to_string().contains("1048576 bytes"), "{refused}");
    println!("rehearsal admit, a 1 MiB kind: refused in {took:?}");
    // At the bound it is read (and is no kind of this list).
    let at = "k".repeat(MAX_KIND_BYTES);
    let refused = gate(true, WHOLE).admit(&at, &QUIET).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("no step of config/store-steps.toml"),
        "{refused}"
    );
}

#[test]
fn a_check_line_field_cannot_forge_another() {
    let forged = check_line(&CheckRecord {
        run: "r1 kind=forged step=S-00 outcome=passed",
        at: 7,
        kind: "k%",
        step: "S-90\tx",
        outcome: &CheckOutcome::Failed("held 3 = 4? no".to_owned()),
    });
    assert_eq!(
        forged,
        "v=1 at=7 run=r1%20kind%3Dforged%20step%3DS-00%20outcome%3Dpassed kind=k%25 step=S-90%09x outcome=failed why=held 3 = 4? no\n"
    );
    // Every field but the last is one space-free word: a first-match reader
    // reads the true outcome.
    let words: Vec<&str> = forged.split(' ').collect();
    assert_eq!(
        words.iter().filter(|w| w.starts_with("outcome=")).count(),
        1
    );
    assert_eq!(words[5], "outcome=failed");
}
