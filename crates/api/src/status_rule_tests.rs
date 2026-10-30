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

/// **EVERY CLASS HAS EXACTLY ONE STATUS, AND NO TWO CLASSES SHARE ONE** — the
/// table in the module header, as code: `400`, `404`, `410`, `416`, `422`,
/// `429`, `503`, and a distinct word for each.
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
    assert_eq!(
        "BAN\u{212a}NIFTY".to_uppercase().to_lowercase(),
        "banknifty"
    );
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

/// **AN IDENTITY IS 64 LOWERCASE HEX DIGITS, EXACTLY, AND NEVER ALL ZEROS** —
/// every digit decodes to its byte; all zeros is the null value and refused;
/// one non-zero digit anywhere is admitted; 63 and 65 characters are refused
/// by length; an upper-case digit by its case and position; a non-digit by
/// its byte and position; an absent field is `None`, which the route refuses.
#[test]
fn an_identity_is_sixty_four_lowercase_hex_digits_exactly() {
    assert_eq!(identity_rule(&"0".repeat(64)), Err(Rule::IdentityZero));
    let mut last = [0_u8; 32];
    last[31] = 1;
    assert_eq!(identity_rule(&format!("{}1", "0".repeat(63))), Ok(last));
    let mut first = [0_u8; 32];
    first[0] = 0x10;
    assert_eq!(identity_rule(&format!("1{}", "0".repeat(63))), Ok(first));
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
    let said = identity_of(&"0".repeat(64), "pin").unwrap_err().to_string();
    assert!(said.starts_with("pin=\""), "{said}");
    assert!(said.contains("the null value"), "{said}");
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
    assert_eq!(
        fault.path.as_deref(),
        Some(absent.display().to_string().as_str())
    );
    assert!(
        fault.why.starts_with("Thing not held: one probe of "),
        "{}",
        fault.why
    );
    assert!(
        fault.why.contains("found nothing (NotFound)"),
        "{}",
        fault.why
    );

    // A WHOLE KIND NEVER SAVED is the same not-found: nothing is listed.
    let never = Path::new("never-written").join(hex_of(3));
    assert_eq!(
        probe(&root, &never, "Thing").unwrap_err().class,
        Class::NotFound
    );

    // THE ROOT ITSELF IS MISSING: still ONE probe's NotFound -- nothing is
    // there -- and the sentence names the root, so a dashboard pointed at the
    // wrong store says so on every answer rather than blaming the store.
    let missing = root.join("no-such-root");
    let fault = probe(&missing, &relative, "Thing").unwrap_err();
    assert_eq!(fault.class, Class::NotFound);
    assert_eq!(fault.kind, Some(ErrorKind::NotFound));
    assert_eq!(
        fault.path.as_deref(),
        Some(relative.display().to_string().as_str())
    );
    assert!(
        fault.why.contains(&missing.display().to_string()),
        "{}",
        fault.why
    );
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
    assert!(
        fault.why.contains(&link_root.display().to_string()),
        "{}",
        fault.why
    );
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
    assert!(
        std::fs::metadata(&held.path).is_err(),
        "it points at nothing"
    );
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
    assert_eq!(
        fault.why,
        "no store root is configured: BRUTEX_STORE is empty"
    );
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

/// **INSIDE A FOUND RESULT, A REFUSAL IS CLASSED BY ITS STEP, NEVER BY ITS
/// WORDS** (D-7550, D-7551): the reader's typed absence and a pin the store no
/// longer holds are the one not-found, a page past the extent `416`, and a
/// reader's own words — even words that sound like a client's — the store's
/// `503`. Every fault names the entry the probe found.
#[test]
fn a_refusal_inside_a_found_result_is_classed_by_its_step() {
    let root = Path::new("/a-store-root");
    for (refused, class, says, kind) in [
        (
            Refused::Absent,
            Class::NotFound,
            "Thing not held: one probe of ns/aa under the configured store root /a-store-root",
            Some(ErrorKind::NotFound),
        ),
        (
            Refused::NotHeld("the pin is not held".into()),
            Class::NotFound,
            "the pin is not held",
            None,
        ),
        (
            Refused::OutOfRange("offset 9 is past".into()),
            Class::OutOfRange,
            "offset 9 is past",
            None,
        ),
        (
            Refused::Unreadable("torn".into()),
            Class::Unreadable,
            "torn",
            None,
        ),
        // A reader's sentence about a page is still the store's refusal: a
        // client's refusal is typed before the read, never read back from words.
        (
            Refused::from("candidate page is outside its exact recorded extent"),
            Class::Unreadable,
            "outside its exact",
            None,
        ),
        (
            Refused::from(String::from("Boolean page bounds refused")),
            Class::Unreadable,
            "bounds refused",
            None,
        ),
    ] {
        let fault = refused.fault(root, "ns/aa".to_owned(), "Thing");
        assert_eq!(fault.class, class, "{}", fault.why);
        assert!(fault.why.contains(says), "{}", fault.why);
        assert_eq!(fault.path.as_deref(), Some("ns/aa"));
        assert_eq!(fault.kind, kind);
    }
    assert_eq!(Refused::Absent.to_string(), "nothing saved for this key");
    for refused in [
        Refused::NotHeld("a".into()),
        Refused::OutOfRange("a".into()),
        Refused::Unreadable("a".into()),
    ] {
        assert_eq!(refused.to_string(), "a");
    }
}

/// **THE EXTENT STEP** (D-7551): an offset AT the saved extent is the empty
/// last page; one past it, up to the largest a client can send, is the
/// client's `416`, with one sentence naming the offset and the extent.
#[test]
fn the_extent_step_passes_the_empty_last_page_and_refuses_one_past_it() {
    let sentence =
        "offset 4 is past the 3 saved rows; restart within the extent, no row was served";
    assert_eq!(reach(0, 0, "rows"), Ok(()));
    assert_eq!(reach(2, 3, "rows"), Ok(()));
    assert_eq!(reach(3, 3, "rows"), Ok(()));
    assert_eq!(
        reach(4, 3, "rows"),
        Err(Refused::OutOfRange(sentence.to_owned()))
    );
    assert_eq!(reach(usize::MAX, usize::MAX, "rows"), Ok(()));
    assert!(reach(usize::MAX, usize::MAX - 1, "rows").is_err());
    assert!(reach(1, 0, "rows").is_err());
    let rows = [1_u8, 2, 3];
    assert_eq!(tail(&rows, 0, "rows"), Ok(&rows[..]));
    assert_eq!(tail(&rows, 2, "rows"), Ok(&rows[2..]));
    assert_eq!(tail(&rows, 3, "rows"), Ok(&[][..]));
    assert_eq!(
        tail(&rows, 4, "rows"),
        Err(Refused::OutOfRange(sentence.to_owned()))
    );
    assert_eq!(
        tail(&rows, usize::MAX, "rows"),
        Err(beyond(&usize::MAX, &3, "rows"))
    );
    assert_eq!(tail::<u8>(&[], 0, "rows"), Ok(&[][..]));
    assert_eq!(
        beyond(&4, &3, "rows"),
        Refused::OutOfRange(sentence.to_owned())
    );
}

// ══ The table over every route of `route_table` ═════════════════════════════

/// The GET routes `route_table` registers, read from its own source text:
/// each path and its handler as written there (a closure is `closure`).
/// Comments are dropped first, so a path a comment names is no route.
fn registered_gets() -> Vec<(String, String)> {
    let source = include_str!("server.rs");
    let start = source
        .find("\nfn route_table(")
        .expect("route_table is in server.rs");
    let body = &source[start..];
    let end = body.find("\n}\n").expect("route_table ends");
    let code: String = body[..end]
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    let code = code.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = Vec::new();
    for part in code.split(".route(").skip(1) {
        let Some(path) = part.split('"').nth(1) else {
            continue;
        };
        let Some((_, after)) = part.split_once("axum::routing::get(") else {
            continue;
        };
        let after = after.trim_start();
        let handler = if after.starts_with("move") {
            "closure".to_owned()
        } else {
            after
                .chars()
                .take_while(|c| !matches!(c, ')' | '(' | ','))
                .collect::<String>()
                .trim()
                .to_owned()
        };
        out.push((path.to_owned(), handler));
    }
    out
}

/// What a route's query takes, by the request its rows send. A query holds
/// `{sym}` and `{seg}` (a symbol route) or `{id}` (an identity route), filled
/// per case.
#[derive(Clone, Copy, Debug)]
enum Takes {
    /// Neither a symbol nor an identity: a row all the same, so a route added
    /// to `route_table` must say which it is.
    Neither,
    /// `symbol=` (with `segment=` where `{seg}` appears).
    Symbol(&'static str),
    /// `identity=`.
    Identity(&'static str),
}

/// Who answers a route's refusals: this part (D-7550 decides every status) or
/// the file outside it that owns the route, whose answers today are recorded
/// beside the row ([`Row::today`]) so a change to them is seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owner {
    ThisPart,
    Outside(&'static str),
}

/// One route of `route_table`.
#[derive(Clone, Copy, Debug)]
struct Row {
    path: &'static str,
    handler: &'static str,
    takes: Takes,
    owner: Owner,
    /// For a route outside this part: what each case answers today, in the
    /// order of [`SYMBOL_CASES`] or [`IDENTITY_CASES`], the root-is-a-file
    /// case last. `None` for this part's routes, whose statuses D-7550 decides.
    today: Option<&'static [u16]>,
}

const fn neither(path: &'static str, handler: &'static str) -> Row {
    Row {
        path,
        handler,
        takes: Takes::Neither,
        owner: Owner::Outside("takes neither"),
        today: None,
    }
}

const fn mine(path: &'static str, handler: &'static str, takes: Takes) -> Row {
    Row {
        path,
        handler,
        takes,
        owner: Owner::ThisPart,
        today: None,
    }
}

const fn outside(
    path: &'static str,
    handler: &'static str,
    takes: Takes,
    file: &'static str,
    today: &'static [u16],
) -> Row {
    Row {
        path,
        handler,
        takes,
        owner: Owner::Outside(file),
        today: Some(today),
    }
}

/// A symbol route's cases: the value of `{sym}` and `{seg}`, and what D-7550
/// decides for this part's routes. The first is the control: the canonical
/// spelling, which proves the row sends every other field its route needs.
const SYMBOL_CASES: [(&str, &str, &str, u16); 7] = [
    ("control", "NIFTY", "INDEX", 0),
    ("lower-case symbol", "nifty", "INDEX", 400),
    ("mixed-case symbol", "NiFtY", "INDEX", 400),
    ("trailing blank", "NIFTY+", "INDEX", 400),
    ("wrong-case segment", "NIFTY", "index", 400),
    ("dotless i folds to I", "N%C4%B1FTY", "INDEX", 400),
    ("Kelvin sign folds to k", "BAN%E2%84%AANIFTY", "INDEX", 400),
];

/// An identity route's cases: the value of `{id}`, and what D-7550 decides.
const IDENTITY_CASES: [(&str, &str, u16); 6] = [
    (
        "unknown identity",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        404,
    ),
    (
        "63 hex digits",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        400,
    ),
    (
        "65 hex digits",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        400,
    ),
    (
        "upper-case hex",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        400,
    ),
    (
        "all zeros",
        "0000000000000000000000000000000000000000000000000000000000000000",
        400,
    ),
    (
        "a NUL after 64 digits",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa%00",
        400,
    ),
];

/// What a root that is a regular file answers on this part's routes: what
/// exists (the file) cannot be read as a store.
const FILE_ROOT: u16 = 503;

/// EVERY GET ROUTE OF `route_table`, each with a row. The symbol and identity
/// rows send the route's other required fields; `today` records an outside
/// route's answers as they are, a deviation from D-7550 named in
/// `docs/06-limits.md` L-STATUS-02 with its owner.
const ROWS: &[Row] = &[
    neither("/dashboard", "home"),
    neither("/instruments", "page"),
    neither("/instruments.json", "instruments_json"),
    neither("/feeds.json", "feeds_json"),
    neither("/settings.json", "settings_json"),
    neither("/universes.json", "universe_reach_json"),
    outside(
        "/calendar.json",
        "calendar_json",
        Takes::Symbol("feed=zerodha&symbol={sym}&timeframe=1min"),
        "crates/api/src/calendar_route.rs",
        &[200, 200, 200, 200, 200, 200, 200, 503],
    ),
    neither("/vocab.json", "vocab_json"),
    neither("/indexmap.json", "indexmap_json"),
    neither("/folder.json", "crate::folder::folder_json"),
    outside(
        "/bars.json",
        "bars_json",
        Takes::Symbol(
            "feed=zerodha&exchange=NSE&segment={seg}&symbol={sym}&timeframe=1min&month=2026-09",
        ),
        "crates/api/src/bars_route.rs",
        &[404, 400, 400, 400, 400, 400, 400, 500],
    ),
    outside(
        "/gaps.json",
        "gaps_json",
        Takes::Symbol(
            "feed=zerodha&exchange=NSE&segment={seg}&symbol={sym}&timeframe=1min&month=2026-09",
        ),
        "crates/api/src/gaps_route.rs",
        &[200, 400, 400, 400, 400, 400, 400, 200],
    ),
    outside(
        "/bars/window.json",
        "bars_window_json",
        Takes::Symbol(
            "feed=zerodha&exchange=NSE&segment={seg}&symbol={sym}&timeframe=1min&from=2026-09&to=2026-09",
        ),
        "crates/api/src/bars_route.rs",
        &[400, 400, 400, 400, 400, 400, 400, 400],
    ),
    outside(
        "/bars/export.csv",
        "bars_export::bars_export_csv",
        Takes::Symbol(
            "feed=zerodha&exchange=NSE&segment={seg}&symbol={sym}&timeframe=1min&from=2026-09&to=2026-09",
        ),
        "crates/api/src/bars_export.rs",
        &[400, 400, 400, 400, 400, 400, 400, 400],
    ),
    outside(
        "/spot.json",
        "spot::spot_json",
        Takes::Symbol(
            "feed=gdfl&exchange=NSE&symbol={sym}&timeframe=1s&month=2026-09&t=1790000000",
        ),
        "crates/api/src/spot.rs",
        &[200, 400, 400, 400, 200, 400, 400, 206],
    ),
    outside(
        "/greeks.json",
        "optiongreeks::greeks_json",
        Takes::Symbol(
            "feed=gdfl&exchange=NSE&segment=FNO&symbol={sym}&contract=2026-09-24-2400000-CE&timeframe=1s&month=2026-09&t=1790000000",
        ),
        "crates/api/src/optiongreeks.rs",
        &[200, 400, 400, 400, 200, 400, 400, 206],
    ),
    neither("/census/facets.json", "facets::facets_json"),
    neither("/census/warm.json", "census_cache::census_warm_json"),
    neither("/census/rollup.json", "census_rollup::rollup_json"),
    neither("/chain.json", "chain::chain_json"),
    neither("/imports.json", "imports::imports_json"),
    outside(
        "/basis.json",
        "basis_route::basis_json",
        Takes::Symbol("symbol={sym}"),
        "crates/api/src/basis_route.rs",
        &[200, 400, 400, 400, 200, 400, 400, 200],
    ),
    neither("/disk.json", "disk_route::disk_json"),
    neither("/scrub.json", "scrub_route::scrub_json"),
    neither("/ingest/refusals.json", "ingest_refusals::refusals_json"),
    neither("/dayflags.json", "dayflags::dayflags_json"),
    neither("/feed-checks.json", "feed_checks::feed_checks_json"),
    neither("/tick-checks.json", "feed_checks::tick_checks_json"),
    neither("/calendar-days.json", "feed_checks::calendar_days_json"),
    neither("/aliases.json", "aliases::aliases_json"),
    mine(
        "/sources.json",
        "sources::sources_json",
        Takes::Symbol("exchange=NSE&segment={seg}&symbol={sym}"),
    ),
    mine(
        "/sources/bars.json",
        "sources::sources_bars_json",
        Takes::Symbol(
            "exchange=NSE&segment={seg}&symbol={sym}&rung=1min&from=2026-09-01&to=2026-09-30",
        ),
    ),
    mine(
        "/sources/day.json",
        "sources::sources_day_json",
        Takes::Symbol("exchange=NSE&segment={seg}&symbol={sym}&from=2026-09"),
    ),
    neither("/provenance.json", "provenance::provenance_json"),
    neither("/store.json", "store_json"),
    neither("/verify.json", "verify_json"),
    neither("/pools.json", "crate::detail::pools_json"),
    neither("/masters.js", "closure"),
    neither("/typeahead.js", "closure"),
    neither("/pull", "pull_get"),
    neither("/pull/repair.json", "repair_plan_json"),
    outside(
        "/pull/quarantine.json",
        "quarantine_plan_json",
        Takes::Symbol("segment={seg}&symbol={sym}&from=2026-09-01&to=2026-09-30&cause=zero_price"),
        "crates/api/src/quarantine_route.rs",
        &[422, 422, 422, 400, 400, 400, 400, 422],
    ),
    neither("/pull/recovery.json", "crate::recovery::recent"),
    neither("/pull/run.json", "pull_run_json"),
    neither("/autopilot.json", "autopilot::status_json"),
    neither("/ingest/status.json", "crate::ingest::status_json"),
    neither("/audit.json", "audit_json::audit_json"),
    neither("/audit/page", "audit_get"),
    outside(
        "/store",
        "store_get",
        Takes::Symbol("symbol={sym}"),
        "crates/api/src/server.rs",
        &[200, 200, 200, 200, 200, 200, 200, 200],
    ),
    outside(
        "/bars",
        "bars_get",
        Takes::Symbol("feed=zerodha&symbol={sym}&timeframe=1min&month=2026-09"),
        "crates/api/src/server.rs",
        &[400, 400, 400, 400, 400, 400, 400, 503],
    ),
    neither("/logs", "crate::logs::logs_page"),
    neither("/logs.json", "crate::logs::logs_json"),
    neither("/backtest.json", "crate::backtest::backtest_json"),
    outside(
        "/trades.json",
        "crate::trades::trades_json",
        Takes::Identity("identity={id}"),
        "crates/api/src/trades.rs",
        &[200, 400, 400, 200, 200, 400, 503],
    ),
    mine(
        "/frontier.json",
        "crate::frontierjson::frontier_json",
        Takes::Identity("identity={id}"),
    ),
    mine(
        "/sweep-evidence.json",
        "crate::sweepevidence::evidence_json",
        Takes::Identity("identity={id}"),
    ),
    mine(
        "/candidate-trades.json",
        "crate::candidatejson::candidate_json",
        Takes::Identity("identity={id}&attempt=1"),
    ),
    mine(
        "/expression-search.json",
        "crate::expressionsearchjson::expression_search_json",
        Takes::Identity("identity={id}"),
    ),
    mine(
        "/boolean-candidates.json",
        "crate::booleanjson::boolean_json",
        Takes::Identity("identity={id}"),
    ),
    mine(
        "/boolean-statistics.json",
        "crate::booleanevidencejson::statistics_json",
        Takes::Identity("identity={id}"),
    ),
    mine(
        "/boolean-admission.json",
        "crate::booleanevidencejson::admission_json",
        Takes::Identity("identity={id}"),
    ),
    mine(
        "/boolean-qualification.json",
        "crate::booleanevidencejson::qualification_json",
        Takes::Identity("identity={id}"),
    ),
    outside(
        "/boolean-campaign.json",
        "crate::booleancampaignjson::campaign_json",
        Takes::Identity("identity={id}"),
        "crates/api/src/booleancampaignjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    outside(
        "/boolean-qualified-campaign.json",
        "crate::booleancampaignjson::qualified_campaign_json",
        Takes::Identity("identity={id}"),
        "crates/api/src/booleancampaignjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    outside(
        "/boolean-qualified-search.json",
        "crate::booleansearchjson::search_json",
        Takes::Identity("identity={id}"),
        "crates/api/src/booleansearchjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    outside(
        "/boolean-oos.json",
        "crate::booleanoosjson::later_json",
        Takes::Identity("identity={id}"),
        "crates/api/src/booleanoosjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    neither(
        "/selection-v6.json",
        "crate::selectionv6json::selection_v6_json",
    ),
    neither("/live.json", "crate::livejson::live_json"),
    neither(
        "/engine/boolean-launch.json",
        "crate::booleanlaunch::metadata",
    ),
    mine(
        "/index-stop.json",
        "crate::indexstopjson::index_stop_json",
        Takes::Identity("identity={id}"),
    ),
    outside(
        "/index-stop-candles.json",
        "crate::indexstopcandlesjson::index_stop_candles_json",
        Takes::Identity(
            "identity={id}&pin=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb&setting=0&kind=source",
        ),
        "crates/api/src/indexstopcandlesjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    neither(
        "/engine/index-stop-launch.json",
        "crate::indexstoplaunch::metadata",
    ),
    outside(
        "/index-stop-qualification.json",
        "crate::indexstopqualificationjson::index_stop_qualification_json",
        Takes::Identity("identity={id}"),
        "crates/api/src/indexstopqualificationjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    outside(
        "/index-stop-ranking.json",
        "crate::indexstoprankingjson::index_stop_ranking_json",
        Takes::Identity("identity={id}&timeframe=1min"),
        "crates/api/src/indexstoprankingjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    outside(
        "/index-stop-vix.json",
        "crate::indexstopvixjson::index_stop_vix_json",
        Takes::Identity(
            "identity={id}&pin=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb&setting=0",
        ),
        "crates/api/src/indexstopvixjson.rs",
        &[503, 400, 400, 400, 400, 400, 503],
    ),
    neither("/engine/top.json", "crate::sweeprun::top_json"),
    neither("/masters", "crate::mastersrun::page"),
    neither("/masters/status.json", "crate::mastersrun::status_json"),
    neither("/backtest/run.json", "crate::sweeprun::run_json"),
    neither("/backtest/audit.json", "crate::operation_audit::audit_json"),
    neither("/health", "health"),
    neither("/speed.json", "crate::speed_route::speed_json"),
];

/// **EVERY GET ROUTE OF `route_table` HAS A ROW, AND EVERY ROW IS A ROUTE** —
/// read from the source, so a route added without saying whether it takes a
/// symbol or an identity fails here, and a row whose handler changed is
/// re-read before it can pass.
#[test]
fn every_route_of_the_route_table_has_a_row() {
    let registered = registered_gets();
    assert_eq!(registered.len(), 82, "{registered:?}");
    let rows: Vec<(String, String)> = ROWS
        .iter()
        .map(|row| (row.path.to_owned(), row.handler.to_owned()))
        .collect();
    for route in &registered {
        assert!(
            rows.contains(route),
            "route_table serves {route:?} and no row of ROWS names it: say whether it takes a \
             symbol or an identity"
        );
    }
    for row in &rows {
        assert!(
            registered.contains(row),
            "ROWS names {row:?}, which route_table does not register"
        );
    }
    // A ROW TAKING NEITHER NAMES NO CASES, AND EVERY OTHER ROW HAS ITS CASES.
    for row in ROWS {
        match (row.takes, row.owner, row.today) {
            (Takes::Neither, Owner::Outside("takes neither"), None)
            | (Takes::Symbol(_) | Takes::Identity(_), Owner::ThisPart, None) => {}
            (Takes::Symbol(_), Owner::Outside(_), Some(today)) => {
                assert_eq!(today.len(), SYMBOL_CASES.len() + 1, "{}", row.path);
            }
            (Takes::Identity(_), Owner::Outside(_), Some(today)) => {
                assert_eq!(today.len(), IDENTITY_CASES.len() + 1, "{}", row.path);
            }
            other => panic!("{}: an inconsistent row {other:?}", row.path),
        }
    }
}

/// The real router over `root`, served on a port of its own, with a front end
/// of the test's own (never `web/build`). Answers the address, the stop and
/// the server task.
async fn serving(
    root: &Path,
    name: &str,
) -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let masters = scratch(&format!("{name}-masters"));
    let front = scratch(&format!("{name}-front"));
    std::fs::create_dir_all(front.join("build")).expect("front end");
    std::fs::write(
        front.join("build").join("index.html"),
        "<!doctype html><title>shell</title>",
    )
    .expect("shell");
    let site = std::sync::Arc::new(crate::server::Site::load(&masters, root));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let addr = listener.local_addr().expect("an address");
    let app = crate::server::router_serving(
        site,
        std::sync::Arc::new(crate::assets::Assets::new(&front)),
        addr,
    );
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let served = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .expect("serves");
    });
    (addr, stop, served)
}

