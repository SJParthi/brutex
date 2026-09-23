//! Explicit Boolean signal evaluation, separate from Apriori AND search.
//!
//! A caller records the expression identity before building/evaluating its
//! column and supplies a fallible evidence visitor. Only definitely true rows
//! are signals. This module neither invents missing evidence nor relabels a
//! Boolean expression as a legacy AND mask or an admitted trading strategy.

use indicators::column::Column;
use vocab::ConditionMask;
pub use vocab::expression::{
    ENCODED_LEN, Expression, MAX_INSTRUCTIONS, Refusal as ParseRefusal, Truth, VERSION,
};

/// Nine-term identity with the complete versioned program in its params term.
/// Legacy AND identities are unchanged; expression bytes are never truncated.
#[must_use]
pub fn identity(run: &crate::identity::Run<'_>, expression: &Expression) -> [u8; 32] {
    crate::identity::identity_with_expression(run, expression).bytes()
}

/// Identity of the complete versioned expression language over one alphabet.
/// Only its initial descriptor is bound; later progress is checkpoint evidence.
#[must_use]
pub fn search_identity(
    run: &crate::identity::Run<'_>,
    initial: &vocab::expression_search::Cursor,
) -> [u8; 32] {
    crate::identity::identity_with_expression_search(run, initial).bytes()
}

/// Reconciled counts for every evaluated row, without retaining the history.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// All rows whose evidence visitor accepted the result.
    pub evaluated: u64,
    /// Definite signal hits.
    pub hits: u64,
    /// Definite misses.
    pub misses: u64,
    /// Rows that cannot settle the expression from available evidence.
    pub unknown: u64,
}

impl Summary {
    /// Every accepted row belongs to exactly one bucket.
    #[must_use]
    pub fn reconciles(self) -> bool {
        self.hits
            .checked_add(self.misses)
            .and_then(|n| n.checked_add(self.unknown))
            == Some(self.evaluated)
    }
}

/// An inconsistent column or a failure saving one of its rows.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal<E> {
    /// Truth, known and source rows have different lengths.
    Alignment,
    /// Source identities are not strictly increasing.
    SourceOrder,
    /// Count cannot fit its fixed-width field.
    CountOverflow,
    /// The caller refused the current row; no complete result is returned.
    Evidence(E),
}

/// Evaluate the exact column while preserving source identities and unknowns.
///
/// # Errors
/// Column alignment/order, count overflow, or the first visitor failure.
pub fn evaluate<E>(
    column: &Column,
    expression: &Expression,
    visit: impl FnMut(usize, Truth) -> Result<(), E>,
) -> Result<Summary, Refusal<E>> {
    evaluate_rows(
        column.bits(),
        column.known(),
        column.sources(),
        expression,
        visit,
    )
}

