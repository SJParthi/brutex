//! BASE-COV-PULL-PS1 (D-7335): the rehearse-then-run gate's two sides no test
//! had reached (Route B, recipe v4): a `[[package]]` table that does not state
//! both its name and whether it is merged, and a check line's why cut inside
//! a character. Every list, kind, package, step and run is invented.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use super::*;

/// Every part of a rehearsal, present and current (the module's own `WHOLE`,
/// copied: it is private to `tests`).
const WHOLE: &str = "state = \"pending\"\ncheck = [\"GET /imports.json: done (0 today)\"]\nrehearsed_ms = 1200\nrehearsed_bytes = 4096\nrehearsed_commit = \"abc1234 200\"\nwriter_changed = \"def5678 150\"\nsnapshot = \"brutex-before-ticks 300\"";

/// The starter's facts: a bulk write before the snapshot, a clock after
/// every stamp, nothing attested.
fn quiet() -> Facts {
    Facts {
        last_bulk_write: LastBulkWrite::At(250),
        now: 1_000,
        attested: Vec::new(),
    }
}

/// `packages`, then one whole step whose kind waits for the package `ZQ-01`.
fn listing(packages: &str) -> String {
    format!(
        "{packages}\n[[step]]\nid = \"S-90\"\nkind = \"ZQ-IMPORT\"\nprerequisites = [\"ZQ-01\"]\n{WHOLE}\n"
    )
}

/// **A PACKAGE TABLE THAT DOES NOT STATE BOTH ITS NAME AND WHETHER IT IS
/// MERGED DECLARES NOTHING.** A `[[package]]` holding a name and no `merged`,
/// and one holding `merged = true` and no name, read without a refusal and
/// record no package: the kind that waits for the name is not ready, naming
/// it. A whole declaration of the name after them is its first, not a second
/// (no "declared twice"), and the kind it waited for is admitted.
#[test]
fn a_package_table_without_its_name_or_its_flag_declares_nothing() {
    let half = "[[package]]\nname = \"ZQ-01\"\n\n[[package]]\nmerged = true\n";
    let gate = Gate::from_list(&listing(half)).unwrap();
    assert_eq!(gate.kinds(), 1);
    assert_eq!(
        gate.admit("ZQ-IMPORT", &quiet()),
        Err(Refusal::NotReady {
            kind: "ZQ-IMPORT".to_owned(),
            waits_for: vec!["ZQ-01".to_owned()],
        })
    );
    let whole = format!("{half}\n[[package]]\nname = \"ZQ-01\"\nmerged = true\n");
    let admitted = Gate::from_list(&listing(&whole))
        .unwrap()
        .admit("ZQ-IMPORT", &quiet())
        .unwrap();
    assert_eq!(
        (
            admitted.step.as_str(),
            admitted.rehearsed_ms,
            admitted.rehearsed_bytes
        ),
        ("S-90", 1_200, 4_096)
    );
}

/// **A WHY CUT INSIDE A CHARACTER STEPS BACK TO THAT CHARACTER'S START.**
/// `MAX_WHY_BYTES` (512) falls two bytes into a three-byte character of a why
/// made of them (512 = 3 * 170 + 2), so the cut steps back twice and keeps
/// 170 whole characters, 510 bytes: never a split character, never past the
/// bound. (The module's own test cuts a why of two-byte characters, whose
/// 512th byte is already a boundary, so the step back never ran.)
#[test]
fn a_why_cut_inside_a_character_steps_back_to_its_start() {
    let euro = '\u{20ac}';
    assert_eq!((euro.len_utf8(), MAX_WHY_BYTES % 3), (3, 2));
    let long = CheckOutcome::Failed(euro.to_string().repeat(MAX_WHY_BYTES));
    let line = check_line(&CheckRecord {
        run: "ZQ-RUN",
        at: 7,
        kind: "ZQ-KIND",
        step: "S-90",
        outcome: &long,
    });
    let kept = euro.to_string().repeat(170);
    assert_eq!(
        line,
        format!("v=1 at=7 run=ZQ-RUN kind=ZQ-KIND step=S-90 outcome=failed why={kept}\n")
    );
}
