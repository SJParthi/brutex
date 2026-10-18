//! Synthetic FIB fixtures shared by this module's tests. Every value is
//! invented; nothing here is a market fact.

#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built is a failed test"
)]

use vocab::fib_cube::{
    CutList, EntryVariants, Generation, GenerationChain, GenerationParts, Ratio, RowId, Search,
    Source, VariantSet,
};

/// A valid generation: one seed cut per measured row, one time cut per width
/// at the width's first candle, edges `[3, 9]`, four exit variants and the
/// three entry styles.
pub(super) fn generation(
    search: Search,
    version: u32,
    birth_step: u32,
    widths: &[u16],
) -> Generation {
    let mut cuts = Vec::new();
    for id in RowId::ALL {
        if id.row().source != Source::Measured || id == RowId::MoverRank {
            continue;
        }
        if id == RowId::TimeOfDay {
            for minutes in widths {
                cuts.push(CutList {
                    row: id,
                    timeframe: Some(*minutes),
                    base_kind: None,
                    cuts: vec![Ratio::whole(u64::from(*minutes) * 60)],
                });
            }
        } else {
            cuts.push(CutList {
                row: id,
                timeframe: None,
                base_kind: None,
                cuts: vec![Ratio::new(1, 2).expect("a half")],
            });
        }
    }
    Generation::new(GenerationParts {
        search,
        version,
        birth_step,
        timeframes: widths.to_vec(),
        rank_edges: vec![3, 9],
        cuts,
        exit_set: VariantSet {
            version: 1,
            digest: [0x11; 32],
            count: 4,
        },
        entry_variants: EntryVariants {
            version: 1,
            digest: [0x22; 32],
            styles: vec![0, 1, 2],
        },
        others: Vec::new(),
    })
    .expect("the fixture is a valid generation")
}

/// A chain of two long versions: widths `[1, 2, 4]` born at step 6, then
/// `[1, 2, 3, 4]` born at step 9.
pub(super) fn chain() -> GenerationChain {
    let mut chain = GenerationChain::new(generation(Search::Long, 1, 6, &[1, 2, 4]));
    chain
        .push(generation(Search::Long, 2, 9, &[1, 2, 3, 4]))
        .expect("the next version");
    chain
}
