//! Structural storage attacks plus a genuine, opaque upstream authority test.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "brutex-selection-v6-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("unique scratch directory");
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// This is only a structurally sealed frame. It cannot construct the production
// authority, whose sole constructor requires real retained Execution V4.
fn frame(seed: u8) -> Block {
    let mut block = [0; SELECTION_V6_BLOCK_BYTES];
    block[..16].copy_from_slice(MAGIC);
    block[16..20].copy_from_slice(&6_u32.to_le_bytes());
    block[56] = seed;
    let identity = block_identity(&block).expect("identity");
    block[24..56].copy_from_slice(&identity);
    let digest = seal(&block).expect("seal");
    block[SEAL_AT..].copy_from_slice(&digest);
    block
}

fn bounds(records: u64) -> SelectionV6Bounds {
    SelectionV6Bounds::new(records, records * BLOCK_BYTES).expect("finite bounds")
}

#[test]
fn physical_bounds_refuse_zero_short_and_overflow() {
    for (records, bytes) in [
        (0, 0),
        (0, BLOCK_BYTES),
        (1, BLOCK_BYTES - 1),
        (u64::MAX, u64::MAX),
    ] {
        assert!(SelectionV6Bounds::new(records, bytes).is_err());
    }
    assert!(SelectionV6Bounds::new(1, BLOCK_BYTES).is_ok());
}

#[test]
fn every_byte_is_authenticated_including_unused_padding_and_completion() {
    let original = frame(1);
    verify_block(&original).expect("control frame");
    for index in 0..SELECTION_V6_BLOCK_BYTES {
        let mut attacked = original;
        attacked[index] ^= 1;
        assert!(verify_block(&attacked).is_err(), "byte {index}");
    }
}

#[test]
fn reopen_reuse_and_append_preserve_exact_history_and_capacity() {
    let scratch = Scratch::new();
    let first = frame(1);
    let second = frame(2);
    assert!(persist(&scratch.0, bounds(2), &first).expect("first write"));
    require_committed(&scratch.0, bounds(2), &first).expect("fresh reopen");
    let first_bytes = std::fs::read(scratch.0.join(FILE_NAME)).expect("saved first");
    assert!(!persist(&scratch.0, bounds(2), &first).expect("exact reuse"));
    assert_eq!(
        std::fs::read(scratch.0.join(FILE_NAME)).expect("reused bytes"),
        first_bytes
    );
    assert!(persist(&scratch.0, bounds(2), &second).expect("second write"));
    let both = std::fs::read(scratch.0.join(FILE_NAME)).expect("history");
    assert_eq!(both.len(), 2 * SELECTION_V6_BLOCK_BYTES);
    assert!(both.starts_with(&first_bytes));
    require_committed(&scratch.0, bounds(2), &first).expect("old retained row");
    require_committed(&scratch.0, bounds(2), &second).expect("new retained row");
    assert!(require_committed(&scratch.0, bounds(2), &frame(3)).is_err());
    assert!(require_committed(&scratch.0, bounds(1), &first).is_err());
    assert!(persist(&scratch.0, bounds(2), &frame(3)).is_err());
    assert_eq!(
        std::fs::read(scratch.0.join(FILE_NAME)).expect("refused bytes"),
        both
    );
}

#[test]
fn incomplete_prefix_is_never_authority_and_only_exact_resume_appends() {
    let expected = frame(7);
    for len in [
        1,
        23,
        55,
        56,
        4096,
        SEAL_AT - 1,
        SEAL_AT,
        SEAL_AT + 1,
        SELECTION_V6_BLOCK_BYTES - 1,
    ] {
        let scratch = Scratch::new();
        let path = scratch.0.join(FILE_NAME);
        std::fs::write(&path, &expected[..len]).expect("interrupted prefix");
        assert!(
            require_committed(&scratch.0, bounds(1), &expected).is_err(),
            "prefix {len}"
        );
        assert!(persist(&scratch.0, bounds(1), &expected).expect("same source resume"));
        assert_eq!(std::fs::read(path).expect("completed bytes"), expected);
    }
    let scratch = Scratch::new();
    let foreign = frame(8);
    let path = scratch.0.join(FILE_NAME);
    std::fs::write(&path, &foreign[..4096]).expect("foreign prefix");
    assert!(persist(&scratch.0, bounds(1), &expected).is_err());
    assert_eq!(
        std::fs::read(path).expect("preserved foreign prefix"),
        &foreign[..4096]
    );
}

#[test]
fn duplicate_corrupt_wrong_version_and_trailing_partial_refuse_without_repair() {
    let first = frame(3);
    let mut corrupt = first;
    corrupt[400] ^= 1;
    let mut old_version = first;
    old_version[16..20].copy_from_slice(&5_u32.to_le_bytes());
    for bad in [
        [first.as_slice(), first.as_slice()].concat(),
        corrupt.to_vec(),
        old_version.to_vec(),
        [first.as_slice(), &[1]].concat(),
    ] {
        let scratch = Scratch::new();
        let path = scratch.0.join(FILE_NAME);
        std::fs::write(&path, &bad).expect("attacked ledger");
        assert!(require_committed(&scratch.0, bounds(3), &first).is_err());
        assert!(persist(&scratch.0, bounds(3), &first).is_err());
        assert_eq!(std::fs::read(path).expect("no hidden repair"), bad);
    }
    let scratch = Scratch::new();
    std::fs::write(scratch.0.join("global-selection-v5.bin"), first).expect("legacy file");
    assert!(require_committed(&scratch.0, bounds(1), &first).is_err());
}

