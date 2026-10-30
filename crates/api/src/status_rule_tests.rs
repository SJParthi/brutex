//! The one status rule's own tests (D-7550): the classifier, its rules, its
//! probe, and the table over every route of `route_table` that takes a symbol
//! or an identity. No fixture here holds market data: every store is empty or
//! holds files of invented bytes.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail, and these lints exist to keep panics out of \
              the crate rather than out of its tests"
)]

use super::*;

/// A scratch directory of this test's own, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("status-rule-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// An identity of one repeated byte, as its 64 hex digits.
fn hex_of(byte: u8) -> String {
    crate::server::hex32([byte; 32])
}

// ══ The table ═══════════════════════════════════════════════════════════════

/// **EVERY CLASS HAS EXACTLY ONE STATUS, AND NO TWO CLASSES BUT THE TWO 503s
/// SHARE ONE** — the table in the module header, as code: `400`, `404`, `410`,
/// `422`, `429`, `503`, and a distinct word for each.
#[test]
fn every_class_has_its_one_status_and_its_own_word() {
    let statuses: Vec<u16> = Class::ALL.iter().map(|c| c.status().as_u16()).collect();
    assert_eq!(statuses, [400, 404, 410, 416, 422, 429, 503]);
    let words: Vec<&str> = Class::ALL.iter().map(|c| c.word()).collect();
    assert_eq!(
        words,
        [
            "malformed",
            "not_found",
            "retired",
            "out_of_range",
            "over_bound",
            "saturated",
            "unreadable"
        ]
    );
    // A client's question is never a 5xx and a server fault never a 4xx.
    for class in Class::ALL {
        let server = matches!(class, Class::Unreadable);
        assert_eq!(class.status().is_server_error(), server, "{class:?}");
    }
}

// ══ Symbols: the store's own rule, before the store ════════════════════════

/// The segment rule's refusal of `value`, or `None` where it is admitted.
fn segment_rule(value: &str) -> Option<Rule> {
    segment("symbol", value).err().map(|why| {
        assert_eq!(why.parameter, "symbol");
        assert_eq!(why.value, value);
        why.rule
    })
}

/// **A WRONG-CASE SYMBOL IS MALFORMED, NEVER FOLDED** — `nifty`, `NiFtY` and a
/// trailing blank (`NIFTY+` decodes to `NIFTY `) are each refused by the store's
/// own rule, naming the byte; `NIFTY`, `M&M` and a one-letter `M` are admitted.
#[test]
fn a_wrong_case_or_blank_symbol_is_malformed_and_never_folded() {
    assert_eq!(segment_rule("NIFTY"), None);
    assert_eq!(segment_rule("M&M"), None, "an encoded M&M is one symbol");
    assert_eq!(segment_rule("M"), None, "an unencoded M&M reads as M");
    assert_eq!(
        segment_rule("nifty"),
        Some(Rule::Segment(PathError::NotCanonicalCase {
            field: "symbol",
            byte: b'n'
        }))
    );
    assert_eq!(
        segment_rule("NiFtY"),
        Some(Rule::Segment(PathError::NotCanonicalCase {
            field: "symbol",
            byte: b'i'
        })),
        "the first lower-case letter is named"
    );
    assert_eq!(
        segment_rule("NIFTY "),
        Some(Rule::Segment(PathError::IllegalByte {
            field: "symbol",
            byte: b' '
        }))
    );
    let said = segment("symbol", "nifty").unwrap_err().to_string();
    assert!(
        said.starts_with("symbol=\"nifty\" is malformed: path segment symbol holds byte 0x6e"),
        "{said}"
    );
    assert!(said.contains("refused rather than folded"), "{said}");
}

/// **THE EDGES OF THE SEGMENT RULE**: empty, a traversal, 24 bytes (the cap,
/// admitted) and 25 (refused), each by its own named rule.
#[test]
fn the_segment_rule_names_empty_traversal_and_length() {
    assert_eq!(
        segment_rule(""),
        Some(Rule::Segment(PathError::EmptySegment { field: "symbol" }))
    );
    assert_eq!(
        segment_rule(".."),
        Some(Rule::Segment(PathError::Traversal { field: "symbol" }))
    );
    assert_eq!(segment_rule(&"A".repeat(24)), None);
    assert_eq!(
        segment_rule(&"A".repeat(25)),
        Some(Rule::Segment(PathError::SegmentTooLong {
            field: "symbol",
            len: 25
        }))
    );
}

/// **COMBINATION: TOO LONG AND WRONG CASE AT ONCE** — one refusal, the first in
/// the rule's declared order (length before case), never two answers.
#[test]
fn a_symbol_both_too_long_and_lower_case_is_refused_for_its_length_first() {
    assert_eq!(
        segment_rule(&"n".repeat(25)),
        Some(Rule::Segment(PathError::SegmentTooLong {
            field: "symbol",
            len: 25
        }))
    );
}