/// One GET over a connection of its own: the status, the body, the time.
async fn get(addr: std::net::SocketAddr, target: String) -> (u16, String, std::time::Duration) {
    tokio::task::spawn_blocking(move || {
        use std::io::{Read as _, Write as _};
        let began = std::time::Instant::now();
        let mut stream = std::net::TcpStream::connect(addr).expect("connect");
        stream
            .write_all(
                format!("GET {target} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .expect("write");
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).expect("read");
        let took = began.elapsed();
        let raw = String::from_utf8_lossy(&raw).into_owned();
        let status = raw
            .get(9..12)
            .and_then(|code| code.parse().ok())
            .expect("a status line");
        let body = raw
            .split_once("\r\n\r\n")
            .map_or_else(String::new, |(_, body)| body.to_owned());
        (status, body, took)
    })
    .await
    .expect("the client thread")
}

/// `query` with `{sym}`, `{seg}` and `{id}` filled.
fn fill(query: &str, sym: &str, seg: &str, id: &str) -> String {
    query
        .replace("{sym}", sym)
        .replace("{seg}", seg)
        .replace("{id}", id)
}

/// Every case of `row` and the status it must answer: the root is an empty
/// store directory, or (`file_root`) a regular file.
fn cases(row: &Row, file_root: bool) -> Vec<(String, String, u16)> {
    let mut out = Vec::new();
    match row.takes {
        Takes::Neither => {}
        Takes::Symbol(query) => {
            let today = row.today;
            if file_root {
                let expect = today.map_or(FILE_ROOT, |today| today[SYMBOL_CASES.len()]);
                out.push((
                    "root is a file".to_owned(),
                    fill(query, "NIFTY", "INDEX", ""),
                    expect,
                ));
            } else {
                for (at, (name, sym, seg, decided)) in SYMBOL_CASES.iter().enumerate() {
                    let expect = today.map_or(*decided, |today| today[at]);
                    out.push(((*name).to_owned(), fill(query, sym, seg, ""), expect));
                }
            }
        }
        Takes::Identity(query) => {
            let today = row.today;
            if file_root {
                let expect = today.map_or(FILE_ROOT, |today| today[IDENTITY_CASES.len()]);
                out.push((
                    "root is a file".to_owned(),
                    fill(query, "", "", IDENTITY_CASES[0].1),
                    expect,
                ));
            } else {
                for (at, (name, id, decided)) in IDENTITY_CASES.iter().enumerate() {
                    let expect = today.map_or(*decided, |today| today[at]);
                    out.push(((*name).to_owned(), fill(query, "", "", id), expect));
                }
            }
        }
    }
    out
}

/// Whether `status` is what a case expects. A control's `0` asks only that
/// the row's query is well formed and nothing failed: an answer, or the one
/// not-found.
const fn answers(status: u16, expect: u16) -> bool {
    if expect == 0 {
        status < 400 || status == 404
    } else {
        status == expect
    }
}

/// The child of [`every_symbol_and_identity_route_answers_the_one_status_rule`]
/// that reads which root it was given.
const TABLE_CHILD: &str = "BRUTEX_STATUS_RULE_TABLE";

/// The file a child test writes what it says for its parent, named in the
/// child's environment. A FILE, NEVER A PRINT: a test's print surface is the
/// one `.github/workflows/ci.yml` declares for gate 23, and this file has none.
const SAID: &str = "BRUTEX_STATUS_RULE_SAID";

/// Appends `line` to `path`.
fn append(path: &std::path::Path, line: &str) {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("a file to append to");
    writeln!(file, "{line}").expect("a line appended");
}

/// What a child says to its parent: one line of the file [`SAID`] names.
fn say(line: &str) {
    append(
        std::path::Path::new(&std::env::var_os(SAID).expect("the parent names the file")),
        line,
    );
}

/// One measured figure, kept where a reader of the run finds it:
/// `<temp>/brutex-<pid>-status-rule-figures.txt`, the report's source.
fn figure(line: &str) {
    append(&crate::scratch::path("status-rule-figures.txt"), line);
}

/// [`crate::isolated::rerun`] of `test` with `env`, and what the child said.
fn rerun_saying(test: &str, name: &str, env: &[(&str, &std::ffi::OsStr)]) -> String {
    let said = scratch(name).join("said.txt");
    let mut env = env.to_vec();
    env.push((SAID, said.as_os_str()));
    let _ = crate::isolated::rerun(test, &env);
    std::fs::read_to_string(&said).expect("the child said what it found")
}

/// The child's run: every row's cases over the real router, every mismatch
/// gathered before the one assertion, so a run names all of them at once.
async fn table_child(file_root: bool) {
    let root = PathBuf::from(std::env::var_os("BRUTEX_STORE").expect("the child's store root"));
    let (addr, stop, served) =
        serving(&root, if file_root { "table-file" } else { "table-dir" }).await;
    let mut wrong = Vec::new();
    let mut seen = Vec::new();
    let mut asked = 0_usize;
    for row in ROWS {
        let mut observed = Vec::new();
        for (name, query, expect) in cases(row, file_root) {
            let (status, body, _) = get(addr, format!("{}?{query}", row.path)).await;
            asked += 1;
            observed.push(status);
            if !answers(status, expect) {
                let said: String = body.chars().take(220).collect();
                wrong.push(format!(
                    "{} [{name}] answered {status}, expected {expect} ({:?}): {said}",
                    row.path, row.owner
                ));
            }
            // A REFUSAL IS NEVER A 5xx FOR A CLIENT'S MALFORMED INPUT on this
            // part's routes, and the not-found is the one body (D-7550).
            if row.owner == Owner::ThisPart && status == 404 {
                let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
                if body["status"] != "not_found"
                    || body["rows"] != json!([])
                    || body["rule"] != DECISION
                {
                    wrong.push(format!(
                        "{} [{name}] 404 is not the one body: {body}",
                        row.path
                    ));
                }
            }
        }
        // WHAT AN OUTSIDE ROUTE ANSWERS, in case order, so its `today` is
        // copied from a measured run and never guessed.
        if row.today.is_some() {
            seen.push(format!("OBSERVED {} {observed:?}", row.path));
        }
    }
    let _ = stop.send(());
    let _ = served.await;
    assert!(
        wrong.is_empty(),
        "{} of {asked} answers differ:\n{}\n{}",
        wrong.len(),
        wrong.join("\n"),
        seen.join("\n")
    );
    say(&seen.join("\n"));
    say(&format!(
        "STATUS TABLE {} OK {asked}",
        if file_root { "file" } else { "dir" }
    ));
}

/// **THE TABLE, OVER THE REAL ROUTE TABLE (D-7550)** — every route taking a
/// symbol or an identity, each row sending its route's other required fields,
/// served by `router_serving` in a child whose `BRUTEX_STORE` is an empty
/// store directory, then a regular file: a lower-case, mixed-case, blank,
/// wrong-segment or Unicode-folding symbol is `400` before the store; an
/// unknown identity is the one `404`; a malformed one (63 or 65 digits, upper
/// case, all zeros, a NUL) is `400`; a root that is a file is `503`. Routes
/// outside this part are held to what they answer today, which
/// `docs/06-limits.md` L-STATUS-02 names where it differs from the rule.
/// Red on `feat/db-fix`: `/sources/*` answered `503` and the identity routes
/// four different statuses for the unknown identity.
#[test]
fn every_symbol_and_identity_route_answers_the_one_status_rule() {
    if let Some(kind) = std::env::var_os(TABLE_CHILD) {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(table_child(kind == "file"));
        return;
    }
    let dir = scratch("table-dir-root");
    let file = scratch("table-file-parent").join("store-is-a-file");
    std::fs::write(&file, b"not a store").expect("a file root");
    for (kind, root) in [("dir", &dir), ("file", &file)] {
        let said = rerun_saying(
            "status_rule::tests::every_symbol_and_identity_route_answers_the_one_status_rule",
            &format!("table-{kind}-said"),
            &[
                ("BRUTEX_STORE", root.as_os_str()),
                (TABLE_CHILD, std::ffi::OsStr::new(kind)),
            ],
        );
        assert!(said.contains(&format!("STATUS TABLE {kind} OK")), "{said}");
    }
}

/// p50, p99 and max of `samples`, in microseconds, and how many there were.
fn spread(samples: &mut [std::time::Duration]) -> (u128, u128, u128, usize) {
    samples.sort_unstable();
    let n = samples.len();
    let at = |q: usize| samples[(n * q / 100).min(n - 1)].as_micros();
    (at(50), at(99), samples[n - 1].as_micros(), n)
}

/// The 64 malformed requests of the parallel case: each identity route of
/// this part with each malformed identity case, each symbol route of this
/// part with each malformed symbol case, and one lower-case symbol beside a
/// malformed date. Every one is a `400`.
fn sixty_four_malformed() -> Vec<String> {
    let mut out = Vec::new();
    for row in ROWS.iter().filter(|row| row.owner == Owner::ThisPart) {
        match row.takes {
            Takes::Identity(query) => {
                for (_, id, decided) in IDENTITY_CASES.iter().filter(|case| case.2 == 400) {
                    assert_eq!(*decided, 400);
                    out.push(format!("{}?{}", row.path, fill(query, "", "", id)));
                }
            }
            Takes::Symbol(query) => {
                for (_, sym, seg, _) in SYMBOL_CASES.iter().skip(1) {
                    out.push(format!("{}?{}", row.path, fill(query, sym, seg, "")));
                }
            }
            Takes::Neither => {}
        }
    }
    out.push(
        "/sources/bars.json?exchange=NSE&segment=INDEX&symbol=nifty&rung=1min&from=2026-13-45"
            .to_owned(),
    );
    out
}

/// **COMBINATION: 64 PARALLEL CLIENTS, EACH A DIFFERENT MALFORMED INPUT,
/// ACROSS EVERY ROUTE OF THIS PART** — each answered its exact `400` with all
/// 64 in flight at once, none a `5xx` and none a `429`: a malformed question
/// is refused before admission, so a burst of them takes no detail slot from
/// a real read. p50, p99 and max printed for the burst and for the same 64
/// asked one at a time; the burst's p99 is never worse than the 64 asked in
/// series at their median.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sixty_four_parallel_malformed_clients_each_get_their_exact_refusal() {
    let root = scratch("parallel-root");
    let (addr, stop, served) = serving(&root, "parallel").await;
    let targets = sixty_four_malformed();
    assert_eq!(targets.len(), 64, "{targets:#?}");
    let mut series = Vec::with_capacity(64);
    for target in &targets {
        let (status, body, took) = get(addr, target.clone()).await;
        assert_eq!(status, 400, "{target}: {body}");
        series.push(took);
    }
    let burst: Vec<_> = targets
        .iter()
        .map(|target| tokio::spawn(get(addr, target.clone())))
        .collect();
    let mut parallel = Vec::with_capacity(64);
    for (target, handle) in targets.iter().zip(burst) {
        let (status, body, took) = handle.await.expect("a client");
        assert_eq!(status, 400, "{target} in the burst: {body}");
        parallel.push(took);
    }
    let _ = stop.send(());
    let _ = served.await;
    let (s50, s99, smax, n) = spread(&mut series);
    let (p50, p99, pmax, m) = spread(&mut parallel);
    figure(&format!(
        "STATUS 400 series p50 {s50} us p99 {s99} us max {smax} us n {n}"
    ));
    figure(&format!(
        "STATUS 400 burst-of-64 p50 {p50} us p99 {p99} us max {pmax} us n {m}"
    ));
    assert!(
        p99 <= 64 * s50 + 50_000,
        "the burst's p99 {p99} us is worse than 64 asked in series at their median {s50} us"
    );
}

