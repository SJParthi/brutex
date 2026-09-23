//! Read-only lower bounds for the declared fixed Boolean syntax search.
//!
//! This is not a historical benchmark or a count of viable trading setups.
//! Every nonempty subset of the alphabet has a distinct sorted conjunction
//! using at most `2 * alphabet.len() - 1` instructions. The mixed grammar also
//! visits these expressions, without AND's support pruning. Consequently
//! `2^L - 1` is a lower bound, even before NOT, OR, repeated leaves and exits.

use vocab::expression::MAX_INSTRUCTIONS;

/// Versioned facts about the full live alphabet, independent of a work batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkModel {
    /// Number of conditions actually offered by the production evaluator.
    pub live_conditions: usize,
    /// Existing grammar wire capacity; not a selectable search depth.
    pub max_instructions: usize,
    /// Exact decimal lower bound on expressions per selected timeframe.
    pub conjunction_program_lower_bound: String,
    /// True proves the cumulative V1 u64 counters cannot reach full exhaustion.
    /// False is not proof that the rest of the grammar fits those counters.
    pub lower_bound_exceeds_cumulative_counter: bool,
}

/// Derive a bounded descriptor from the same alphabet used by `BITS=all`.
/// No bar, source receipt or result is read and no search is started.
///
/// # Errors
/// Invalid live alphabet or a changed representation that cannot hold the proof.
pub fn work_model() -> Result<WorkModel, String> {
    let alphabet = runner::live_positions();
    vocab::expression_search::Cursor::new(&alphabet).map_err(|why| format!("{why:?}"))?;
    let leaves = alphabet.len();
    let required = leaves
        .checked_mul(2)
        .and_then(|count| count.checked_sub(1))
        .ok_or("conjunction lower-bound capacity overflow")?;
    if required > MAX_INSTRUCTIONS {
        return Err("the grammar no longer admits the complete subset lower bound".into());
    }
    Ok(WorkModel {
        live_conditions: leaves,
        max_instructions: MAX_INSTRUCTIONS,
        conjunction_program_lower_bound: subset_count(leaves)?,
        lower_bound_exceeds_cumulative_counter: leaves > u64::BITS as usize,
    })
}

// Fixed decimal workspace is sufficient even for 2^384 - 1. No float or
// saturating counter can convert this population into a plausible small total.
fn subset_count(leaves: usize) -> Result<String, String> {
    if leaves > vocab::ConditionMask::BITS as usize {
        return Err("alphabet exceeds the fixed condition mask".into());
    }
    let mut digits = [0_u8; 116];
    for _ in 0..leaves {
        let mut carry = 1;
        for digit in &mut digits {
            let value = *digit * 2 + carry;
            *digit = value % 10;
            carry = value / 10;
        }
        if carry != 0 {
            return Err("conjunction lower-bound decimal capacity overflow".into());
        }
    }
    let text: String = digits
        .iter()
        .rev()
        .skip_while(|&&digit| digit == 0)
        .map(|&digit| char::from(b'0' + digit))
        .collect();
    Ok(if text.is_empty() { "0".into() } else { text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_integer_subsets_and_retains_large_exact_decimals() -> Result<(), String> {
        for leaves in 0..128 {
            assert_eq!(subset_count(leaves)?, ((1_u128 << leaves) - 1).to_string());
        }
        assert_eq!(subset_count(128)?, u128::MAX.to_string());
        assert_eq!(
            subset_count(328)?,
            "546812681195752981093125556779405341338292357723303109106442651602488249799843980805878294255763455"
        );
        assert_eq!(
            subset_count(384)?,
            "39402006196394479212279040100143613805079739270465446667948293404245721771497210611414266254884915640806627990306815"
        );
        assert!(subset_count(385).is_err());
        Ok(())
    }

    #[test]
    fn live_model_counts_the_runtime_alphabet_without_claiming_a_total_or_eta() -> Result<(), String>
    {
        let model = work_model()?;
        let alphabet = runner::live_positions();
        assert_eq!(model.live_conditions, alphabet.len());
        assert_eq!(model.max_instructions, MAX_INSTRUCTIONS);
        assert_eq!(
            model.conjunction_program_lower_bound,
            subset_count(alphabet.len())?
        );
        assert_eq!(
            model.lower_bound_exceeds_cumulative_counter,
            alphabet.len() > 64
        );
        // A sorted conjunction fits even when every available bit participates.
        let source = alphabet
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" & ");
        let expression =
            vocab::expression::Expression::parse(&source).map_err(|why| format!("{why:?}"))?;
        assert_eq!(
            expression.referenced().popcount(),
            u32::try_from(alphabet.len()).map_err(|why| why.to_string())?
        );
        Ok(())
    }
}
