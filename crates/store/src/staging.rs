//! A staging file: the temporary a writer fills beside a final name before it
//! links or renames it into place.
//!
//! # The hole this closes
//!
//! A writer that opens its temporary "create, truncate" opens whatever is
//! already at that name. If the name is a second hard link of another file
//! (planted, or left by a tool that links instead of copying), the truncate
//! cuts THAT file to zero and the write goes through it: the sort index's
//! second check lost a bar file exactly this way, and the rules audit of
//! 11 October 2026 found the same open in the verdict store (RA-290) and
//! expected it at every other temporary the ingest automation adds (RA-287).
//! `O_NOFOLLOW` does not help: a hard link is not a symbolic link.
//!
//! # The rule
//!
//! [`create`] opens with `O_CREAT | O_EXCL`, under which the kernel opens
//! nothing that already exists: not a regular file, not a second name of one,
//! not a link of either kind. When the name exists it is looked at without
//! following it:
//!
//! | what is at the name | answer |
//! |---|---|
//! | nothing | the new file |
//! | a regular file with exactly one name | a dead writer's leftover: the NAME is removed (`unlink`, which changes no byte) and the create is tried once more; [`Staged::cleared`] says how many bytes it held |
//! | a regular file with more than one name | refused `staging_linked`; nothing is removed, truncated or written |
//! | anything else (a symbolic link, a directory, a pipe) | refused `staging_not_regular`; left as it is |
//! | a file again after the one removal | refused `staging_contended`: another writer is using this name |
//!
//! No path here truncates, and the only removal is of a name whose file has
//! no other. A file swapped in between the look and the removal loses a name
//! at most, never a byte.
//!
//! # What it does not do
//!
//! It does not log: `store` proves every one of its emit sites through
//! `emits.rs`, and a helper called from several crates would hide the
//! caller's context. The caller reports [`Staged::cleared`] and every
//! [`StagingError`] in its own event. It does not make the bytes durable or
//! publish them: that is the caller's `sync_all` and its `hard_link` or
//! `rename`.
//!
//! Cost: one `open`; on a name that exists, at most one `lstat`, one
//! `unlink` and one more `open`. O(1).

use std::fmt;
use std::fs::{self, File};
use std::io::{self, ErrorKind};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

/// A staging file that [`create`] made, and what it had to clear to make it.
#[derive(Debug)]
pub struct Staged {
    /// The new, empty file, open for reading and writing.
    pub file: File,
    /// The bytes of a dead writer's leftover whose name was removed first,
    /// or `None` when the name was free. The caller reports it.
    pub cleared: Option<u64>,
}

/// Every refusal of [`create`], each named.
#[derive(Debug)]
#[non_exhaustive]
pub enum StagingError {
    /// `staging_linked`: the name is one of several names of a file. Writing
    /// through it would change the file behind the other names, so nothing
    /// is removed, truncated or written.
    Linked {
        /// The staging name.
        path: PathBuf,
        /// How many names the file has.
        links: u64,
    },
    /// `staging_not_regular`: the name is a symbolic link, a directory or a
    /// special file. Left as it is.
    NotRegular {
        /// The staging name.
        path: PathBuf,
    },
    /// `staging_contended`: the name existed again after its one removal:
    /// another writer is using it.
    Contended {
        /// The staging name.
        path: PathBuf,
    },
    /// `io`: the host refused an operation.
    Io {
        /// The staging name.
        path: PathBuf,
        /// What was being done.
        action: &'static str,
        /// The host's refusal.
        source: io::Error,
    },
}

impl StagingError {
    /// The refusal's word.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Linked { .. } => "staging_linked",
            Self::NotRegular { .. } => "staging_not_regular",
            Self::Contended { .. } => "staging_contended",
            Self::Io { .. } => "io",
        }
    }

    /// The staging name the refusal is about.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Linked { path, .. }
            | Self::NotRegular { path }
            | Self::Contended { path }
            | Self::Io { path, .. } => path,
        }
    }
}

impl fmt::Display for StagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = self.word();
        match self {
            Self::Linked { path, links } => write!(
                f,
                "{word}: {} is one of {links} names of a file; it is left as it is and nothing was written through it",
                path.display()
            ),
            Self::NotRegular { path } => write!(
                f,
                "{word}: {} is not a regular file; it is left as it is",
                path.display()
            ),
            Self::Contended { path } => write!(
                f,
                "{word}: {} existed again after it was cleared; another writer is using the name",
                path.display()
            ),
            Self::Io {
                path,
                action,
                source,
            } => write!(f, "{word}: {action} {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for StagingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// `O_CREAT | O_EXCL`, read and write: the new file, or the host's refusal.
fn open_new(path: &Path) -> io::Result<File> {
    File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
}

/// Creates the staging file `path`, never opening anything that is already
/// there (the module's table).
///
/// # Errors
///
/// [`StagingError::Linked`], [`StagingError::NotRegular`],
/// [`StagingError::Contended`], and the host's refusal as
/// [`StagingError::Io`].
pub fn create(path: &Path) -> Result<Staged, StagingError> {
    let io = |action: &'static str, source: io::Error| StagingError::Io {
        path: path.to_path_buf(),
        action,
        source,
    };
    match open_new(path) {
        Ok(file) => {
            return Ok(Staged {
                file,
                cleared: None,
            });
        }
        Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
        Err(why) => return Err(io("create", why)),
    }
    let cleared = match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() => {
            return Err(StagingError::NotRegular {
                path: path.to_path_buf(),
            });
        }
        Ok(meta) if meta.nlink() != 1 => {
            return Err(StagingError::Linked {
                path: path.to_path_buf(),
                links: meta.nlink(),
            });
        }
        Ok(meta) => {
            match fs::remove_file(path) {
                Err(why) if why.kind() != ErrorKind::NotFound => return Err(io("remove", why)),
                _ => {}
            }
            Some(meta.len())
        }
        // It went between the open and the look: the name is free now.
        Err(why) if why.kind() == ErrorKind::NotFound => None,
        Err(why) => return Err(io("stat", why)),
    };
    match open_new(path) {
        Ok(file) => Ok(Staged { file, cleared }),
        Err(why) if why.kind() == ErrorKind::AlreadyExists => Err(StagingError::Contended {
            path: path.to_path_buf(),
        }),
        Err(why) => Err(io("create", why)),
    }
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