/// **COMBINATION: A LOWER-CASE SYMBOL AND A MALFORMED DATE AT ONCE** — one
/// answer, the first refusal in the route's declared order (exchange,
/// segment, symbol, then the rung and the range): the symbol, never the date
/// and never two sentences.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lower_case_symbol_beside_a_malformed_date_is_refused_for_the_symbol() {
    let root = scratch("order-root");
    let (addr, stop, served) = serving(&root, "order").await;
    for target in [
        "/sources/bars.json?exchange=NSE&segment=INDEX&symbol=nifty&rung=1min&from=2026-13-45",
        "/sources/day.json?exchange=NSE&segment=INDEX&symbol=nifty&from=2026-13",
        "/sources.json?exchange=NSE&segment=INDEX&symbol=nifty&from=garbage",
    ] {
        let (status, body, _) = get(addr, target.to_owned()).await;
        assert_eq!(status, 400, "{target}: {body}");
        let body: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        let said = body["error"].as_str().expect("the refusal");
        assert!(
            said.starts_with("symbol=\"nifty\" is malformed"),
            "{target}: {said}"
        );
        assert!(!said.contains("2026-13"), "one refusal, not two: {said}");
    }
    let _ = stop.send(());
    let _ = served.await;
}

/// The store sizes the timing compares: one, ten and a hundred times.
const SIZES: [usize; 3] = [16, 160, 1_600];

