#![cfg(test)]
//! Serialization only; actual evidence admission is owned by the CLI reader.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "finite exact projection assertions"
)]
use super::*;

#[test]
fn conservative_zero_keeps_unreduced_draw_denominator_without_inventing_shared_facts() {
    let projected = romano([2, u64::MAX, u64::MAX, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!(projected["classification"], "conservative-zero");
    assert_eq!(
        projected["probability"]["denominator"],
        u64::MAX.to_string()
    );
    assert_eq!(projected["probability"]["numerator"], u64::MAX.to_string());
    assert!(projected["shared"].is_null());
}

#[test]
fn measured_and_nonpositive_classes_keep_exact_subfamily_indices_and_numeric_bits() {
    for (class, number) in [(1, 2.5_f64), (3, -0.0)] {
        let projected = romano([
            class,
            100,
            100,
            1,
            9_007_199_254_740_993,
            7,
            number.to_bits(),
            2,
            3,
            100,
            99,
            100,
        ])
        .unwrap();
        assert_eq!(projected["shared"]["strategy"], "9007199254740993");
        assert_eq!(
            projected["shared"]["statistic"]["bits"],
            number.to_bits().to_string()
        );
        assert_eq!(projected["shared"]["initial"]["numerator"], "3");
        assert_eq!(projected["shared"]["adjusted"]["numerator"], "99");
        assert_eq!(projected["shared"]["stepdown_rank"], "7");
    }
    assert!(romano([0; 12]).is_err());
    assert!(romano([2, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
}
