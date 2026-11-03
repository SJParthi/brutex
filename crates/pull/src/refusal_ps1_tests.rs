//! BASE-COV-PULL-PS1 (D-7335): a vendor error contract's equality and hash,
//! which no test had compared on its field or its envelope, or hashed, and a
//! code read inside a named envelope by `disposition_of`, which no contract
//! but the root kind had been read through. Every contract, key and code is
//! invented.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};

use super::*;

/// An invented reader that knows one code.
fn known(code: &str) -> Option<Disposition> {
    (code == "ZQ1").then_some(Disposition::RequestWrong)
}

/// An invented reader that knows no code.
fn knows_none(_code: &str) -> Option<Disposition> {
    None
}

/// The invented contract: its code one level down, under its own envelope.
const CONTRACT: ErrorNames = ErrorNames {
    field: "ZQ_CODE",
    envelope: Some("ZQ_ENVELOPE"),
    read: known,
    message: None,
    source: "INVENTED contract A",
};

/// `contract` hashed by the standard library's fixed-key hasher: one answer
/// per build.
fn digest(contract: &ErrorNames) -> u64 {
    let mut state = DefaultHasher::new();
    contract.hash(&mut state);
    state.finish()
}

/// **A CONTRACT IS ITS FIELD, ITS ENVELOPE AND ITS SOURCE, AND HASHES AS IT
/// COMPARES.** Another field, another envelope or another source is another
/// contract; another reader is the same one (a `fn` address is not stable
/// across builds). Two equal contracts hash alike, so a set holds one of
/// them.
#[test]
fn a_contract_is_its_field_its_envelope_and_its_source() {
    let same = ErrorNames {
        read: knows_none,
        ..CONTRACT
    };
    assert_eq!(same, CONTRACT, "another reader, the same contract");
    assert_eq!(
        digest(&same),
        digest(&CONTRACT),
        "equal contracts hash alike"
    );
    let other_field = ErrorNames {
        field: "ZQ_OTHER",
        ..CONTRACT
    };
    assert_ne!(other_field, CONTRACT, "another field");
    assert_ne!(
        ErrorNames {
            envelope: None,
            ..CONTRACT
        },
        CONTRACT,
        "another envelope"
    );
    assert_ne!(
        ErrorNames {
            source: "INVENTED contract B",
            ..CONTRACT
        },
        CONTRACT,
        "another source"
    );
    let held: HashSet<ErrorNames> = [CONTRACT, same, other_field].into_iter().collect();
    assert_eq!(held.len(), 2);
}

/// **A CODE NESTED IN ITS NAMED ENVELOPE IS READ THERE, AND ONLY THERE.**
/// The contract's code is read inside its envelope object and weighed by the
/// contract's own reader; the same code at the root, or an envelope that is
/// no object holding it, is no answer (named, never searched for: the caller
/// falls back to the status). An equal contract with another reader answers
/// by that reader.
#[test]
fn a_code_in_its_named_envelope_is_read_there_and_only_there() {
    let nested = r#"{"ZQ_ENVELOPE":{"ZQ_CODE":"ZQ1"}}"#;
    assert_eq!(
        disposition_of(nested, &CONTRACT),
        Some(Disposition::RequestWrong)
    );
    assert_eq!(
        disposition_of(r#"{"ZQ_CODE":"ZQ1"}"#, &CONTRACT),
        None,
        "at the root"
    );
    assert_eq!(
        disposition_of(r#"{"ZQ_ENVELOPE":"ZQ1"}"#, &CONTRACT),
        None,
        "an envelope that holds no code"
    );
    let other_reader = ErrorNames {
        read: knows_none,
        ..CONTRACT
    };
    assert_eq!(
        disposition_of(nested, &other_reader),
        None,
        "its own reader decides"
    );
}