/// Requests timed per route, per kind, per size.
const TIMED: usize = 200;

/// The child of the timing test that reads its switched root.
const TIMING_CHILD: &str = "BRUTEX_STATUS_RULE_TIMING";

/// One of this part's identity routes, called in process through its public
/// handler: the status and the body.
async fn call(path: &str, query: &str) -> (StatusCode, String) {
    let uri: axum::http::Uri = format!("{path}?{query}").parse().expect("a URI");
    let (status, _, body) = match path {
        "/boolean-candidates.json" => crate::booleanjson::boolean_json(uri).await,
        "/boolean-statistics.json" => crate::booleanevidencejson::statistics_json(uri).await,
        "/boolean-admission.json" => crate::booleanevidencejson::admission_json(uri).await,
        "/boolean-qualification.json" => crate::booleanevidencejson::qualification_json(uri).await,
        "/index-stop.json" => crate::indexstopjson::index_stop_json(uri).await,
        "/frontier.json" => crate::frontierjson::frontier_json(uri).await,
        "/expression-search.json" => crate::expressionsearchjson::expression_search_json(uri).await,
        "/sweep-evidence.json" => crate::sweepevidence::evidence_json(uri).await,
        "/candidate-trades.json" => crate::candidatejson::candidate_json(uri).await,
        other => panic!("{other} is not an identity route of this part"),
    };
    (status, body)
}

