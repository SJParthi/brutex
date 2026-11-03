//! BASE-COV-LIBS-LS4: the alias lookups' own-field checks, reached by a direct
//! construction.
//!
//! Every lookup probes its index once and then checks the row's own field, so
//! an index that disagrees with its rows answers nothing rather than the wrong
//! row. `build` can never make such a table: it keys each index on the very
//! field the check reads. The fields are private to `alias`, so a table built
//! field by field in this child module is the one way to show the check holds
//! (the recipe's "direct call"); without it the check's refusing side is never
//! run and nothing would notice it being deleted.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::{IndexRow, Indices, Lineage, Status, SymbolChange, Ymd};
use crate::symbol::SYMBOL_CAPACITY;
use crate::universe::MemberIndex;

/// A sourced status with no citation: it applies, and cites nothing a reader
/// could mistake for a real source.
const APPLIES: Status = Status::Sourced(&[]);

/// A status that never applies.
const WITHHELD: Status = Status::Unverified("a test row");

const fn row(fno: Option<&'static str>, store_key: &'static str, status: Status) -> IndexRow {
    IndexRow {
        fno,
        nse_name: "TEST INDEX",
        index_symbol: "TEST INDEX",
        kite: "TEST INDEX",
        store_key,
        reference_only: false,
        status,
    }
}

const fn change(old: &'static str, new: &'static str, status: Status) -> SymbolChange {
    SymbolChange {
        old,
        new,
        effective: Ymd::new(2019, 8, 19),
        status,
    }
}

