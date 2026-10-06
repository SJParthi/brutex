#![cfg(test)]
//! The footer pre-check: each refusal at its edge, and the two process aborts
//! (CE-12, CE-13) now returned as named refusals.
#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::reader::LakeFile;

/// `PAR1` + footer + its little-endian length + `PAR1`.
fn wrap(footer: &[u8]) -> Vec<u8> {
    let mut v = b"PAR1".to_vec();
    v.extend_from_slice(footer);
    let len = u32::try_from(footer.len()).expect("test footers are small");
    v.extend_from_slice(&len.to_le_bytes());
    v.extend_from_slice(b"PAR1");
    v
}

fn varint(mut n: u64, out: &mut Vec<u8>) {
    loop {
        let low = u8::try_from(n & 0x7f).expect("seven bits");
        n >>= 7;
        if n == 0 {
            out.push(low);
            return;
        }
        out.push(low | 0x80);
    }
}

fn ok(r: Result<(), LakeError>) {
    if let Err(e) = r {
        panic!("expected the walk to admit, got {e:?}");
    }
}

fn reason(r: Result<(), LakeError>) -> String {
    match r {
        Err(LakeError::FooterUnreadable { reason }) => reason,
        other => panic!("expected FooterUnreadable, got {other:?}"),
    }
}

/// A footer whose field 2 is a schema list of `n` elements, each a struct
/// holding a one-byte name and, when `child`, `num_children = 1`.
fn schema_footer(n: u64, child: bool) -> Vec<u8> {
    let mut f = vec![0x15, 0x02, 0x19, 0xfc];
    varint(n, &mut f);
    for _ in 0..n {
        f.extend_from_slice(&[0x48, 0x01, b'a']);
        if child {
            f.extend_from_slice(&[0x15, 0x02]);
        }
        f.push(0x00);
    }
    f.extend_from_slice(&[0x16, 0x00, 0x19, 0x0c, 0x00]);
    f
}

#[test]
fn the_ce12_list_length_that_aborted_the_allocator_is_refused_by_name() {
    // The audit's repro: a schema list declaring 2^31-1 elements in 21 bytes.
    let file = wrap(&[0x15, 0x02, 0x19, 0xfc, 0xff, 0xff, 0xff, 0xff, 0x07]);
    assert_eq!(file.len(), 21);
    match LakeFile::from_bytes(file) {
        Err(LakeError::FooterUnreadable { reason }) => {
            assert!(reason.contains("refused before parsing"), "{reason}");
        }
        other => panic!("expected FooterUnreadable, got {:?}", other.err()),
    }
}

#[test]
fn the_ce13_million_deep_schema_is_refused_before_parquet_recurses() {
    let file = wrap(&schema_footer(1_000_000, true));
    match LakeFile::from_bytes(file) {
        Err(LakeError::FooterUnreadable { reason }) => {
            assert!(reason.contains("1000000 elements"), "{reason}");
        }
        other => panic!("expected FooterUnreadable, got {:?}", other.err()),
    }
}

#[test]
fn the_schema_bound_is_the_widest_lake_shape_exactly() {
    assert_eq!(MAX_SCHEMA_ELEMENTS, 18);
    ok(walk(&schema_footer(18, false)));
    let why = reason(walk(&schema_footer(19, false)));
    assert!(why.contains("19 elements"), "{why}");
}

#[test]
fn a_long_form_field_id_still_names_the_schema() {
    // Header 0x09: delta 0, type list; the id follows as zigzag(2) = 4.
    let mut f = vec![0x09, 0x04, 0xfc];
    varint(19, &mut f);
    f.extend(std::iter::repeat_n(0x00, 19));
    f.push(0x00);
    assert!(reason(walk(&f)).contains("19 elements"));
    // zigzag(3) = -2: not the schema, so 19 empty structs are stepped over.
    let mut g = vec![0x09, 0x03, 0xfc];
    varint(19, &mut g);
    g.extend(std::iter::repeat_n(0x00, 19));
    g.push(0x00);
    ok(walk(&g));
}

#[test]
fn a_list_in_a_nested_struct_is_not_mistaken_for_the_schema() {
    // Field 1 struct, inside it field 2 a list of 19 empty structs.
    let mut f = vec![0x1c, 0x29, 0xfc];
    varint(19, &mut f);
    f.extend(std::iter::repeat_n(0x00, 19));
    f.extend_from_slice(&[0x00, 0x00]);
    ok(walk(&f));
}

/// `k` structs nested inside the top-level one, through field 10 then 1s.
fn nested(k: usize) -> Vec<u8> {
    let mut f = vec![0xac];
    f.extend(std::iter::repeat_n(0x1c, k - 1));
    f.extend(std::iter::repeat_n(0x00, k + 1));
    f
}

