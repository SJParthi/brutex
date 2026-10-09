//! How an environment value is read when it names a folder or a switch.
//!
//! # Why one reader, here
//!
//! Every crate that takes a folder from the environment read it as
//! `PathBuf::from(value)` and refused only an UNSET variable. A variable set
//! but EMPTY is `PathBuf::from("")`, a path relative to the working directory,
//! so `BRUTEX_LOG_DIR=` wrote the event log into the cwd, `BRUTEX_STORE=` put
//! `bars/` and `manifest/` there, and an empty `HOME` turned every `$HOME/...`
//! default into a cwd-relative one (CE-5, CE-33, CE-36, CE-37, CE-38). And two
//! switches turned off only for a literal `0`, so `false`, `off` and `no` left
//! the operator's stated choice silently reversed (CE-6, CE-39).
//!
//! Each copy was fixed once, here, so the next reader cannot drift from it
//! (D-1769). The functions take the RAW value rather than reading the
//! environment, so the rule is testable without `set_var`, which is unsafe and
//! which the crates that call this forbid.

use std::ffi::OsString;
use std::path::PathBuf;

/// A folder named by `name`, or `None` when the variable is unset.
///
/// # Errors
///
/// The value is empty or only whitespace. That is a path relative to the
/// working directory, never a folder the operator meant, and it is refused by
/// the variable's name rather than resolved.
pub fn folder(name: &str, raw: Option<OsString>) -> Result<Option<PathBuf>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.to_string_lossy().trim().is_empty() {
        return Err(format!(
            "{name} is set but empty. An empty folder is the working directory, \
             which is never what an operator names; unset it to take the default \
             or give a folder"
        ));
    }
    Ok(Some(PathBuf::from(raw)))
}

/// The operator's home directory from `HOME`.
///
/// # Errors
///
/// `HOME` is unset, empty or relative. Every `$HOME/...` default is a place
/// this build writes or reads secrets from, and a relative `HOME` would make
/// it the working directory instead.
pub fn home(raw: Option<OsString>) -> Result<PathBuf, String> {
    let home = folder("HOME", raw)?.ok_or_else(|| "HOME is not set".to_owned())?;
    if home.is_relative() {
        return Err(format!(
            "HOME is {:?}, which is relative. Every default under it would be \
             read from the working directory instead",
            home.display()
        ));
    }
    Ok(home)
}

/// An on/off switch named by `name`, or `default` when the variable is unset.
///
/// The words are the ones `BRUTEX_VALIDATE` already took through the request
/// path: `1`, `true`, `on`, `yes` and `0`, `false`, `off`, `no`, in any case,
/// with surrounding space ignored.
///
/// # Errors
///
/// Any other value, refused by name. A typo must never silently keep a switch
/// the operator meant to flip.
pub fn switch(name: &str, raw: Option<&str>, default: bool) -> Result<bool, String> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        _ => Err(format!(
            "{name}={raw:?} is not a switch. Use 1, true, on or yes to turn it on, \
             and 0, false, off or no to turn it off"
        )),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_or_blank_folder_is_refused_by_name_and_unset_is_none() {
        assert_eq!(folder("BRUTEX_STORE", None), Ok(None));
        for blank in ["", " ", "\t\n"] {
            let why = folder("BRUTEX_STORE", Some(blank.into())).unwrap_err();
            assert!(why.starts_with("BRUTEX_STORE is set but empty"), "{why}");
        }
        assert_eq!(
            folder("BRUTEX_STORE", Some("/data/store".into())),
            Ok(Some(PathBuf::from("/data/store")))
        );
    }

    #[test]
    fn home_must_be_set_non_empty_and_absolute() {
        assert_eq!(home(None).unwrap_err(), "HOME is not set");
        assert!(
            home(Some("".into()))
                .unwrap_err()
                .contains("HOME is set but empty")
        );
        assert!(
            home(Some("rel/dir".into()))
                .unwrap_err()
                .contains("relative")
        );
        assert_eq!(
            home(Some("/Users/op".into())),
            Ok(PathBuf::from("/Users/op"))
        );
    }

    #[test]
    fn a_switch_takes_eight_words_and_refuses_the_rest() {
        assert_eq!(switch("K", None, true), Ok(true));
        assert_eq!(switch("K", None, false), Ok(false));
        for on in ["1", "true", "ON", " yes "] {
            assert_eq!(switch("K", Some(on), false), Ok(true), "{on}");
        }
        for off in ["0", "False", "off", "NO", " 0"] {
            assert_eq!(switch("K", Some(off), true), Ok(false), "{off}");
        }
        for odd in ["", "2", "nope", "o n"] {
            let why = switch("K", Some(odd), true).unwrap_err();
            assert!(
                why.starts_with(&format!("K={odd:?} is not a switch")),
                "{why}"
            );
        }
    }
}
