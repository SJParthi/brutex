#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): `Layout::declared` and
//! `Layout::with_partition` build every row of [`Layout::KNOWN`] in the
//! compiler, which Route B does not count; run here at run time, each builds
//! exactly its shipped row, and the partition step changes nothing but the
//! partition.

use super::*;

/// Each row of the table, declared again at run time, is the row the table
/// holds; the partitioned row differs from its unpartitioned twin in that
/// alone.
#[test]
fn every_layout_row_is_its_own_declaration_at_run_time() {
    let v1 = Layout::declared(1, MAGIC, V1_ENTRY_STRIDE, V1_ENTRY_STRIDE_LEN, false);
    let v2 = Layout::declared(2, MAGIC_V2, ENTRY_STRIDE, ENTRY_LEN, true);
    let v3 = Layout::declared(3, MAGIC_V2, ENTRY_STRIDE, ENTRY_LEN, true);
    let unpartitioned = Layout::declared(4, MAGIC_V2, ENTRY_STRIDE, ENTRY_LEN, true);
    let v4 = unpartitioned.with_partition();
    assert_eq!(
        [v1, v2, v3, v4],
        [Layout::V1, Layout::V2, Layout::V3, Layout::V4]
    );
    assert_eq!(Layout::KNOWN, [v1, v2, v3, v4]);
    assert_ne!(unpartitioned, v4);
    assert_eq!(
        Layout {
            partitioned: true,
            ..unpartitioned
        },
        v4
    );
}
