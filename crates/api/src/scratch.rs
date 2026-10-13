//! Test-only temporary paths, unique to the process that asks for one.
//!
//! # The failure this exists to remove
//!
//! `master::tests::a_master_larger_than_this_reader_holds_is_refused_before_it_is_read`
//! failed intermittently — about one run in three — with
//! `cleanup: Os { code: 2, kind: NotFound }` on the line that deletes its own
//! fixture. The fixture path was
//! `std::env::temp_dir().join("brutex-master-toobig.csv")`: **one fixed name in
//! a directory shared by every process on the machine.**
//!
//! The test creates a 256 MiB sparse file at that path, calls `load` twice —
//! once over the bound and once at it, and the second call reads all 256 MiB —
//! and then deletes it. Any second process running the same test binary
//! deletes the file inside that window. Two concurrent `cargo test` runs, a
//! `cargo llvm-cov` run beside a `cargo test`, or two agents working the same
//! checkout are all enough, and it was reproduced deliberately: two copies of
//! the test binary started together, one failed and one passed.
//!
//! The assertions were never wrong. `MAX_MASTER_BYTES + 1` is refused and
//! `MAX_MASTER_BYTES` exactly is not, and both are still asserted unchanged.
//! What was wrong was a fixture that two processes could both claim.
//!
//! # The rule
//!
//! Every temporary path this crate's tests touch goes through [`path`], which
//! stamps the process id into the name. Two processes therefore never name the
//! same file, and a test remains free to delete what it created — which is the
//! only way `remove_file` can be an assertion rather than a hope.
//!
//! The process id is not a random number and is not meant to be: it is unique
//! among *live* processes, which is exactly the set that can collide. A stale
//! file from a dead process with the same id is overwritten by `File::create`,
//! which truncates.

use std::path::PathBuf;

/// A temporary path no other live process will name.
///
/// `name` should describe the fixture, not the crate — the `brutex-` prefix
/// and the process id are added here so every caller gets both.
pub(crate) fn path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("brutex-{}-{name}", std::process::id()))
}

/// Plants beside `bars` the `.srt` sort index the store's writer kept before
/// the small-month rule (D-4933), for a month written from empty in appends
/// of `chunk` rows: one whole run over the first append's rows and one
/// appended run per later append — byte for byte what that writer left, and
/// the shape of every month the real store holds from before the rule.
///
/// Since D-4933 the writer gives a month under 1,024 bars no `.srt`, and a
/// sorted page over one orders its rows itself; the tests that call this are
/// about months that HAVE their runs. A month whose writer already kept a
/// `.srt` (one of 1,024 bars or more, or one this already planted and a later
/// append extended) is left as it is.
#[allow(
    clippy::expect_used,
    reason = "a fixture that cannot be planted is a test that cannot run, and says why"
)]
pub(crate) fn plant_pre_rule_sort_index(
    bars: &std::path::Path,
    shape: &store::sort_index::Shape,
    rows: &[store::format::Bar],
    chunk: usize,
) {
    use store::sort_index::{RunKind, build_run_as};
    let srt = bars.with_extension("srt");
    if srt.exists() || rows.is_empty() {
        return;
    }
    let mut image = shape.encode().to_vec();
    let mut start = 0_u64;
    for (k, piece) in rows.chunks(chunk.max(1)).enumerate() {
        let kind = if k == 0 {
            RunKind::Whole
        } else {
            RunKind::Appended
        };
        image.extend(build_run_as(shape, start, piece, kind).expect("a run over invented rows"));
        start += u64::try_from(piece.len()).expect("a test month fits u64");
    }
    std::fs::write(&srt, image).expect("the planted sort index");
}

/// Plants beside the month `path` names the `.tix` time index the store's
/// writer kept before the small-month rule (D-4933): every month it opened
/// had one, so every month the real store holds from before the rule does.
///
/// Since D-4933 a month under 1,024 bars is given none, and its time lookups
/// bisect the bars; a test about a month that HAS its index — a pre-rule
/// month, the commonest month on disk — calls this. The index is built by the
/// store's own rebuild door from the committed bars, so its bytes are the
/// ones a writer keeping it in step would have left: an empty file is put at
/// the name first, and the door rebuilds what is there rather than excusing
/// it. A month that already has an index is left as it is. The month's
/// writer must be closed: the door takes the month's lock.
#[allow(
    clippy::expect_used,
    reason = "a fixture that cannot be planted is a test that cannot run, and says why"
)]
pub(crate) fn plant_pre_rule_time_index(
    root: &std::path::Path,
    path: store::path::StorePath<'_>,
    symbol_id: u32,
) {
    let tix = path
        .with_file(store::path::FileKind::TimeIndex)
        .to_path_buf(root);
    if tix.exists() {
        return;
    }
    std::fs::write(&tix, []).expect("an empty file at the index's name");
    let outcome = store::file::BarFile::index_existing(root, path, symbol_id)
        .expect("the rebuild door opens the month")
        .outcome;
    assert!(
        matches!(outcome, store::file::Reindexed::Built(_)),
        "the planted time index was not built: {outcome:?}"
    );
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;

    #[test]
    fn two_names_differ_and_both_carry_this_process_and_never_the_other() {
        let a = path("alpha");
        let b = path("beta");
        assert_ne!(a, b, "two fixtures must not share one path");
        let pid = std::process::id().to_string();
        for p in [&a, &b] {
            let text = p.to_string_lossy().into_owned();
            assert!(
                text.contains(&pid),
                "a fixture that does not name its process can be deleted by another: {text}"
            );
            assert!(
                text.contains("brutex-"),
                "and it must be identifiable: {text}"
            );
        }
        assert_eq!(
            a.parent(),
            Some(std::env::temp_dir().as_path()),
            "fixtures live in the temporary directory and nowhere else"
        );
        // The same name asked for twice is the same path — a test that writes
        // then reads must reach its own file.
        assert_eq!(path("alpha"), a);
    }
}
