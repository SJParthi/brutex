#![cfg(test)]
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
fn incomplete_prefix_is_never_authority_and_a_foreign_one_is_set_aside() {
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
    // D-1569: a foreign unsealed prefix is moved aside whole, never lost.
    assert!(persist(&scratch.0, bounds(1), &expected).expect("foreign tail set aside"));
    assert_eq!(std::fs::read(path).expect("completed bytes"), expected);
    assert_eq!(
        std::fs::read(scratch.0.join(format!("{FILE_NAME}.abandoned-0")))
            .expect("preserved foreign prefix"),
        &foreign[..4096]
    );
}

#[test]
fn duplicate_corrupt_and_wrong_version_refuse_without_repair() {
    let first = frame(3);
    let mut corrupt = first;
    corrupt[400] ^= 1;
    let mut old_version = first;
    old_version[16..20].copy_from_slice(&5_u32.to_le_bytes());
    for bad in [
        [first.as_slice(), first.as_slice()].concat(),
        corrupt.to_vec(),
        old_version.to_vec(),
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

/// **An abandoned partial tail neither hides committed history nor wedges a
/// new source.** audit-20261003 hunt-cli-a-5, D-1569.
///
/// One interrupted persist left an unsealed partial block after a committed
/// one. Every read of the committed block then refused, and a different
/// source could never be appended: "incomplete prefix belongs to different
/// source; nothing repaired". The partial tail is now never read as authority,
/// committed blocks before it stay readable, and the next writer, under the
/// exclusive lock, moves the abandoned bytes aside into a named quarantine
/// file before appending, so nothing is lost and nothing is hidden.
#[test]
fn an_abandoned_partial_tail_neither_hides_committed_history_nor_wedges_a_new_source() {
    let scratch = Scratch::new();
    let first = frame(11);
    let foreign = frame(12);
    let second = frame(13);
    assert!(persist(&scratch.0, bounds(3), &first).expect("first write"));
    let path = scratch.0.join(FILE_NAME);
    let mut torn = std::fs::read(&path).expect("committed first");
    torn.extend_from_slice(&foreign[..4096]);
    std::fs::write(&path, &torn).expect("interrupted foreign persist");

    require_committed(&scratch.0, bounds(3), &first)
        .expect("a committed block stays readable behind an unsealed tail");
    assert!(
        require_committed(&scratch.0, bounds(3), &foreign).is_err(),
        "an unsealed tail is never authority"
    );
    assert!(persist(&scratch.0, bounds(3), &second).expect("a new source appends"));
    assert_eq!(
        std::fs::read(&path).expect("repaired history"),
        [first.as_slice(), second.as_slice()].concat()
    );
    let quarantine = scratch
        .0
        .join(format!("{FILE_NAME}.abandoned-{SELECTION_V6_BLOCK_BYTES}"));
    assert_eq!(
        std::fs::read(&quarantine).expect("abandoned bytes kept aside"),
        &foreign[..4096]
    );
    require_committed(&scratch.0, bounds(3), &first).expect("first retained");
    require_committed(&scratch.0, bounds(3), &second).expect("second committed");
    // Exact reuse behind a tail also clears the tail aside, then reuses.
    let mut torn = std::fs::read(&path).expect("two committed");
    torn.extend_from_slice(&foreign[..10]);
    std::fs::write(&path, &torn).expect("second interruption");
    assert!(!persist(&scratch.0, bounds(3), &second).expect("exact reuse"));
    assert_eq!(
        std::fs::read(&path).expect("tail moved aside"),
        [first.as_slice(), second.as_slice()].concat()
    );
}

/// Reseal a block after a deliberate edit, as a forger with the format would.
fn resealed(mut block: Block) -> Block {
    let identity = block_identity(&block).expect("identity");
    block[24..56].copy_from_slice(&identity);
    let digest = seal(&block).expect("seal");
    block[SEAL_AT..].copy_from_slice(&digest);
    block
}

/// **The display reader decodes a genuine committed block to the authority's
/// own winners, and refuses any family but the two.** D-1578,
/// audit-20261003 gaps-10.
///
/// The block is the one `commit_stored_selection_v6` writes from a genuine
/// Execution V4 authority. Read back through `read_stored_selection_v6` from
/// the `ROOT/selection/<rung>/` layout `ledger-v6` writes, every winner must
/// equal `top_twenty_five` field for field: rank, family, digests, side,
/// mask, score and metrics. Other rungs read as absent and name their path.
///
/// A sealed block whose family envelope or winner names any code but NIFTY's
/// 1 or BANKNIFTY's 2 is refused with the `CLAUDE.md` §1 sentence, never
/// shown under a guessed name, and so is an equity asked for by name.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one genuine fixture carries the decode, the layout and both refusals"
)]
fn the_display_reader_decodes_the_authoritys_winners_and_refuses_any_other_family() {
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
            let selection_root = Scratch::new();
            let policy = RankingPolicyV1::new(runner::topn::Weights::equal())
                .map_err(|why| format!("{why:?}"))?;
            let mut selection =
                commit_stored_selection_v6(&selection_root.0, bounds(4), execution, policy)?;
            let top = selection.top_twenty_five()?;
            let bytes = std::fs::read(selection_root.0.join(FILE_NAME)).map_err(|e| e.to_string())?;
            let block: Block = bytes.as_slice().try_into().map_err(|_| "one block")?;
            let record = read::decode_block(&block)?;
            assert_eq!(record.identity, selection.identity());
            assert_eq!(record.winners.len(), top.len());
            for (stored, authority) in record.winners.iter().zip(&top) {
                assert_eq!(stored.rank, authority.rank);
                assert_eq!(stored.family, authority.family);
                assert_eq!(stored.disposition_id, authority.disposition_id);
                assert_eq!(stored.selected_exit_digest, authority.selected_exit_digest);
                let candidate = authority.ranked.candidate;
                assert_eq!(stored.strategy_digest, candidate.strategy_digest.bytes());
                assert_eq!(stored.mask_words, candidate.mask_words);
                assert_eq!(stored.score, authority.ranked.score);
                assert_eq!(
                    stored.direction,
                    match candidate.direction {
                        costs::fill::Direction::Long => "long",
                        costs::fill::Direction::Short => "short",
                    }
                );
                let m = candidate.metrics;
                assert_eq!(
                    (stored.drawdown, stored.worst_loss, stored.pessimistic_profit),
                    (m.drawdown, m.worst_loss, m.pessimistic_profit)
                );
                assert_eq!(
                    (stored.loss_ratio_ppm, stored.reward_to_risk_ppm),
                    (m.loss_ratio_ppm, m.reward_to_risk_ppm)
                );
            }
            assert_eq!(
                record.families.map(|f| f.family),
                ["NIFTY", "BANKNIFTY"]
            );

            // THE LAYOUT `ledger-v6` WRITES.
            let rung = crate::ledger_all::LEDGER_RUNGS
                .into_iter()
                .find(|rung| {
                    crate::stored::rung_length_micros(rung).ok()
                        == i64::try_from(record.rung_seconds).ok().map(|s| s * 1_000_000)
                })
                .ok_or("the fixture's rung is a ledger rung")?;
            let ledger_root = Scratch::new();
            let directory = ledger_root.0.join("selection").join(rung);
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            std::fs::write(directory.join(FILE_NAME), &bytes).map_err(|e| e.to_string())?;
            for (name, read) in read_stored_selection_v6(&ledger_root.0, 4) {
                match read {
                    StoredSelectionV6Rung::Records(records) => {
                        assert_eq!(name, rung);
                        assert_eq!(records, vec![record.clone()]);
                    }
                    StoredSelectionV6Rung::Absent(path) => {
                        assert_ne!(name, rung);
                        assert!(path.ends_with(&format!("selection/{name}/{FILE_NAME}")));
                    }
                    StoredSelectionV6Rung::Refused(why) => panic!("{name}: {why}"),
                }
            }
            // A FILE OVER THE READER'S BOUND IS REFUSED WHOLE, NOT CUT.
            std::fs::write(directory.join(FILE_NAME), [bytes.as_slice(), &frame(7)].concat())
                .map_err(|e| e.to_string())?;
            let over = read_stored_selection_v6(&ledger_root.0, 1);
            assert!(over.iter().any(|(name, read)| *name == rung
                && matches!(read, StoredSelectionV6Rung::Refused(why) if why.contains("above the 1"))));

            // ANY OTHER FAMILY CODE, RESEALED, IS REFUSED BY NAME.
            let mut envelope = block;
            envelope[672..680].copy_from_slice(&3_u64.to_le_bytes());
            let why = read::decode_block(&resealed(envelope)).expect_err("family 3 refused");
            assert!(why.contains(SELECTION_V6_EQUITY_REFUSAL), "{why}");
            if !record.winners.is_empty() {
                let mut winner = block;
                winner[1056..1064].copy_from_slice(&3_u64.to_le_bytes());
                let why = read::decode_block(&resealed(winner)).expect_err("winner family refused");
                assert!(why.contains(SELECTION_V6_EQUITY_REFUSAL), "{why}");
            }
            Ok(())
        },
    )
    .expect("real source-retaining successor");
    for word in ["RELIANCE", "TCS"] {
        let why = selection_v6_family(word).expect_err("an equity is refused");
        assert!(
            why.starts_with(&format!("{word} is a cash equity.")),
            "{why}"
        );
        assert!(why.contains(SELECTION_V6_EQUITY_REFUSAL));
    }
    assert!(selection_v6_family("INDIAVIX").is_err());
    assert_eq!(selection_v6_family("NIFTY"), Ok("NIFTY"));
    assert_eq!(selection_v6_family("BANKNIFTY"), Ok("BANKNIFTY"));
}
