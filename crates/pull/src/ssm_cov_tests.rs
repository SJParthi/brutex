//! BASE-COV-PULL: the date a signature is scoped to and the words of a
//! refusal, no network and no credential: `SigV4` is deterministic and these
//! are its inputs. The identity below is invented.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

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
    assert_eq!(civil_from_days(790), (1972, 2, 29));
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

/// This process's environment answers as the lookup behind it does: the
/// identity is there exactly when both of its variables are.
#[test]
fn the_environment_door_is_the_lookup_over_this_process() {
    let set = |name: &str| std::env::var(name).is_ok_and(|value| !value.trim().is_empty());
    assert_eq!(
        AwsIdentity::from_env().is_ok(),
        set("AWS_ACCESS_KEY_ID") && set("AWS_SECRET_ACCESS_KEY")
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
