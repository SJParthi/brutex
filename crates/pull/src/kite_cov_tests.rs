#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): a Kite exception is said by the name the
//! vendor writes for it, the name a refusal and a log line carry, and that
//! name reads back as the same exception.

use super::*;

/// Every exception's `Display` is its wire name, each name is its own, and
/// the name reads back as the exception it names.
#[test]
fn every_kite_exception_is_said_by_its_wire_name() {
    let said: Vec<String> = KiteException::ALL.iter().map(ToString::to_string).collect();
    assert_eq!(
        said,
        [
            "TokenException",
            "UserException",
            "OrderException",
            "InputException",
            "MarginException",
            "HoldingException",
            "NetworkException",
            "DataException",
            "GeneralException",
            "PermissionException",
        ]
    );
    for exception in KiteException::ALL {
        assert_eq!(KiteException::from_wire(exception.wire()), Some(exception));
    }
}
