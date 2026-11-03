//! BASE-COV-PULL-PS1 (D-7335): an identity word holding a control character
//! that is not whitespace is refused by its field (Route B, recipe v4). Every
//! symbol, identifier and ISIN is invented.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use super::*;

/// The five columns the reader needs, as the module's own tests write them.
const HEADER: &str = "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn";

/// **A CONTROL CHARACTER THAT IS NOT WHITESPACE MAKES AN IDENTITY
/// MALFORMED.** A symbol holding U+0001 and an ISIN holding U+007F (control
/// characters, not whitespace, and neither a line break nor a NUL, which the
/// record reader refuses first) refuse the master at their line, naming the
/// field; the same row without them reads, and a lookup of a symbol holding
/// U+0001 is refused the same way. (No test of the module held a control
/// character that is not whitespace, so the control check never refused a
/// word.)
#[test]
fn an_identity_holding_a_control_character_is_refused_by_its_field() {
    let read = |row: &str| DailyEligibility::parse(&format!("{HEADER}\n{row}\n"));
    for (row, field) in [
        ("7,ZQ\u{1}AUC,EQ,ZQ0000000001,1", "TckrSymb"),
        ("7,ZQAUC,EQ,ZQ00000\u{7f}00001,1", "ISIN"),
    ] {
        assert_eq!(
            read(row).err(),
            Some(format!(
                "line 2: {}",
                unknown(&format!("missing, oversized or malformed {field}"))
            ))
        );
    }
    let master = read("7,ZQAUC,EQ,ZQ0000000001,1").unwrap();
    assert_eq!(master.eligibility("ZQAUC", "ZQ0000000001"), Ok(true));
    assert_eq!(
        master.eligibility("ZQ\u{1}AUC", "ZQ0000000001"),
        Err(unknown("missing, oversized or malformed lookup symbol"))
    );
}