/// **OUT OF THE BOX: A SYMBOL WHOSE UNICODE CASE FOLD IS ASCII.** A dotless
/// `ı` upper-cases to `I` and a Kelvin sign lower-cases to `k`, so a reader that
/// folded with `str::to_uppercase` would turn `NıFTY` into `NIFTY` and
/// `BAN\u{212A}NIFTY` into a key that is not the one sent. Both are malformed by
/// their first non-ASCII byte, never folded into a real key.
#[test]
fn a_symbol_whose_unicode_fold_is_ascii_is_malformed_never_folded() {
    // The folds are real: this is what a Unicode-aware reader would do.
    assert_eq!("N\u{131}FTY".to_uppercase(), "NIFTY");
    assert_eq!("BAN\u{212a}NIFTY".to_uppercase().to_lowercase(), "banknifty");
    assert_eq!(
        segment_rule("N\u{131}FTY"),
        Some(Rule::Segment(PathError::IllegalByte {
            field: "symbol",
            byte: 0xc4
        }))
    );
    assert_eq!(
        segment_rule("BAN\u{212a}NIFTY"),
        Some(Rule::Segment(PathError::IllegalByte {
            field: "symbol",
            byte: 0xe2
        }))
    );
}

// ══ Identities ══════════════════════════════════════════════════════════════

/// The identity rule's refusal of `text`, or the bytes it admits.
fn identity_rule(text: &str) -> Result<[u8; 32], Rule> {
    identity_of(text, "identity").map_err(|why| {
        assert_eq!(why.parameter, "identity");
        assert_eq!(why.value, text);
        why.rule
    })
}

/// **AN IDENTITY IS 64 LOWERCASE HEX DIGITS, EXACTLY** — all zeros and every
/// digit decode to their bytes; 63 and 65 characters are refused by length;
/// an upper-case digit by its case and position; a non-digit by its byte and
/// position; an absent field is `None`, which the route refuses itself.
#[test]
fn an_identity_is_sixty_four_lowercase_hex_digits_exactly() {
    assert_eq!(identity_rule(&"0".repeat(64)), Ok([0; 32]));
    let every = "0123456789abcdef".repeat(4);
    let mut expected = [0_u8; 32];
    for (n, slot) in expected.iter_mut().enumerate() {
        *slot = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef][n % 8];
    }
    assert_eq!(identity_rule(&every), Ok(expected));
    assert_eq!(
        identity_rule(&"a".repeat(63)),
        Err(Rule::IdentityLength { len: 63 })
    );
    assert_eq!(
        identity_rule(&"a".repeat(65)),
        Err(Rule::IdentityLength { len: 65 })
    );
    assert_eq!(
        identity_rule(&"A".repeat(64)),
        Err(Rule::IdentityCase { at: 0, byte: b'A' })
    );
    let last_upper = format!("{}F", "f".repeat(63));
    assert_eq!(
        identity_rule(&last_upper),
        Err(Rule::IdentityCase { at: 63, byte: b'F' })
    );
    let tenth = format!("{}g{}", "0".repeat(10), "0".repeat(53));
    assert_eq!(
        identity_rule(&tenth),
        Err(Rule::IdentityDigit { at: 10, byte: b'g' })
    );
    let second = format!("0\u{0}{}", "0".repeat(62));
    assert_eq!(
        identity_rule(&second),
        Err(Rule::IdentityDigit { at: 1, byte: 0 })
    );
    assert_eq!(identity("kind=coordinates", "identity"), Ok(None));
    assert_eq!(
        identity(&format!("identity={}", hex_of(0xab)), "identity"),
        Ok(Some([0xab; 32]))
    );
}

/// **COMBINATION: UPPER CASE AND THE WRONG LENGTH AT ONCE**, and **A MULTI-BYTE
/// CHARACTER THAT MAKES THE LENGTH RIGHT** — length is named first, and a
/// 63-character value whose last character takes two bytes is 64 bytes long and
/// refused by that byte, never decoded.
#[test]
fn an_identity_is_refused_for_its_first_breach_in_the_declared_order() {
    assert_eq!(
        identity_rule(&"A".repeat(63)),
        Err(Rule::IdentityLength { len: 63 })
    );
    let wide = format!("{}\u{e9}", "a".repeat(62));
    assert_eq!(wide.len(), 64);
    assert_eq!(
        identity_rule(&wide),
        Err(Rule::IdentityDigit { at: 62, byte: 0xc3 })
    );
    // A trailing blank (`+` in a query) is one character too many, never
    // trimmed into a match.
    let blank = format!("identity={}+", hex_of(0));
    assert_eq!(
        identity(&blank, "identity").map_err(|why| why.rule),
        Err(Rule::IdentityLength { len: 65 })
    );
}