/// Shared streaming kernel. Per row is bounded by the fixed program capacity;
/// the complete pass is O(rows), and auxiliary memory is fixed.
///
/// # Errors
/// Column alignment/order, count overflow, or the first visitor failure.
pub fn evaluate_rows<E>(
    truth: &[ConditionMask],
    known: &[ConditionMask],
    sources: &[usize],
    expression: &Expression,
    mut visit: impl FnMut(usize, Truth) -> Result<(), E>,
) -> Result<Summary, Refusal<E>> {
    if truth.len() != known.len() || truth.len() != sources.len() {
        return Err(Refusal::Alignment);
    }
    // Validate shape before publishing any row. This is a full linear pass,
    // explicitly separate from the constant per-row expression operation.
    if sources.windows(2).any(|pair| pair.first() >= pair.get(1)) {
        return Err(Refusal::SourceOrder);
    }
    let mut summary = Summary::default();
    for ((truth, known), &source) in truth.iter().zip(known).zip(sources) {
        let verdict = expression.evaluate(*truth, *known);
        visit(source, verdict).map_err(Refusal::Evidence)?;
        summary.evaluated = summary
            .evaluated
            .checked_add(1)
            .ok_or(Refusal::CountOverflow)?;
        let count = match verdict {
            Truth::True => &mut summary.hits,
            Truth::False => &mut summary.misses,
            Truth::Unknown => &mut summary.unknown,
        };
        *count = count.checked_add(1).ok_or(Refusal::CountOverflow)?;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_identity_binds_the_program_and_every_runtime_term() -> Result<(), String> {
        use crate::identity::{Direction, Params, Run};
        use brutex_core::instrument::{Exchange, InstrumentKey};
        let parse = |text| Expression::parse(text).map_err(|e| format!("{e:?}"));
        let expression = parse("0 | !369")?;
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(|e| format!("{e:?}"))?;
        let bank =
            InstrumentKey::index(Exchange::Nse, "BANKNIFTY").map_err(|e| format!("{e:?}"))?;
        let base = || Run {
            mask: expression.referenced(),
            direction: Direction::Undirected,
            instrument: &key,
            timeframe: "1min",
            params: Params::of(engine::Ladder::with_min_hits(2)),
            data_digest: [1; 32],
            commit: "synthetic-test-commit",
            feed: "groww",
        };
        let expected = identity(&base(), &expression);
        assert_eq!(expected, identity(&base(), &parse("(!369) | (0)")?));
        assert_ne!(expected, crate::identity::identity(&base()).bytes());
        for source in ["0 & !369", "!0 | 369", "0 | 369"] {
            let other = parse(source)?;
            assert_eq!(expression.referenced(), other.referenced());
            assert_ne!(expected, identity(&base(), &other));
        }
        let mut changed = base();
        changed.mask = ConditionMask::ZERO;
        assert_ne!(expected, identity(&changed, &expression), "mask");
        let mut changed = base();
        changed.direction = Direction::Long;
        assert_ne!(expected, identity(&changed, &expression), "direction");
        let mut changed = base();
        changed.instrument = &bank;
        assert_ne!(expected, identity(&changed, &expression), "instrument");
        let mut changed = base();
        changed.timeframe = "5min";
        assert_ne!(expected, identity(&changed, &expression), "timeframe");
        for params in [
            Params::of(engine::Ladder::with_min_hits(3)),
            Params::of(engine::Ladder::with_min_hits(2).with_ceiling(7)),
            Params::of(engine::Ladder::with_min_hits(2).with_pair_budget(7)),
            Params::of(engine::Ladder::with_min_hits(2)).with_policy(&[7]),
        ] {
            let mut changed = base();
            changed.params = params;
            assert_ne!(expected, identity(&changed, &expression), "parameters");
        }
        let mut changed = base();
        changed.data_digest = [2; 32];
        assert_ne!(expected, identity(&changed, &expression), "data");
        let mut changed = base();
        changed.commit = "other-test-commit";
        assert_ne!(expected, identity(&changed, &expression), "commit");
        let mut changed = base();
        changed.feed = "dhan";
        assert_ne!(expected, identity(&changed, &expression), "feed");
        assert_eq!(expected, identity(&base(), &expression), "repeat");
        Ok(())
    }

    #[test]
    fn search_identity_binds_language_alphabet_and_market_inputs() -> Result<(), String> {
        use crate::identity::{Direction, Params, Run};
        use brutex_core::instrument::{Exchange, InstrumentKey};
        use vocab::expression_search::Cursor;
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(|e| format!("{e:?}"))?;
        let cursor = Cursor::new(&[0, 369]).map_err(|e| format!("{e:?}"))?;
        let base = || Run {
            mask: cursor.alphabet(),
            direction: Direction::Undirected,
            instrument: &key,
            timeframe: "1min",
            params: Params::of(engine::Ladder::with_min_hits(2)),
            data_digest: [1; 32],
            commit: "generated-test-commit",
            feed: "synthetic",
        };
        let expected = search_identity(&base(), &cursor);
        assert_eq!(
            expected,
            search_identity(
                &base(),
                &Cursor::new(&[0, 369]).map_err(|e| format!("{e:?}"))?
            )
        );
        assert_ne!(expected, crate::identity::identity(&base()).bytes());
        let explicit = Expression::parse("0 | 369").map_err(|e| format!("{e:?}"))?;
        assert_ne!(expected, identity(&base(), &explicit));
        let mut advanced = cursor.clone();
        let _ = advanced
            .advance(100, &mut 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            expected,
            search_identity(&base(), &advanced),
            "resuming the same search must retain its identity"
        );
        assert_ne!(
            expected,
            search_identity(&base(), &Cursor::new(&[0]).map_err(|e| format!("{e:?}"))?)
        );
        let mut changed = base();
        changed.feed = "other-generated-feed";
        assert_ne!(expected, search_identity(&changed, &cursor));
        let mut changed = base();
        changed.params = Params::of(engine::Ladder::with_min_hits(3));
        assert_ne!(expected, search_identity(&changed, &cursor));
        let mut changed = base();
        changed.data_digest = [2; 32];
        assert_ne!(expected, search_identity(&changed, &cursor));
        Ok(())
    }

    #[test]
    fn missing_evidence_never_becomes_a_negated_hit_and_visitor_failure_propagates()
    -> Result<(), ParseRefusal> {
        let expression = Expression::parse("!0")?;
        let bit = ConditionMask::ZERO.with_bit(0);
        let zero = ConditionMask::ZERO;
        let mut observed = Vec::new();
        let summary = evaluate_rows(
            &[bit, zero, zero],
            &[bit, bit, zero],
            &[3, 7, 99],
            &expression,
            |source, verdict| {
                observed.push((source, verdict));
                Ok::<_, ()>(())
            },
        );
        assert_eq!(
            summary,
            Ok(Summary {
                evaluated: 3,
                hits: 1,
                misses: 1,
                unknown: 1
            })
        );
        assert!(summary.is_ok_and(Summary::reconciles));
        assert_eq!(
            observed,
            vec![(3, Truth::False), (7, Truth::True), (99, Truth::Unknown)]
        );
        let mut calls = 0;
        assert_eq!(
            evaluate_rows(&[bit, zero], &[bit, bit], &[3, 7], &expression, |_, _| {
                calls += 1;
                Err("disk full")
            }),
            Err(Refusal::Evidence("disk full"))
        );
        assert_eq!(calls, 1);
        Ok(())
    }

    #[test]
    fn malformed_columns_refuse_before_any_evidence_is_published() -> Result<(), ParseRefusal> {
        let expression = Expression::parse("0")?;
        let rows = [ConditionMask::ZERO; 2];
        let mut calls = 0;
        let mut visitor = |_, _| {
            calls += 1;
            Ok::<_, ()>(())
        };
        assert_eq!(
            evaluate_rows(&rows, &[], &[0, 1], &expression, &mut visitor),
            Err(Refusal::Alignment)
        );
        assert_eq!(
            evaluate_rows(&rows, &rows, &[1, 1], &expression, &mut visitor),
            Err(Refusal::SourceOrder)
        );
        assert_eq!(
            evaluate_rows(&rows, &rows, &[1, 0], &expression, &mut visitor),
            Err(Refusal::SourceOrder)
        );
        assert_eq!(
            evaluate_rows(&[], &[], &[], &expression, &mut visitor),
            Ok(Summary::default())
        );
        assert_eq!(calls, 0);
        assert!(
            !Summary {
                evaluated: 0,
                hits: u64::MAX,
                misses: 1,
                unknown: 0
            }
            .reconciles()
        );
        Ok(())
    }
}