/// A store root of `size`: that many saved entries in every namespace this
/// part probes, and that many index instruments' minute folders.
fn sized_root(at: &Path, size: usize) {
    use crate::status_rule::namespace;
    for space in [
        namespace::BOOLEAN_CANDIDATES,
        namespace::BOOLEAN_STATISTICS,
        namespace::BOOLEAN_ADMISSION,
        namespace::BOOLEAN_QUALIFICATION,
        namespace::INDEX_STOP,
        namespace::EXPRESSION_SEARCH,
        namespace::SWEEP_EVIDENCE,
        namespace::CANDIDATE_TRADES,
    ] {
        for n in 0..size {
            let mut id = [0x11_u8; 32];
            id[24..].copy_from_slice(&u64::try_from(n).expect("small").to_be_bytes());
            std::fs::create_dir_all(at.join(space).join(crate::server::hex32(id)))
                .expect("an entry");
        }
    }
    for n in 0..size {
        std::fs::create_dir_all(
            at.join("bars/zerodha/NSE/INDEX")
                .join(format!("SYM{n}"))
                .join("1min"),
        )
        .expect("an instrument");
    }
}

/// Points the link `current` at `target`, atomically: a new link renamed
/// over the old one.
fn point(current: &Path, target: &Path) {
    let fresh = current.with_extension("next");
    let _ = std::fs::remove_file(&fresh);
    std::os::unix::fs::symlink(target, &fresh).expect("a link");
    std::fs::rename(&fresh, current).expect("the link swapped");
}

