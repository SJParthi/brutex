//! Rows of `merge.rs` no test reached before BASE-COV-API repair 3 (shard
//! AS1, D-8661): one vendor asserting one key more than once, with the same
//! id and ISIN, or the same id and another ISIN. The ids are invented; the
//! ISINs are the ones this module's own tests already use.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;
use brutex_core::instrument::{Exchange, Kind, Segment};
use brutex_core::symbol::Symbol;

fn equity(symbol: &str, id: &str, isin: Option<&str>) -> Listing {
    Listing {
        vendor_id: VendorId::new(id).expect("a legal id"),
        key: InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Cash,
            underlying: Symbol::new(symbol).expect("a symbol"),
            kind: Kind::Equity,
        },
        isin: isin.map(|isin| Isin::new(isin).expect("an ISIN")),
        unsuffixed: None,
    }
}

/// **A VENDOR THAT ASSERTS ONE KEY AGAIN IS AMBIGUOUS ONLY WHEN IT SAYS
/// SOMETHING ELSE** (BASE-COV-API repair 3, D-8661): `merge_assertion`.
/// Extremes: one assertion (its id kept); the same id and ISIN again (kept,
/// no conflict); the same id with another ISIN (ambiguous, its id withheld);
/// another id with the same ISIN (ambiguous); an assertion marked ambiguous
/// at its source (withheld at once); a second vendor beside an ambiguous
/// first (its own id kept); and, at once, the same id with no ISIN after one
/// with an ISIN (ambiguous), and a third repeat after the key turned
/// ambiguous (it stays withheld); out of the box, the same assertion a
/// hundred times (one key, never ambiguous).
#[test]
fn a_vendor_that_asserts_one_key_again_is_ambiguous_only_when_it_says_something_else() {
    let reliance = |id, isin| equity("RELIANCE", id, isin);
    let key = reliance("1333", None).key;
    let id_of = |out: &Merged, vendor: Vendor| out.by_key[&key].ids[vendor as usize];
    let ambiguous = |out: &Merged, vendor: Vendor| out.by_key[&key].ambiguous.contains(vendor);

    let mut out = Merged::default();
    merge_assertion(
        &mut out,
        Vendor::Groww,
        key,
        reliance("1333", Some("INE002A01018")),
        false,
    );
    assert_eq!(
        id_of(&out, Vendor::Groww),
        Some(VendorId::new("1333").expect("an id"))
    );
    for _ in 0..100 {
        merge_assertion(
            &mut out,
            Vendor::Groww,
            key,
            reliance("1333", Some("INE002A01018")),
            false,
        );
    }
    assert!(!ambiguous(&out, Vendor::Groww) && out.conflicts.is_empty());

    let mut isin = Merged::default();
    merge_assertion(
        &mut isin,
        Vendor::Groww,
        key,
        reliance("1333", Some("INE002A01018")),
        false,
    );
    merge_assertion(
        &mut isin,
        Vendor::Groww,
        key,
        reliance("1333", Some("INE657B01025")),
        false,
    );
    assert!(ambiguous(&isin, Vendor::Groww));
    assert_eq!(
        id_of(&isin, Vendor::Groww),
        None,
        "an ambiguous id is withheld"
    );
    assert!(
        isin.conflicts
            .iter()
            .any(|why| why.contains("has ambiguous id/ISIN assertions")),
        "{:?}",
        isin.conflicts
    );
    merge_assertion(
        &mut isin,
        Vendor::Groww,
        key,
        reliance("1333", Some("INE002A01018")),
        false,
    );
    assert_eq!(id_of(&isin, Vendor::Groww), None, "it stays withheld");

    let mut id = Merged::default();
    merge_assertion(
        &mut id,
        Vendor::Dhan,
        key,
        reliance("1333", Some("INE002A01018")),
        false,
    );
    merge_assertion(
        &mut id,
        Vendor::Dhan,
        key,
        reliance("2885", Some("INE002A01018")),
        false,
    );
    assert!(ambiguous(&id, Vendor::Dhan));

    let mut none = Merged::default();
    merge_assertion(
        &mut none,
        Vendor::Dhan,
        key,
        reliance("1333", Some("INE002A01018")),
        false,
    );
    merge_assertion(&mut none, Vendor::Dhan, key, reliance("1333", None), false);
    assert!(
        ambiguous(&none, Vendor::Dhan),
        "an ISIN dropped is another assertion"
    );

    let mut raw = Merged::default();
    merge_assertion(
        &mut raw,
        Vendor::Dhan,
        key,
        reliance("1333", Some("INE002A01018")),
        true,
    );
    assert!(ambiguous(&raw, Vendor::Dhan) && id_of(&raw, Vendor::Dhan).is_none());
    merge_assertion(
        &mut raw,
        Vendor::Groww,
        key,
        reliance("7", Some("INE002A01018")),
        false,
    );
    assert_eq!(
        id_of(&raw, Vendor::Groww),
        Some(VendorId::new("7").expect("an id")),
        "another vendor's id is its own"
    );
}
