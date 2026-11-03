#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): the Dropbox token as `ParameterStoreToken`
//! reads it (`CLAUDE.md` §8): through the credential reader, over an
//! in-memory secret source a test answers, never AWS. Every path segment and
//! value is invented.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::config::CredentialPath;
use crate::secret::SecretError;

/// A credential config naming the token field for `gdfl` (`with`) or naming
/// no field of `gdfl` at all.
fn config(with: bool) -> CredentialConfig {
    let vendor = if with {
        "[vendor.gdfl]\nvendor = \"vendorthree\"\nfields = [\"dropbox_token\"]\n"
    } else {
        "[vendor.dhan]\nvendor = \"vendortwo\"\nfields = [\"fieldone\"]\n"
    };
    CredentialConfig::parse(&format!(
        "org    = \"orgone\"\nenv    = \"testenv\"\nregion = \"ap-south-1\"\n\n{vendor}"
    ))
    .unwrap()
}

/// A secret source that answers every read the same.
struct Answers(Result<&'static str, SecretError>);

impl SecretSource for Answers {
    fn read(&self, _path: &CredentialPath<'_>) -> Result<Secret, SecretError> {
        self.0.and_then(|value| Secret::new(value.to_owned()))
    }
}

fn secret(value: &str) -> Secret {
    Secret::new(value.to_owned()).unwrap()
}

/// The token is read from the path the config names; a re-read after a
/// rejection that returns the same value is a dead token, never a retry with
/// it; a source that refuses is a missing token in the source's words; and
/// a config that names no token field for `gdfl` is a missing token on both
/// doors (extremes at once: the rejected value read again, and no field).
#[test]
fn the_dropbox_token_is_read_where_the_config_names_it_and_a_dead_one_is_named() {
    let token = ParameterStoreToken::new(
        config(true),
        CredentialReader::new(Answers(Ok("zq-invented-token"))),
    );
    let read = token.read().unwrap();
    assert!(read == secret("zq-invented-token"));
    assert_eq!(token.reread(&read).unwrap_err(), Wait::TokenDead);
    let refused = ParameterStoreToken::new(
        config(true),
        CredentialReader::new(Answers(Err(SecretError::NotFound))),
    );
    let Wait::TokenMissing { why } = refused.read().unwrap_err() else {
        panic!("a refused read is a missing token");
    };
    assert!(!why.is_empty());
    let unnamed = ParameterStoreToken::new(
        config(false),
        CredentialReader::new(Answers(Ok("zq-invented-token"))),
    );
    let Wait::TokenMissing { why } = unnamed.read().unwrap_err() else {
        panic!("no field is a missing token");
    };
    assert!(
        why.starts_with("the credential config names no dropbox_token for gdfl: "),
        "{why}"
    );
    let Wait::TokenMissing { why } = unnamed.reread(&read).unwrap_err() else {
        panic!("no field is a missing token on the re-read too");
    };
    assert!(!why.is_empty());
}