/// Rows `rows`, with each index keyed on the names given, not on the rows.
fn skewed<const R: usize>(
    rows: [IndexRow; R],
    fnos: &[&'static str],
    keys: &[&'static str],
) -> Indices<R, 1, 8> {
    Indices {
        rows,
        names: [("TESTINDEX", 0)],
        by_fno: MemberIndex::build(fnos),
        by_store_key: MemberIndex::build(keys),
        by_name: MemberIndex::build(&["TESTINDEX"]),
    }
}

/// **A PROBE THAT LANDS ON A ROW NAMING ANOTHER SYMBOL ANSWERS NOTHING.** The
/// index finds position 0 for `BBB`; the row there says `AAA`; the lookup
/// refuses it. Each case is one extreme of the two checks `by_fno` and
/// `by_store_key` make after their probe.
#[test]
fn an_index_that_points_at_a_row_naming_another_symbol_answers_nothing() {
    let aaa = row(Some("AAA"), "AAAKEY", APPLIES);

    // 1. CONTROL: built by `build`, index and rows agree and both answer.
    let built: Indices<1, 1, 8> = Indices::build([aaa], [("TESTINDEX", 0)]);
    assert_eq!(built.by_fno("AAA"), Some(&aaa));
    assert_eq!(built.by_store_key("AAAKEY"), Some(&aaa));

    // 2. The F&O index names `BBB` where the row says `AAA`.
    let table = skewed([aaa], &["BBB"], &["BBBKEY"]);
    assert_eq!(table.by_fno("BBB"), None, "the row's own symbol is AAA");
    // 3. The store-key index names `BBBKEY` where the row says `AAAKEY`.
    assert_eq!(
        table.by_store_key("BBBKEY"),
        None,
        "the row's own key is AAAKEY"
    );
    // 4. The row's own names are not in that index, so they are not found.
    assert_eq!(table.by_fno("AAA"), None);
    assert_eq!(table.by_store_key("AAAKEY"), None);
    // 5. The by-name index still agrees with the names, and still answers.
    assert_eq!(table.by_name("test index"), Some(&aaa));

    // 6. COMBINED: a row with NO F&O symbol behind an index that gives it
    //    one. The index answers position 0, the row's own field is `None`,
    //    and `None` is never `Some("CCC")`.
    let vix_like = skewed([row(None, "VIXKEY", APPLIES)], &["CCC"], &["VIXKEY"]);
    assert_eq!(vix_like.by_fno("CCC"), None);
    assert_eq!(
        vix_like.by_store_key("VIXKEY").map(|r| r.fno),
        Some(None),
        "the agreeing key still answers the row"
    );

    // 7. COMBINED: an index that AGREES with a row that does not apply. The
    //    field check passes and the status check refuses: both halves of the
    //    filter are needed, and either alone would be wrong.
    let withheld = row(Some("DDD"), "DDDKEY", WITHHELD);
    let agreeing: Indices<1, 1, 8> = Indices::build([withheld], [("TESTINDEX", 0)]);
    assert_eq!(agreeing.by_fno("DDD"), None);
    assert_eq!(agreeing.by_store_key("DDDKEY"), None);

    // 8. Asking for a name longer than any symbol is refused before the
    //    probe, skewed table or not; the empty name likewise.
    let long = "B".repeat(SYMBOL_CAPACITY + 1);
    assert_eq!(table.by_fno(&long), None);
    assert_eq!(table.by_fno(""), None);

    // 9. OUT OF THE BOX: two rows, the index keyed on them CROSSED (row 0's
    //    symbol at row 1's ordinal and back). Every probe lands on the other
    //    row, and every lookup answers nothing; uncrossed, both answer.
    let one = row(Some("ONE"), "ONEKEY", APPLIES);
    let two = row(Some("TWO"), "TWOKEY", APPLIES);
    let crossed = skewed([one, two], &["TWO", "ONE"], &["TWOKEY", "ONEKEY"]);
    for asked in ["ONE", "TWO"] {
        assert_eq!(crossed.by_fno(asked), None, "{asked}");
    }
    for asked in ["ONEKEY", "TWOKEY"] {
        assert_eq!(crossed.by_store_key(asked), None, "{asked}");
    }
    let straight = skewed([one, two], &["ONE", "TWO"], &["ONEKEY", "TWOKEY"]);
    assert_eq!(straight.by_fno("TWO"), Some(&two));
    assert_eq!(straight.by_store_key("ONEKEY"), Some(&one));
}

/// **A LINEAGE WHOSE INDEX POINTS AT ANOTHER CHANGE ANSWERS NOTHING**, in both
/// directions, while the agreeing direction of the same table still answers.
#[test]
fn a_lineage_whose_index_points_at_another_change_answers_nothing() {
    let renamed = change("OLDNAME", "NEWNAME", APPLIES);

    // CONTROL: built by `build`, both directions answer.
    let built: Lineage<1, 8> = Lineage::build([renamed]);
    assert_eq!(built.from_old("OLDNAME"), Some(&renamed));
    assert_eq!(built.to_new("NEWNAME"), Some(&renamed));

    // The `old` index names a symbol the change does not start from.
    let wrong_old: Lineage<1, 8> = Lineage {
        rows: [renamed],
        by_old: MemberIndex::build(&["ELSEWHERE"]),
        by_new: MemberIndex::build(&["NEWNAME"]),
    };
    assert_eq!(wrong_old.from_old("ELSEWHERE"), None);
    assert_eq!(wrong_old.to_new("NEWNAME"), Some(&renamed));
    // Nothing forward of a symbol whose only hop is refused.
    assert_eq!(wrong_old.forward("ELSEWHERE").count(), 0);

    // The `new` index names a symbol the change does not produce.
    let wrong_new: Lineage<1, 8> = Lineage {
        rows: [renamed],
        by_old: MemberIndex::build(&["OLDNAME"]),
        by_new: MemberIndex::build(&["ELSEWHERE"]),
    };
    assert_eq!(wrong_new.to_new("ELSEWHERE"), None);
    assert_eq!(wrong_new.from_old("OLDNAME"), Some(&renamed));

    // COMBINED: an agreeing index over a change that does not apply.
    let withheld: Lineage<1, 8> = Lineage::build([change("OLDNAME", "NEWNAME", WITHHELD)]);
    assert_eq!(withheld.from_old("OLDNAME"), None);
    assert_eq!(withheld.to_new("NEWNAME"), None);
}