/// The timing child: every identity route's not-found and malformed `400` in
/// process, and the sources routes' `400` over three servers, interleaved
/// across the three sizes sample by sample.
async fn timing_child() {
    let current = PathBuf::from(std::env::var_os("BRUTEX_STORE").expect("the link"));
    let parent = current.parent().expect("the link's folder").to_path_buf();
    let roots: Vec<PathBuf> = SIZES
        .iter()
        .map(|size| parent.join(format!("size-{size}")))
        .collect();
    let mut servers = Vec::new();
    for (root, size) in roots.iter().zip(SIZES) {
        servers.push(serving(root, &format!("timing-{size}")).await);
    }
    let identity_rows: Vec<(&str, &str)> = ROWS
        .iter()
        .filter(|row| row.owner == Owner::ThisPart)
        .filter_map(|row| match row.takes {
            Takes::Identity(query) => Some((row.path, query)),
            _ => None,
        })
        .collect();
    let symbol_rows: Vec<(&str, &str)> = ROWS
        .iter()
        .filter(|row| row.owner == Owner::ThisPart)
        .filter_map(|row| match row.takes {
            Takes::Symbol(query) => Some((row.path, query)),
            _ => None,
        })
        .collect();
    // (route, kind, size) -> samples
    let mut taken: std::collections::BTreeMap<(String, &str, usize), Vec<std::time::Duration>> =
        std::collections::BTreeMap::new();
    for sample in 0..TIMED {
        for turn in 0..SIZES.len() {
            let at = (sample + turn) % SIZES.len();
            point(&current, &roots[at]);
            for (path, query) in &identity_rows {
                for (kind, id, expect) in [
                    ("not-found", IDENTITY_CASES[0].1, StatusCode::NOT_FOUND),
                    ("400", IDENTITY_CASES[3].1, StatusCode::BAD_REQUEST),
                ] {
                    let began = std::time::Instant::now();
                    let (status, body) = call(path, &fill(query, "", "", id)).await;
                    let took = began.elapsed();
                    assert_eq!(status, expect, "{path} {kind} at {}: {body}", SIZES[at]);
                    taken
                        .entry(((*path).to_owned(), kind, SIZES[at]))
                        .or_default()
                        .push(took);
                }
            }
            for (path, query) in &symbol_rows {
                let target = format!("{path}?{}", fill(query, "nifty", "INDEX", ""));
                let (status, body, took) = get(servers[at].0, target).await;
                assert_eq!(status, 400, "{path} at {}: {body}", SIZES[at]);
                taken
                    .entry(((*path).to_owned(), "400", SIZES[at]))
                    .or_default()
                    .push(took);
            }
        }
    }
    for (addr_stop_served, _) in servers.into_iter().zip(SIZES) {
        let (_, stop, served) = addr_stop_served;
        let _ = stop.send(());
        let _ = served.await;
    }
    let mut flat = Vec::new();
    let routes: std::collections::BTreeSet<(String, &str)> = taken
        .keys()
        .map(|(route, kind, _)| (route.clone(), *kind))
        .collect();
    for (route, kind) in routes {
        let mut p50s = Vec::new();
        for size in SIZES {
            let samples = taken
                .get_mut(&(route.clone(), kind, size))
                .expect("every size measured");
            let (p50, p99, max, n) = spread(samples);
            say(&format!(
                "STATUS TIMING {route} {kind} store {size}: p50 {p50} us p99 {p99} us max {max} us n {n}"
            ));
            p50s.push(p50);
        }
        // FLAT: the hundredfold store's median within three times the
        // onefold's, plus the scheduling floor of one blocking hand-off.
        if p50s[2] > 3 * p50s[0] + 200 {
            flat.push(format!("{route} {kind}: p50 {p50s:?} us at {SIZES:?}"));
        }
    }
    assert!(flat.is_empty(), "not flat:\n{}", flat.join("\n"));
    say("STATUS TIMING OK");
}