/// **A REFUSED VALUE IS QUOTED TO A BOUND** — a value past [`QUOTED_CHARS`]
/// characters is quoted to there and its byte length named, so a 7,009-byte
/// value cannot make a 7 KiB sentence.
#[test]
fn a_refused_value_is_quoted_to_a_bound_and_its_length_named() {
    let long = "A".repeat(QUOTED_CHARS + 1);
    let said = identity_of(&long, "identity").unwrap_err().to_string();
    assert!(
        said.starts_with(&format!(
            "identity={:?}... ({} bytes) is malformed",
            "A".repeat(QUOTED_CHARS),
            QUOTED_CHARS + 1
        )),
        "{said}"
    );
    let exact = "B".repeat(QUOTED_CHARS);
    let said = identity_of(&exact, "identity").unwrap_err().to_string();
    assert!(
        said.starts_with(&format!("identity={exact:?} is malformed")),
        "{said}"
    );
    assert!(said.contains("is 80 characters"), "{said}");
}

// ══ The probe ═══════════════════════════════════════════════════════════════

/// **ONE PROBE DECIDES HELD, NOT FOUND, OR UNREADABLE** — an entry that is
/// there is held; a missing one under a root that is there is `404` naming the
/// path probed; a missing root, a root that is a file, and a file where a
/// folder should be are each `503` naming the path and the `io::ErrorKind`.
#[test]
fn one_probe_decides_held_not_found_or_unreadable() {
    let root = scratch("probe");
    let relative = Path::new("research").join("kind").join(hex_of(1));
    std::fs::create_dir_all(root.join(&relative)).expect("held");
    let held = probe(&root, &relative, "Thing").expect("held");
    assert_eq!(held.path, root.join(&relative));
    assert_eq!(held.shown, relative.display().to_string());

    let absent = Path::new("research").join("kind").join(hex_of(2));
    let fault = probe(&root, &absent, "Thing").unwrap_err();
    assert_eq!(fault.class, Class::NotFound);
    assert_eq!(fault.status(), StatusCode::NOT_FOUND);
    assert_eq!(fault.kind, Some(ErrorKind::NotFound));
    assert_eq!(fault.path.as_deref(), Some(absent.display().to_string().as_str()));
    assert!(fault.why.starts_with("Thing not held: one probe of "), "{}", fault.why);
    assert!(fault.why.contains("found nothing (NotFound)"), "{}", fault.why);

    // A WHOLE KIND NEVER SAVED is the same not-found: nothing is listed.
    let never = Path::new("never-written").join(hex_of(3));
    assert_eq!(probe(&root, &never, "Thing").unwrap_err().class, Class::NotFound);

    // THE ROOT ITSELF IS MISSING: still ONE probe's NotFound -- nothing is
    // there -- and the sentence names the root, so a dashboard pointed at the
    // wrong store says so on every answer rather than blaming the store.
    let missing = root.join("no-such-root");
    let fault = probe(&missing, &relative, "Thing").unwrap_err();
    assert_eq!(fault.class, Class::NotFound);
    assert_eq!(fault.kind, Some(ErrorKind::NotFound));
    assert_eq!(fault.path.as_deref(), Some(relative.display().to_string().as_str()));
    assert!(fault.why.contains(&missing.display().to_string()), "{}", fault.why);
    assert!(fault.why.contains("BRUTEX_STORE"), "{}", fault.why);

    // A FILE WHERE A FOLDER SHOULD BE, at the root and below it.
    let file_root = root.join("a-file");
    std::fs::write(&file_root, b"not a folder").expect("file");
    let fault = probe(&file_root, &relative, "Thing").unwrap_err();
    assert_eq!(fault.class, Class::Unreadable);
    assert_eq!(fault.kind, Some(ErrorKind::NotADirectory));
    std::fs::write(root.join("research").join("flat"), b"x").expect("file");
    let below = Path::new("research").join("flat").join(hex_of(4));
    let fault = probe(&root, &below, "Thing").unwrap_err();
    assert_eq!(fault.class, Class::Unreadable);
    assert_eq!(fault.kind, Some(ErrorKind::NotADirectory));
    assert!(fault.why.contains("NotADirectory"), "{}", fault.why);
}

