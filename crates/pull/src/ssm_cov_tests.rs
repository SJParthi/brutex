//! BASE-COV-PULL: the date a signature is scoped to and the words of a
//! refusal, no network and no credential: `SigV4` is deterministic and these
//! are its inputs. The identity below is invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// The civil date of a day count, on the edges where the closed form turns:
/// the epoch, a leap day, the turn of a century not divisible by 400, a day
/// before the epoch, and each side of March, where its year turns.
#[test]
fn a_day_count_is_its_civil_date_on_every_edge() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(-1), (1969, 12, 31));
    assert_eq!(civil_from_days(58), (1970, 2, 28));
    assert_eq!(civil_from_days(59), (1970, 3, 1));
    assert_eq!(civil_from_days(789), (1972, 2, 29));
    assert_eq!(civil_from_days(10_957), (2000, 1, 1));
    assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    assert_eq!(civil_from_days(47_541), (2100, 3, 1));
    assert_eq!(civil_from_days(47_540), (2100, 2, 28));
    assert_eq!(civil_from_days(-719_468), (0, 3, 1));
}

/// Now, as `SigV4` spells it: sixteen characters, a `T` and a `Z`, in a year
/// the clock can be in.
#[test]
fn the_signature_stamp_is_the_shape_aws_reads() {
    let stamp = now_stamp().unwrap();
    assert_eq!(stamp.len(), 16, "{stamp}");
    assert_eq!(stamp.as_bytes()[8], b'T', "{stamp}");
    assert!(stamp.ends_with('Z'), "{stamp}");
    assert!(stamp.as_str() >= "20260101T000000Z", "{stamp}");
}

#[test]
fn a_refusal_says_its_detail_and_keeps_its_kind() {
    let refusal = SsmError::unreachable("an invented reason".to_owned());
    assert_eq!(refusal.to_string(), "an invented reason");
    assert!(matches!(refusal.into_secret(), SecretError::Unreachable));
}

/// This process's environment answers as the lookup behind it does: the door
/// and the rule over the same variables give the same identity or the same
/// refusal (the `Debug` redacts the secret). No side of this test depends on
/// which variables the process holds; the rule's present and absent sides
/// are driven by `a_lookup_with_one_half_of_the_pair_names_the_missing_one`.
#[test]
fn the_environment_door_is_the_lookup_over_this_process() {
    assert_eq!(
        format!("{:?}", AwsIdentity::from_env()),
        format!(
            "{:?}",
            AwsIdentity::from_lookup(|name| std::env::var(name).ok())
        )
    );
}

/// The rule over an invented lookup, with each variable of the pair present
/// and absent: the identity is built exactly when both are there and not
/// blank, and a refusal names the one that is missing.
#[test]
fn a_lookup_with_one_half_of_the_pair_names_the_missing_one() {
    let lookup = |held: &'static [(&'static str, &'static str)]| {
        move |name: &str| {
            held.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        }
    };
    let both = AwsIdentity::from_lookup(lookup(&[
        ("AWS_ACCESS_KEY_ID", "ZQKEY"),
        ("AWS_SECRET_ACCESS_KEY", "ZQSECRET"),
    ]))
    .unwrap();
    assert_eq!(
        (both.key_id.as_str(), both.secret.as_str()),
        ("ZQKEY", "ZQSECRET")
    );
    assert_eq!(both.session_token, None);
    let no_secret = AwsIdentity::from_lookup(lookup(&[("AWS_ACCESS_KEY_ID", "ZQKEY")]))
        .unwrap_err()
        .to_string();
    assert!(
        no_secret.contains("AWS_SECRET_ACCESS_KEY is unset"),
        "{no_secret}"
    );
    let blank_key = AwsIdentity::from_lookup(lookup(&[
        ("AWS_ACCESS_KEY_ID", "  "),
        ("AWS_SECRET_ACCESS_KEY", "ZQSECRET"),
    ]))
    .unwrap_err()
    .to_string();
    assert!(
        blank_key.contains("AWS_ACCESS_KEY_ID is unset"),
        "{blank_key}"
    );
}

/// A line of a credentials file with no `=` is passed over, and a name this
/// reader does not use changes nothing.
#[test]
fn a_credentials_file_passes_over_what_it_does_not_read() {
    let root = crate::gdfl_fixtures::scratch("cov-credentials");
    let path = root.join("credentials");
    std::fs::write(
        &path,
        "[default]\nno equals sign here\nregion = invented-1\naws_access_key_id = ZQKEY\n\
         aws_secret_access_key = ZQSECRET\n",
    )
    .unwrap();
    assert!(AwsIdentity::from_credentials_file(&path, "default").is_ok());
    assert!(AwsIdentity::from_credentials_file(&path, "other").is_err());
    std::fs::remove_dir_all(root).unwrap();
}

// ───── BASE-COV-PULL repair 2 (D-7334): the answer, without a socket ─────