/// **EACH 400 AND EACH NOT-FOUND COSTS THE SAME AT ONE, TEN AND A HUNDRED
/// TIMES THE STORE** — p50, p99 and max of every identity route's `404` and
/// `400` and every sources route's `400`, n = 200 per size, the three sizes
/// interleaved sample by sample (a symbolic link switched between three real
/// roots), so a stall hits every size alike. A `400` is decided before the
/// store; a not-found is one `stat`, never a listing.
#[test]
fn a_refusal_and_a_not_found_cost_the_same_at_one_ten_and_a_hundred_times_the_store() {
    if std::env::var_os(TIMING_CHILD).is_some() {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(timing_child());
        return;
    }
    let dir = scratch("timing");
    for size in SIZES {
        sized_root(&dir.join(format!("size-{size}")), size);
    }
    let current = dir.join("current");
    point(&current, &dir.join(format!("size-{}", SIZES[0])));
    let said = rerun_saying(
        "status_rule::tests::a_refusal_and_a_not_found_cost_the_same_at_one_ten_and_a_hundred_times_the_store",
        "timing-said",
        &[
            ("BRUTEX_STORE", current.as_os_str()),
            (TIMING_CHILD, std::ffi::OsStr::new("1")),
        ],
    );
    for line in said
        .lines()
        .filter(|line| line.starts_with("STATUS TIMING"))
    {
        figure(line);
    }
    assert!(said.contains("STATUS TIMING OK"), "{said}");
}

// ══ The one path: serve and page ═══════════════════════════════════════════

/// A route's refusal shape, for the path's own tests.
fn refusal(status: StatusCode, why: &str) -> Reply {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json!({ "status": "refused", "refusal": why }).to_string(),
    )
}

/// **A PAGE, ITS FAULT, AND A PAGE TOO LARGE** — a page within the bound is
/// `200` with the route's content type; a fault is its class's status with
/// the one body; a page one byte over the bound is the route's own `503`
/// sentence, no prefix.
#[test]
fn a_page_is_served_its_fault_answered_and_an_oversized_page_refused_whole() {
    let ok = page(
        Ok("{}".to_owned()),
        Some([1; 32]),
        "application/x-test",
        refusal,
        "big",
    );
    assert_eq!(
        ok,
        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/x-test")],
            "{}".to_owned()
        )
    );
    let at_bound = "x".repeat(crate::detail::MAX_RESPONSE_BYTES);
    assert_eq!(
        page(Ok(at_bound), None, "a/b", refusal, "big").0,
        StatusCode::OK
    );
    let over = "x".repeat(crate::detail::MAX_RESPONSE_BYTES + 1);
    let (status, _, body) = page(Ok(over), None, "a/b", refusal, "too big; no prefix");
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains("too big; no prefix"), "{body}");
    assert!(!body.contains("xxx"), "no prefix: {body:.80}");
    let root = scratch("page-fault");
    let fault = probe(&root, Path::new("nothing"), "Thing").unwrap_err();
    let (status, headers, body) = page(Err(fault), Some([2; 32]), "a/b", refusal, "big");
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers[0].1, "a/b");
    let body: Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(body["status"], "not_found");
    assert_eq!(body["identity"], hex_of(2));
}

/// **A MALFORMED QUESTION IS ANSWERED BEFORE ADMISSION, AND A LOST TASK IS
/// `503`** — `serve` hands back the route's own `400` without running the
/// work, runs the work for a parsed question, and answers a task that
/// panicked with the route's `503` naming the join, never a hang or a `200`.
#[test]
fn serve_refuses_before_admission_runs_the_work_and_names_a_lost_task() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::clone(&ran);
    let refused = runtime.block_on(serve(
        Err::<(), Reply>(refusal(StatusCode::BAD_REQUEST, "bad")),
        refusal,
        "full",
        move |()| {
            seen.store(true, std::sync::atomic::Ordering::SeqCst);
            refusal(StatusCode::OK, "ran")
        },
    ));
    assert_eq!(refused.0, StatusCode::BAD_REQUEST);
    assert!(
        !ran.load(std::sync::atomic::Ordering::SeqCst),
        "the work never ran"
    );
    let served = runtime.block_on(serve(Ok(7_u8), refusal, "full", |n| {
        refusal(StatusCode::OK, &n.to_string())
    }));
    assert_eq!(served.0, StatusCode::OK);
    assert!(served.2.contains("\"7\""), "{}", served.2);
    let lost = runtime.block_on(serve(Ok(()), refusal, "full", |()| -> Reply {
        panic!("the blocking task is lost")
    }));
    assert_eq!(lost.0, StatusCode::SERVICE_UNAVAILABLE);
    assert!(lost.2.contains("could not be joined"), "{}", lost.2);
}

