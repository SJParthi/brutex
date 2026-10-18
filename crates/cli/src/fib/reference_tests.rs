//! Tests of the FIB reference stamps (FIB-8; D-6443).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

/// NO TYPED NUMBERS: the shipping part of this file holds no numeric
/// literal but 0 and 1.
#[test]
fn reference_rs_types_no_number() {
    let source = include_str!("reference.rs");
    let typed = vocab::fib_cube::untyped_numbers(source);
    assert!(typed.is_empty(), "{typed:?}");
}