/// **EVERY ANSWER OF `GetParameter` IS READ BY ITS STATUS AND FAULT NAME**
/// (repair 2): a denial by its fault name or by 403 alone, a missing
/// parameter by its fault name or by 404 alone, any other status
/// unreachable, the body never quoted beyond a known fault name; a 200 whose
/// value is empty is no credential, one that is no JSON or has no value is
/// refused, and one with a value answers it. Extremes at once: a fault name
/// under a 200-range miss (299 is success, 300 is not), and a 403 whose body
/// names the other fault.
#[test]
fn every_answer_is_read_by_its_status_and_fault_name() {
    let kind = |status: u16, text: &str| answer_of("ap-south-1", status, text).unwrap_err().kind;
    assert_eq!(
        kind(400, "AccessDeniedException"),
        SecretError::AccessDenied
    );
    assert_eq!(kind(403, "ParameterNotFound"), SecretError::AccessDenied);
    assert_eq!(kind(403, ""), SecretError::AccessDenied);
    assert_eq!(kind(400, "ParameterNotFound"), SecretError::NotFound);
    assert_eq!(kind(404, ""), SecretError::NotFound);
    assert_eq!(kind(500, "an invented outage"), SecretError::Unreachable);
    assert_eq!(kind(300, ""), SecretError::Unreachable);
    assert_eq!(kind(199, ""), SecretError::Unreachable);
    let denied = answer_of(
        "ap-south-1",
        400,
        "User: arn:aws:invented is not authorized: AccessDeniedException",
    )
    .unwrap_err();
    assert!(!denied.detail.contains("arn:aws"), "{}", denied.detail);
    let empty = answer_of("ap-south-1", 200, r#"{"Parameter":{"Value":""}}"#).unwrap_err();
    assert_eq!(empty.kind, SecretError::Empty);
    assert!(empty.detail.contains("holds nothing"), "{}", empty.detail);
    let not_json = answer_of("ap-south-1", 200, "not json").unwrap_err();
    assert!(
        not_json.detail.contains("is not JSON"),
        "{}",
        not_json.detail
    );
    let no_value = answer_of("ap-south-1", 299, r#"{"Parameter":{}}"#).unwrap_err();
    assert!(
        no_value.detail.contains("no Parameter.Value"),
        "{}",
        no_value.detail
    );
    assert_eq!(
        answer_of(
            "ap-south-1",
            200,
            r#"{"Parameter":{"Value":"zq-invented"}}"#
        )
        .unwrap(),
        "zq-invented"
    );
}

/// The variable a child of the discovery test reads its case from.
const CASE: &str = "BRUTEX_SSM_COV_CASE";

/// This test's name, which a child runs `--exact`.
const DISCOVERY: &str =
    "ssm::cov_tests::the_discovery_doors_read_this_processs_environment_and_home";

/// **THE DISCOVERY DOORS READ THIS PROCESS'S ENVIRONMENT AND HOME** (repair
/// 2): each case is a child process of this binary with its own environment,
/// never this one's (no test may set a variable here): both variables set is
/// the environment's identity; none set is the credentials file under HOME;
/// HOME unset, HOME relative, and an identity variable that is not UTF-8 are
/// each refused by name. The HOME given is a scratch folder holding an
/// invented credentials file; the operator's own is never read.
#[test]
fn the_discovery_doors_read_this_processs_environment_and_home() {
    match std::env::var(CASE).ok() {
        Some(case) => discovery_case(&case),
        None => {
            let home = crate::gdfl_fixtures::scratch("ssm-cov-home");
            std::fs::create_dir_all(home.join(".aws")).unwrap();
            std::fs::write(
                home.join(".aws").join("credentials"),
                "[default]\naws_access_key_id = ZQFILEKEY\naws_secret_access_key = ZQFILESECRET\n",
            )
            .unwrap();
            let relative = std::path::Path::new("relative-home");
            let pair = [
                ("AWS_ACCESS_KEY_ID", std::ffi::OsStr::new("ZQKEY")),
                ("AWS_SECRET_ACCESS_KEY", std::ffi::OsStr::new("ZQSECRET")),
            ];
            let not_utf8 = {
                use std::os::unix::ffi::OsStrExt as _;
                [("AWS_PROFILE", std::ffi::OsStr::from_bytes(&[0xFF, 0xFE]))]
            };
            child("env", &pair, Some(&home));
            child("file", &[], Some(&home));
            child("no-home", &[], None);
            child("relative-home", &[], Some(relative));
            child("not-utf8", &not_utf8, Some(&home));
            std::fs::remove_dir_all(home).unwrap();
        }
    }
}

/// One case, in the child that was started for it.
fn discovery_case(case: &str) {
    let found = AwsIdentity::discover().map(|id| id.key_id);
    let said = format!("{found:?}");
    match case {
        "env" => assert_eq!(found, Ok("ZQKEY".to_owned())),
        "file" => assert_eq!(found, Ok("ZQFILEKEY".to_owned())),
        "no-home" => assert!(said.contains("HOME is unset"), "{said}"),
        "relative-home" => assert!(said.contains("which is relative"), "{said}"),
        _ => {
            assert!(
                said.contains("AWS_PROFILE is set but is not valid UTF-8"),
                "{said}"
            );
            let door = format!("{:?}", AwsIdentity::from_env());
            assert!(
                door.contains("AWS_PROFILE is set but is not valid UTF-8"),
                "{door}"
            );
        }
    }
}

/// Runs [`DISCOVERY`] again in a child of this binary for `case`, every AWS
/// identity variable removed and then `set`, with `home` as HOME or HOME
/// removed, and asserts the child ran it and passed.
fn child(case: &str, set: &[(&str, &std::ffi::OsStr)], home: Option<&std::path::Path>) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", DISCOVERY, "--test-threads=1"])
        .env(CASE, case);
    for name in AWS_IDENTITY_VARS {
        command.env_remove(name);
    }
    for (name, value) in set {
        command.env(name, value);
    }
    match home {
        Some(home) => command.env("HOME", home),
        None => command.env_remove("HOME"),
    };
    let output = command.output().unwrap();
    let said = format!(
        "{case}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{said}");
    assert!(said.contains("1 passed"), "{said}");
}