/// The child of the full-pool test.
const POOL_CHILD: &str = "BRUTEX_STATUS_RULE_POOL";

/// **A FULL DETAIL POOL IS `429` WITH THE ROUTE'S OWN SENTENCE, AND NOTHING
/// QUEUED** — in a child, alone, so holding every slot crowds no other test:
/// four held tasks, then one more question waits its admission and is refused.
#[test]
fn a_full_detail_pool_is_429_with_the_routes_sentence() {
    if std::env::var_os(POOL_CHILD).is_none() {
        let said = rerun_saying(
            "status_rule::tests::a_full_detail_pool_is_429_with_the_routes_sentence",
            "pool-said",
            &[(POOL_CHILD, std::ffi::OsStr::new("1"))],
        );
        assert!(said.contains("STATUS POOL OK"), "{said}");
        return;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let (started, running) = std::sync::mpsc::channel::<()>();
        let mut releases = Vec::new();
        let mut held = Vec::new();
        for _ in 0..crate::detail::MAX_CONCURRENT {
            let (release, wait) = std::sync::mpsc::channel::<()>();
            releases.push(release);
            let started = started.clone();
            held.push(tokio::spawn(crate::detail::run(move || {
                let _ = started.send(());
                let _ = wait.recv();
            })));
        }
        for _ in 0..crate::detail::MAX_CONCURRENT {
            running.recv().expect("a held slot");
        }
        let full = serve(Ok(()), refusal, "capacity full; nothing queued", |()| {
            refusal(StatusCode::OK, "admitted")
        })
        .await;
        assert_eq!(full.0, StatusCode::TOO_MANY_REQUESTS, "{}", full.2);
        assert!(
            full.2.contains("capacity full; nothing queued"),
            "{}",
            full.2
        );
        for release in releases {
            let _ = release.send(());
        }
        for slot in held {
            assert!(slot.await.expect("a held task").is_ok());
        }
    });
    say("STATUS POOL OK");
}

// ══ Every namespace is the one its reader opens ════════════════════════════

/// The text a reader's refusal carries when a path component it walks is a
/// regular file: the kernel's own words for `ENOTDIR`.
fn not_a_directory() -> String {
    std::io::Error::from_raw_os_error(20).to_string()
}

/// One Boolean reader's open of an identity's entry under a root, at a byte
/// budget: `Ok` when it opened, or its refusal.
type Opens = fn(&Path, [u8; 32], u64) -> Result<(), String>;

/// **EVERY NAMESPACE THE PROBE USES IS THE ONE ITS `cli` READER OPENS** — a
/// probe looking anywhere else would answer `404` for every saved result. Each
/// reader is asked on an empty root and then with something planted exactly
/// where the constant says the entry is: the typed readers stop answering
/// `Ok(None)`, and the four Boolean readers, given a regular file where the
/// entry's folder should be, meet `ENOTDIR` — they could not have if they
/// looked under another name.
#[test]
fn every_probed_namespace_is_the_one_its_reader_opens() {
    use crate::status_rule::namespace;
    assert_eq!(namespace::INDEX_STOP, cli::index_stop_store::NAMESPACE);
    let id = [0x5a_u8; 32];
    let hex = crate::server::hex32(id);
    let max = crate::detail::MAX_SCAN_BYTES;

    let root = scratch("bind-expression");
    assert!(matches!(
        cli::expression_search::reader::Reader::open(&root, id, max),
        Ok(None)
    ));
    std::fs::create_dir_all(root.join(namespace::EXPRESSION_SEARCH).join(&hex)).expect("dir");
    assert!(!matches!(
        cli::expression_search::reader::Reader::open(&root, id, max),
        Ok(None)
    ));

    let root = scratch("bind-sweep");
    assert_eq!(cli::sweep_evidence::read(&root, id, max), Ok(None));
    let entry = root.join(namespace::SWEEP_EVIDENCE).join(&hex);
    std::fs::create_dir_all(&entry).expect("dir");
    std::fs::write(entry.join("starts.bin"), [0xde_u8; 64]).expect("bytes that are no events");
    assert_ne!(cli::sweep_evidence::read(&root, id, max), Ok(None));

    for (model, space) in [
        (
            cli::candidate_trades::Model::And,
            namespace::CANDIDATE_TRADES,
        ),
        (
            cli::candidate_trades::Model::Expression,
            namespace::EXPRESSION_CANDIDATE_TRADES,
        ),
    ] {
        let root = scratch(&format!("bind-candidates-{}", model.as_str()));
        assert!(matches!(
            cli::candidate_trades::read_model(&root, id, 3, model, max),
            Ok(None)
        ));
        let entry = root.join(space).join(&hex).join("3");
        std::fs::create_dir_all(&entry).expect("dir");
        std::fs::write(entry.join("catalog.bin"), [0xde_u8; 64])
            .expect("bytes that are no catalog");
        assert!(!matches!(
            cli::candidate_trades::read_model(&root, id, 3, model, max),
            Ok(None)
        ));
    }

    let opened: [(&str, Opens); 4] = [
        (namespace::BOOLEAN_CANDIDATES, |root, id, max| {
            cli::boolean_observation::Reader::open(root, id, max).map(drop)
        }),
        (namespace::BOOLEAN_STATISTICS, |root, id, max| {
            cli::boolean_evidence::Statistics::open(root, id, max).map(drop)
        }),
        (namespace::BOOLEAN_ADMISSION, |root, id, max| {
            cli::boolean_evidence::Admission::open(root, id, max).map(drop)
        }),
        (namespace::BOOLEAN_QUALIFICATION, |root, id, max| {
            cli::boolean_evidence::Qualification::open(root, id, max).map(drop)
        }),
    ];
    for (space, open) in opened {
        let root = scratch(&format!("bind-{space}"));
        let before = open(&root, id, max).expect_err("nothing saved");
        assert!(!before.contains(&not_a_directory()), "{space}: {before}");
        std::fs::create_dir_all(root.join(space)).expect("namespace");
        std::fs::write(root.join(space).join(&hex), b"a file where the folder goes").expect("file");
        let after = open(&root, id, max).expect_err("a file is no saved result");
        assert!(
            after.contains(&not_a_directory()),
            "{space} reads elsewhere: {after}"
        );
        // AND THE PROBE CALLS THE SAME ENTRY HELD: there is something there.
        assert!(probe_identity(&root, space, id, "Thing").is_ok());
    }

    // A SAVED CATALOG THE READER OPENS IS ONE THE PROBE FINDS.
    let root = crate::booleanjson::tests::fixture("bind-catalog");
    assert!(cli::boolean_observation::Reader::open(&root, [0x42; 32], max).is_ok());
    assert!(probe_identity(&root, namespace::BOOLEAN_CANDIDATES, [0x42; 32], "Thing").is_ok());
    let _ = std::fs::remove_dir_all(root);
}
