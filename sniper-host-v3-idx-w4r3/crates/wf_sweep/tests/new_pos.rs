//! POSITIONAL v2, B4 (design 6.3): `Layout::new_pos` takes its LEAVES from the groups' booking months and its COUNTED months (K9's
//! denominator: every counted learning month positive) from the SESSIONS, as common_wf `statics()` counts them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, missing_docs)]

use wf_sweep::acc::Layout;

const fn mk(y: i32, m: i32) -> i32 {
    y * 12 + m - 1
}

#[test]
fn b4_a_learning_month_with_no_booking_is_still_counted_by_k9() {
    // Sessions in every month of 2019; groups booked in every month but February (a learning month with no booking).
    let sess: Vec<i32> = (1..=12).flat_map(|m| std::iter::repeat_n(mk(2019, m), 20)).collect();
    let leaf: Vec<i32> = (1..=12).filter(|&m| m != 2).map(|m| mk(2019, m)).collect();
    let wins = vec![("WF".to_string(), mk(2019, 1), mk(2019, 12))];
    let l = Layout::new_pos(&leaf, &sess, (mk(2019, 1), mk(2019, 12)), &wins).unwrap();
    // February is counted: "every counted learning month positive" fails on it, as in common_wf.
    assert_eq!(l.counted, vec![12]);
    // The cash Layout built from the groups would count 11 (the B4 defect).
    let c = Layout::new(&leaf, (mk(2019, 1), mk(2019, 12)), &wins).unwrap();
    assert_eq!(c.counted, vec![11]);
    // Leaves come from the groups in both: one leaf per group inside the hull.
    assert_eq!(l.leaves, c.leaves);
    assert_eq!(l.leaves.iter().sum::<usize>(), 11);
}

#[test]
fn b4_a_group_with_no_priced_row_keeps_no_leaf() {
    // i32::MAX is the host's month for a group with no priced row: outside the hull, no leaf.
    let sess: Vec<i32> = (1..=12).map(|m| mk(2019, m)).collect();
    let leaf = vec![mk(2019, 1), i32::MAX, mk(2019, 3)];
    let wins = vec![("WF".to_string(), mk(2019, 1), mk(2019, 12))];
    let l = Layout::new_pos(&leaf, &sess, (mk(2019, 1), mk(2019, 12)), &wins).unwrap();
    assert_eq!(l.leaves.iter().sum::<usize>(), 2);
}