#[test]
fn fixed_encoder_refuses_exhaustion_without_out_of_bounds_write() {
    let mut bytes = [99; 4];
    let mut encoder = Encoder {
        bytes: &mut bytes,
        at: 0,
    };
    assert!(encoder.number(1).is_err());
    encoder.bytes(&[1, 2, 3, 4]).expect("exact capacity");
    assert!(encoder.bytes(&[5]).is_err());
    assert_eq!(bytes, [1, 2, 3, 4]);
}

#[cfg(unix)]
#[test]
fn directories_symlinks_and_hardlink_aliases_refuse() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let target = scratch.0.join("target");
    std::fs::write(&target, frame(1)).expect("target");
    let linked = scratch.0.join(FILE_NAME);
    symlink(&target, &linked).expect("file symlink");
    assert!(open(&scratch.0, false).is_err());
    std::fs::remove_file(&linked).expect("unlink fixture");
    std::fs::hard_link(&target, &linked).expect("hard link");
    assert!(open(&scratch.0, true).is_err());
    std::fs::remove_file(&linked).expect("unlink fixture");
    std::fs::create_dir(&linked).expect("directory at file path");
    assert!(open(&scratch.0, false).is_err());
    let root_alias = scratch.0.join("root-alias");
    symlink(&scratch.0, &root_alias).expect("root symlink");
    assert!(checked_root(&root_alias).is_err());
    assert!(checked_root(&target).is_err());
}

fn execution_bounds() -> crate::execution_v4::ExecutionV4Bounds {
    use crate::execution_v4::{ExecutionV4Bounds, ExecutionV4FileBound};
    let bound = |stride, name| {
        ExecutionV4FileBound::new(1_000_000, 2_000_000_000, stride, name)
            .expect("fixture file bounds")
    };
    ExecutionV4Bounds::new(
        bound(
            crate::execution_v4::EXECUTION_V4_PARAMETER_BYTES,
            "parameters",
        ),
        bound(
            crate::execution_v4::EXECUTION_V4_PERCENTILE_BYTES,
            "percentiles",
        ),
        bound(
            crate::execution_v4::EXECUTION_V4_DISPOSITION_BYTES,
            "dispositions",
        ),
        bound(
            crate::execution_v4::EXECUTION_V4_COMPLETION_BYTES,
            "completions",
        ),
        1_000_000,
        1_000_000,
    )
    .expect("fixture bounds")
}

#[test]
fn genuine_execution_v4_authority_reaches_v6_and_corrupt_selection_refuses() {
    crate::step3_orchestrator::with_population_v6_evaluated_pair_fixture(
        |finalization, nifty, banknifty, population_root| {
            let upstream =
                crate::population_v6::bind_population_v6_source_v1(finalization, nifty, banknifty)?;
            let population = crate::population_v6::commit_population_v6(
                population_root,
                crate::population_v6::PopulationV6Bounds::new(4, 1_000_000, 512 * 1024 * 1024)?,
                upstream,
            )?
            .into_authority();
            let execution_root = Scratch::new();
            let execution = crate::execution_v4::commit_stored_execution_v4(
                &execution_root.0,
                execution_bounds(),
                population,
            )?;
            assert_eq!(execution.structural_receipt().parameter_count(), 4);
            let selection_root = Scratch::new();
            let policy = RankingPolicyV1::new(runner::topn::Weights::equal())
                .map_err(|why| format!("{why:?}"))?;
            let mut selection =
                commit_stored_selection_v6(&selection_root.0, bounds(4), execution, policy)?;
            assert!(selection.was_written());
            assert_ne!(selection.identity(), [0; 32]);
            let top = selection.top_twenty_five()?;
            let ten = selection.top_ten()?;
            assert!(top.starts_with(&ten));
            assert_eq!(ten.len(), top.len().min(10));
            assert!(top.len() <= 25);
            for (rank, winner) in top.iter().enumerate() {
                assert_eq!(winner.rank as usize, rank);
                assert!(winner.ranked.candidate.admitted);
                assert_ne!(winner.disposition_id, [0; 32]);
                assert_ne!(winner.selected_exit_digest, [0; 32]);
            }
            let path = selection_root.0.join(FILE_NAME);
            let original = std::fs::read(&path).map_err(|why| why.to_string())?;
            // Even a well-sealed structural impostor cannot replace the source
            // retained inside the authority and authorize a different ranking.
            std::fs::write(&path, frame(99)).map_err(|why| why.to_string())?;
            assert!(selection.top_twenty_five().is_err());
            std::fs::write(&path, original).map_err(|why| why.to_string())?;
            assert_eq!(selection.top_twenty_five()?, top);
            std::fs::remove_file(path).map_err(|why| why.to_string())?;
            assert!(selection.top_ten().is_err());
            Ok(())
        },
    )
    .expect("real source-retaining successor");
}
