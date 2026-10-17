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
//! | nothing by the look, or a file with no name left (the look caught its last name going) | the name is free: the create is tried once more and nothing is cleared |
//! | anything else (a symbolic link, a directory, a pipe, a socket, a device) | refused `staging_not_regular`, naming what it found ([`crate::safe_open::Found`]); left as it is |
//! | a file again after the one removal | refused `staging_contended`: another writer is using this name |
//!
//! # What a caller must bring: a name of its own
//!
//! The second row cannot tell a DEAD writer's leftover from a LIVE writer's
//! file: any regular file with one name is cleared. So a staging name must
//! belong to one writer at a time, by the caller's own means: a name that
//! is unique to the writer (`store::verdict` puts its process id and run in
//! it), or a lock held across the create and the publish (`store::verdict`
//! holds the year's commit lock). Two live writers handed the same name
//! with neither would each unlink the other's file; this helper does not
//! and cannot prevent that (L-IA-44).
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
//! # Cost
//!
//! One host call (`open`) on a free name; four at most on a name that
//! exists (`open`, `lstat`, `unlink`, `open`), whatever the leftover holds
//! and however many names share its directory: no read, no listing, no
//! loop. [`Staged::calls`] is that count as each branch states it, written
//! by hand: the test
//! `store::staging::tests::a_create_makes_one_host_call_on_a_free_name_and_four_over_a_leftover`
//! pins which branch a create took (at a leftover of 0 bytes and of 4 MiB,
//! alone and beside 2,000 other names), NOT the host calls themselves
//! (verify-IA-5-r2, follow-up 2). The calls are measured instead: C-SF-03 in
//! `crates/store/benches/ratio.rs` times a create on a free name and over a
//! leftover of 0 bytes and of 4 MiB, alone and beside 2,000 names, p50, p99
//! and max at n = 10,000, and holds the cost flat in its directory (R3-16).
//! It is NOT flat in the leftover's size, and is not claimed to be: the
//! helper reads no byte of a leftover, but the host's `unlink` frees its
//! blocks, measured 22x to 30x the 0-byte p50 at 4 MiB on `WD_BLACK`
//! (`docs/06-limits.md` L-R316-06). What each call costs is the host's
//! (`docs/06-limits.md` L-IA-44).

use std::fmt;
use std::fs::{self, File};
use std::io::{self, ErrorKind};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use crate::safe_open::Found;

/// A staging file that [`create`] made, and what it had to clear to make it.
#[derive(Debug)]
pub struct Staged {
    /// The new, empty file, open for reading and writing.
    pub file: File,
    /// The bytes of a dead writer's leftover whose name was removed first,
    /// or `None` when the name was free. The caller reports it.
    pub cleared: Option<u64>,
    /// The host calls this create made: 1 on a free name, 4 over a
    /// leftover (`open`, `lstat`, `unlink`, `open`). Never more.
    pub calls: u8,
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
    /// special file; `found` says which. Left as it is.
    NotRegular {
        /// The staging name.
        path: PathBuf,
        /// What is there.
        found: Found,
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
            | Self::NotRegular { path, .. }
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
            Self::NotRegular { path, found } => write!(
                f,
                "{word} ({}): {} is {}, not a regular file; it is left as it is",
                found.word(),
                path.display(),
                found.sentence()
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
///
/// Under `cfg(test)` a one-shot, per-thread refusal of a chosen kind can be
/// armed for the Nth open of this thread, because no host refuses the
/// second create of [`create`] other than `EEXIST` short of a change to the
/// directory between two calls, and that arm must still be driven. The
/// injector ADDS a branch under test, so the open that ships is the open
/// every test runs.
fn open_new(path: &Path) -> io::Result<File> {
    #[cfg(test)]
    if let Some(kind) = tests::open_refusal_fires() {
        return Err(io::Error::new(
            kind,
            "open refused by the staging test seam",
        ));
    }
    File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
}

/// What one `lstat` of an existing staging name saw.
struct Seen {
    /// What is there when it is not a regular file; `None` for one.
    found: Option<Found>,
    /// How many names the file has. 0 when its last name went between the
    /// host's lookup of the name and its read of the file's status.
    links: u64,
    /// How many bytes it holds.
    len: u64,
}

/// One `lstat` of `path`, never following a final link.
///
/// Under `cfg(test)` the look can be answered by an armed stand-in on this
/// thread, once, because the two answers a race between [`create`]'s open
/// and this look produces (a refusal other than `ENOENT`, and a file whose
/// last name is going) cannot be staged on request.
fn look(path: &Path) -> io::Result<Seen> {
    #[cfg(test)]
    if let Some(stand_in) = tests::look_stand_in() {
        return stand_in(path);
    }
    let meta = fs::symlink_metadata(path)?;
    Ok(Seen {
        found: Found::of(meta.file_type()),
        links: meta.nlink(),
        len: meta.len(),
    })
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
                calls: 1,
            });
        }
        Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
        Err(why) => return Err(io("create", why)),
    }
    // Calls so far: the refused `open` and the look below.
    let mut calls = 2u8;
    let cleared = match look(path) {
        Ok(Seen {
            found: Some(found), ..
        }) => {
            return Err(StagingError::NotRegular {
                path: path.to_path_buf(),
                found,
            });
        }
        // Its last name went between the host's lookup and its status: the
        // look caught the file mid-unlink, and the name is free now. Not
        // `staging_linked`: a file with no name has none to protect.
        Ok(Seen { links: 0, .. }) => None,
        Ok(Seen { links, .. }) if links != 1 => {
            return Err(StagingError::Linked {
                path: path.to_path_buf(),
                links,
            });
        }
        Ok(Seen { len, .. }) => {
            calls += 1;
            match fs::remove_file(path) {
                Err(why) if why.kind() != ErrorKind::NotFound => return Err(io("remove", why)),
                _ => {}
            }
            Some(len)
        }
        // It went between the open and the look: the name is free now.
        Err(why) if why.kind() == ErrorKind::NotFound => None,
        Err(why) => return Err(io("stat", why)),
    };
    match open_new(path) {
        Ok(file) => Ok(Staged {
            file,
            cleared,
            calls: calls + 1,
        }),
        Err(why) if why.kind() == ErrorKind::AlreadyExists => Err(StagingError::Contended {
            path: path.to_path_buf(),
        }),
        Err(why) => Err(io("create", why)),
    }
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
