//! IA-16's tests of the one audit-ledger append (D-4888, D-4890, D-4906;
//! RA-125, IA-GR-41, bench C-GR-15). Every line is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brutex_core::vendor::Vendor;

use super::{append, append_within, lock_path};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-audit-ledger-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn ledger(root: &Path) -> String {
    std::fs::read_to_string(root.join("audit").join("gdfl.repairs")).unwrap_or_default()
}

/// **IA-GR-41 (D-4888, RA-125): TWO WRITERS OF ONE LEDGER NEVER TEAR OR JOIN
/// A LINE.** Two threads, each with its own descriptors (an `flock` is held
/// per open file, so they contend exactly as two programs do), append 1,000
/// lines each to `audit/gdfl.repairs`; every line is whole and every one of
/// the 2,000 is there once.
#[test]
fn two_writers_of_one_ledger_leave_every_line_whole() {
    let root = scratch("two");
    let writers: Vec<_> = ["a", "b"]
        .into_iter()
        .map(|who| {
            let root = root.clone();
            std::thread::spawn(move || {
                for n in 0..1_000 {
                    let line = format!("repair writer={who} n={n} pad={}", "x".repeat(n % 97));
                    append(&root, Vendor::Gdfl, "repairs", &[line]).unwrap();
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    let text = ledger(&root);
    assert!(text.ends_with('\n'), "the last line is whole");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2_000);
    let mut seen = std::collections::HashSet::new();
    for line in &lines {
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!(fields.len(), 4, "a joined or torn line: {line:?}");
        assert_eq!(fields[0], "repair");
        let n: usize = fields[2].strip_prefix("n=").unwrap().parse().unwrap();
        assert_eq!(fields[3], format!("pad={}", "x".repeat(n % 97)), "{line:?}");
        assert!(seen.insert((fields[1].to_owned(), n)), "twice: {line:?}");
    }
}

/// A crash's fragment (a last line with no line break) is closed first, so
/// it stays one malformed line of its own and the next line is whole.
#[test]
fn a_torn_tail_is_closed_never_joined() {
    let root = scratch("torn");
    std::fs::create_dir_all(root.join("audit")).unwrap();
    std::fs::write(
        root.join("audit").join("gdfl.repairs"),
        "repair whole=1\nrepair tor",
    )
    .unwrap();
    append(
        &root,
        Vendor::Gdfl,
        "repairs",
        &["repair next=1".to_owned()],
    )
    .unwrap();
    assert_eq!(ledger(&root), "repair whole=1\nrepair tor\nrepair next=1\n");
}

/// A name that is not one lower-case word, and a root that is not a
/// directory (an unplugged drive's mount point, FM-125), are refused by name
/// and nothing is created; no lines is no write at all.
#[test]
fn a_bad_name_or_a_missing_root_is_refused_and_nothing_is_created() {
    let root = scratch("refused");
    for name in ["", "Repairs", "re pairs", "../escape", "a.b", "r\n"] {
        let why = append(&root, Vendor::Gdfl, name, &["l".to_owned()]).unwrap_err();
        assert!(why.contains("not an audit ledger name"), "{name:?}: {why}");
    }
    assert!(
        !root.join("audit").exists(),
        "nothing written for a bad name"
    );
    let gone = root.join("unplugged");
    let why = append(&gone, Vendor::Gdfl, "repairs", &["l".to_owned()]).unwrap_err();
    assert!(why.contains("not a directory"), "{why}");
    assert!(!gone.exists(), "a root that went away is never recreated");
    append(&root, Vendor::Gdfl, "repairs", &[]).unwrap();
    assert!(!root.join("audit").exists(), "no lines, no write");
}

/// **D-4906: NO LEDGER LOCK WAITS FOR EVER.** A holder that never lets go
/// (a stalled drive, a process stopped mid-append) is waited on for the
/// bound, then the append is refused by name with the lock's path, and the
/// ledger is untouched.
#[test]
fn a_stalled_holder_is_refused_past_the_wait_by_name() {
    let root = scratch("stalled");
    std::fs::create_dir_all(root.join("audit")).unwrap();
    let path = lock_path(&root, Vendor::Gdfl, "repairs");
    let held = crate::gdfl_run::locks::take_path_within(&path, Duration::ZERO).unwrap();
    let began = Instant::now();
    let why = append_within(
        &root,
        Vendor::Gdfl,
        "repairs",
        &["repair while=held".to_owned()],
        Duration::from_millis(150),
    )
    .unwrap_err();
    let waited = began.elapsed();
    assert!(waited >= Duration::from_millis(150), "{waited:?}");
    assert!(waited < Duration::from_secs(10), "{waited:?}");
    assert!(why.contains("gdfl.repairs.lock"), "{why}");
    assert!(why.contains("not written"), "{why}");
    assert_eq!(ledger(&root), "");
    drop(held);
    append_within(
        &root,
        Vendor::Gdfl,
        "repairs",
        &["repair after=release".to_owned()],
        Duration::from_millis(150),
    )
    .unwrap();
    assert_eq!(ledger(&root), "repair after=release\n");
}

/// A line breaking the line rules is the ledger's own refusal, and nothing
/// of the batch is written.
#[test]
fn a_line_that_breaks_the_rules_writes_nothing() {
    let root = scratch("rules");
    let why = append(
        &root,
        Vendor::Gdfl,
        "repairs",
        &["repair ok=1".to_owned(), "two\nlines".to_owned()],
    );
    assert!(why.is_err(), "{why:?}");
    assert_eq!(ledger(&root), "");
}

/// **Bench C-GR-15: AN APPEND COSTS THE SAME AT ANY LEDGER LENGTH.** 200
/// appends to an empty ledger, then 200 to one of 100,000 lines (about
/// 4 MB): the p50 and p99 of the long ledger stay within a small factor of
/// the short one's (one lock, a one-byte tail read, one write, one sync:
/// nothing scans the file). Printed as `C-GR-15 p50= p99= max= n=`.
#[test]
fn appends_cost_the_same_at_any_ledger_length() {
    fn timed(root: &Path) -> Vec<Duration> {
        (0..200)
            .map(|n| {
                let began = Instant::now();
                append(
                    root,
                    Vendor::Gdfl,
                    "repairs",
                    &[format!("repair timed={n}")],
                )
                .unwrap();
                began.elapsed()
            })
            .collect()
    }
    fn pct(mut all: Vec<Duration>) -> (Duration, Duration, Duration) {
        all.sort_unstable();
        let at = |q: usize| all[(all.len() * q / 100).min(all.len() - 1)];
        (at(50), at(99), *all.last().unwrap())
    }
    let short = scratch("bench-short");
    let long = scratch("bench-long");
    std::fs::create_dir_all(long.join("audit")).unwrap();
    let mut text = String::with_capacity(4_200_000);
    for n in 0..100_000 {
        let _ = writeln!(text, "repair filler={n} pad=abcdefghijklmnop");
    }
    std::fs::write(long.join("audit").join("gdfl.repairs"), &text).unwrap();
    let (s50, s99, smax) = pct(timed(&short));
    let (l50, l99, lmax) = pct(timed(&long));
    println!("C-GR-15 short p50={s50:?} p99={s99:?} max={smax:?} n=200");
    println!("C-GR-15 long(100000 lines) p50={l50:?} p99={l99:?} max={lmax:?} n=200");
    // A scan of 4 MB per append would cost milliseconds more than the
    // short ledger at the median; a sync dominates both.
    assert!(
        l50 <= s50 * 4 + Duration::from_millis(2),
        "long p50 {l50:?} vs short {s50:?}"
    );
    assert_eq!(ledger(&long).lines().count(), 100_200);
}