/// **A ROOT THAT POINTS NOWHERE IS ONE PROBE'S NOT FOUND, AND SAYS WHICH
/// ROOT** — a store root that is a symbolic link to nothing holds nothing;
/// the answer is the `404` every missing entry gets, never a second probe and
/// never a `503`, and its sentence names the root it looked under.
#[test]
fn a_root_that_points_nowhere_is_not_found_naming_the_root() {
    let dir = scratch("root-link");
    let link_root = dir.join("root-link");
    std::os::unix::fs::symlink(dir.join("nowhere"), &link_root).expect("symlink");
    let fault = probe(&link_root, Path::new("x"), "Thing").unwrap_err();
    assert_eq!(fault.class, Class::NotFound);
    assert_eq!(fault.kind, Some(ErrorKind::NotFound));
    assert_eq!(fault.path.as_deref(), Some("x"));
    assert!(fault.why.contains(&link_root.display().to_string()), "{}", fault.why);
}

/// **A DANGLING SYMBOLIC LINK WHERE THE ENTRY SHOULD BE IS HELD, NOT ABSENT**
/// — the entry is there, so the probe hands it to the read, whose refusal is
/// the `503`; a `404` would tell the operator nothing was saved when something
/// was, and broke.
#[test]
fn a_dangling_link_at_the_entry_is_held_and_never_not_found() {
    let root = scratch("dangling");
    std::fs::create_dir_all(root.join("research")).expect("dir");
    let entry = Path::new("research").join(hex_of(5));
    std::os::unix::fs::symlink(root.join("gone"), root.join(&entry)).expect("symlink");
    let held = probe(&root, &entry, "Thing").expect("the link is held");
    assert!(std::fs::metadata(&held.path).is_err(), "it points at nothing");
}

/// **A ROOT WHOSE MODE IS 000 IS UNREADABLE, NEVER NOT FOUND** — run where the
/// permission bits bind (as uid 65534 under root, D-0995), so the refusal is
/// the kernel's `PermissionDenied` and not a host's leniency.
#[test]
fn a_root_whose_mode_is_000_is_unreadable_never_not_found() {
    crate::isolated::where_permission_binds(
        "status_rule::tests::a_root_whose_mode_is_000_is_unreadable_never_not_found",
        || {
            use std::os::unix::fs::PermissionsExt as _;
            let root = scratch("mode-000");
            std::fs::create_dir_all(root.join("research")).expect("dir");
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o000))
                .expect("chmod 000");
            let fault = probe(&root, &Path::new("research").join(hex_of(6)), "Thing");
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))
                .expect("chmod back");
            let fault = fault.unwrap_err();
            assert_eq!(fault.class, Class::Unreadable, "{fault:?}");
            assert_eq!(fault.kind, Some(ErrorKind::PermissionDenied), "{fault:?}");
            assert!(fault.why.contains("PermissionDenied"), "{}", fault.why);
        },
    );
}

/// **A SERVER WITH NO STORE ROOT CONFIGURED IS UNAVAILABLE** — `503` with the
/// configuration's own sentence, never a client's `4xx`.
#[test]
fn no_configured_root_is_unreadable() {
    let fault = configured(Err("BRUTEX_STORE is empty".to_owned())).unwrap_err();
    assert_eq!(fault.class, Class::Unreadable);
    assert_eq!(fault.why, "no store root is configured: BRUTEX_STORE is empty");
    let root = scratch("configured");
    assert_eq!(configured(Ok(root.clone())), Ok(root));
}

/// **ONE BODY SHAPE FOR EVERY FAULT ON EVERY IDENTITY ROUTE** — the same eight
/// keys, the class's word as `status`, the identity as its hex, no rows.
#[test]
fn every_identity_fault_has_one_body_shape() {
    let root = scratch("body");
    let fault = probe(&root, &Path::new("r").join(hex_of(7)), "Thing").unwrap_err();
    let body = fault.identity_body(Some([7; 32]));
    let keys: Vec<&str> = body
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        [
            "error_kind",
            "identity",
            "path",
            "refusal",
            "rows",
            "rule",
            "schema_version",
            "status"
        ]
    );
    assert_eq!(body["status"], "not_found");
    assert_eq!(body["identity"], hex_of(7));
    assert_eq!(body["rows"], json!([]));
    assert_eq!(body["rule"], DECISION);
    assert_eq!(body["error_kind"], "NotFound");
    let malformed = Fault::malformed(&identity_of("x", "identity").unwrap_err());
    let body = malformed.identity_body(None);
    assert_eq!(body["status"], "malformed");
    assert_eq!(body["identity"], Value::Null);
    assert_eq!(body["error_kind"], Value::Null);
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(Fault::refused("no").status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        Fault::saturated("full").status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        Fault::unreadable(Some("p".to_owned()), "torn").identity_body(None)["status"],
        "unreadable"
    );
}