#[test]
fn nesting_is_refused_one_past_the_bound_and_admitted_at_it() {
    ok(walk(&nested(MAX_DEPTH - 1)));
    assert!(reason(walk(&nested(MAX_DEPTH))).contains("deeper than 32"));
}

#[test]
fn a_string_longer_than_the_footer_is_refused_and_one_that_fits_is_not() {
    let mut short = vec![0x68];
    varint(4, &mut short);
    short.extend_from_slice(b"abc");
    assert!(reason(walk(&short)).contains("declares 4 bytes"));
    let mut fits = vec![0x68];
    varint(3, &mut fits);
    fits.extend_from_slice(b"abc");
    fits.push(0x00);
    ok(walk(&fits));
}

#[test]
fn a_list_declaring_more_values_than_bytes_is_refused_at_the_edge() {
    // Field 4, list of i32, declaring 3 values. Two bytes follow, then stop.
    let over = [0x49, 0x35, 0x02, 0x04, 0x00];
    // 3 values > 3 bytes left? Three bytes remain, so it is admitted, then
    // the walk runs out: still a refusal, but by the cursor.
    assert!(reason(walk(&over)).contains("ends inside"));
    let lie = [0x49, 0x45, 0x02, 0x04, 0x00];
    assert!(reason(walk(&lie)).contains("declares 4 values"));
    let honest = [0x49, 0x25, 0x02, 0x04, 0x00];
    ok(walk(&honest));
}

#[test]
fn a_map_is_walked_and_a_map_that_lies_is_refused() {
    // Field 5 map: one pair, key binary, value i32.
    let pair = [0x5b, 0x01, 0x85, 0x01, b'k', 0x02, 0x00];
    ok(walk(&pair));
    let empty = [0x5b, 0x00, 0x00];
    ok(walk(&empty));
    let lie = [0x5b, 0x03, 0x85, 0x01, b'k', 0x02, 0x00];
    assert!(reason(walk(&lie)).contains("declares 6 values"));
    let bad_kind = [0x5b, 0x01, 0xe5, 0x00, 0x00];
    assert!(reason(walk(&bad_kind)).contains("element type 14"));
}

#[test]
fn every_scalar_type_is_stepped_over_by_its_width() {
    let mut f = vec![
        0x11, // field 1 bool true: no data
        0x12, // field 2 bool false: no data
        0x13, 0x7f, // field 3 byte
        0x14, 0x02, // field 4 i16
        0x16, 0x81, 0x01, // field 5 i64, two-byte varint
        0x17, // field 6 double
    ];
    f.extend_from_slice(&[0; 8]);
    f.push(0x1d); // field 7 uuid
    f.extend_from_slice(&[0; 16]);
    f.extend_from_slice(&[0x19, 0x21, 0x01, 0x02]); // field 8 bool list
    f.extend_from_slice(&[0x1a, 0x00]); // field 9 set, empty by a zero byte
    f.push(0x00);
    ok(walk(&f));
    // One byte short of the uuid.
    let cut = f.get(..f.len() - 9).expect("shorter").to_vec();
    assert!(reason(walk(&cut)).contains("declares 16 bytes"));
}

#[test]
fn a_varint_of_ten_bytes_is_read_and_eleven_is_refused() {
    let mut ten = vec![0x16];
    ten.extend(std::iter::repeat_n(0x80, 9));
    ten.extend_from_slice(&[0x01, 0x00]);
    ok(walk(&ten));
    let mut eleven = vec![0x16];
    eleven.extend(std::iter::repeat_n(0x80, 10));
    eleven.extend_from_slice(&[0x01, 0x00]);
    assert!(reason(walk(&eleven)).contains("ten bytes"));
}

#[test]
fn an_unknown_type_and_a_missing_stop_are_refused() {
    assert!(reason(walk(&[0x1e, 0x00])).contains("thrift type 14"));
    assert!(reason(walk(&[0x15, 0x02])).contains("ends inside"));
    assert!(reason(walk(&[0x19, 0x30])).contains("element type 0"));
}

#[test]
fn the_footer_length_must_fit_between_the_magics() {
    let mut file = wrap(&[0x00]);
    ok(check(&file));
    // Declare two bytes where one sits between the magics.
    let at = file.len() - 8;
    file.splice(at..at + 4, 2_u32.to_le_bytes());
    assert!(reason(check(&file)).contains("exceeds the 1 bytes"));
    assert!(reason(check(b"PAR")).contains("too short"));
    assert!(reason(check(b"")).contains("too short"));
}
