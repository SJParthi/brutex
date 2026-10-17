//! One way to open a file for reading, and one question every writer asks of
//! a store root before it writes (R3-16; rules audit RA-317 and RA-298).
//!
//! # Reading: [`open_read`]
//!
//! Every reader used to open its files its own way, and the ways differed in
//! exactly the cases that hurt: the verdict store's reader opened with
//! `O_NOFOLLOW` alone, so a pipe planted at a year index's name made every
//! lookup wait for a writer that never came (verify-IA-5-r2, follow-up 1);
//! the GDFL tick store's day reader followed a link, so a dangling one read
//! as "no such day" and the import ended with exit 0 (FM-046). One door now,
//! and its answer depends only on what is at the name:
//!
//! | what is at the name | answer |
//! |---|---|
//! | a regular file, with one name or several | the handle, and its length from the same `fstat` |
//! | a symbolic link, dangling or not | refused `read_not_regular`, found `symlink`: never followed |
//! | a pipe (FIFO) | refused, found `pipe`: the open returns at once (`O_NONBLOCK`) and the handle's `fstat` refuses it; nothing waits for a writer |
//! | a socket | refused, found `socket` (the host will not open one; one `lstat` names it) |
//! | a character or a block device | refused, found `char_device` or `block_device` |
//! | a directory | refused, found `directory` |
//! | nothing | the host's own `NotFound`, as `io` ([`ReadRefusal::is_missing`]) |
//!
//! The kind is asked of the HANDLE, so nothing can be swapped in between a
//! look and the open. A file with several names is readable: reading through
//! a second name changes no byte, so "exactly one name" is the staging rule
//! of a WRITER ([`crate::staging`]), not a reader's (RA-317).
//!
//! # What it does not do
//!
//! It is not an `openat` sandbox: `O_NOFOLLOW` refuses a link at the LAST
//! component only, and a link at a directory component on the way is
//! followed. `std` offers no per-component open without `unsafe`, which every
//! crate root forbids; [`crate::file`]'s writer walks the components with
//! `symlink_metadata` before it creates anything (CE-62, D-2686), and a link
//! swapped into a directory after that walk is the window FM-223 names. It
//! does not log: the caller names the refusal in its own event, as
//! [`crate::staging`] says of itself.
//!
//! # Writing: [`refuse_linked_store_root`]
//!
//! A test server's shadow store links `<root>/bars` to the real store's
//! (`state/fix/shadow.sh`), so anything that writes a month through that root
//! writes the REAL store, as a second writer the operator's `/ingest` never
//! shows (RA-298, FM-198). [`crate::file`]'s writer already refuses every link
//! below the root when it opens a month (CE-62); this is the same rule asked
//! ONCE, by name, before a writing verb does any work at all: a store whose
//! `bars` is a symbolic link is refused `linked_store_root`, naming the link
//! and where it points. A read is never refused by it: a shadow exists to be
//! read.
//!
//! # Cost
//!
//! [`open_read`]: two host calls on every answer that is a handle (`open` and
//! `fstat`), one metadata call more than a bare `open`, whatever the file
//! holds; a refusal of an open the host itself refused adds one `lstat` to
//! name what is there. [`refuse_linked_store_root`]: one `lstat`, plus one
//! `readlink` when it refuses. No read of the file's bytes, no listing, no
//! loop. Timed by C-SF-01 and C-SF-02 in `crates/store/benches/ratio.rs`
//! (p50, p99 and max at n = 10,000).

use std::fmt;
use std::fs::{self, File, FileType, Metadata};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

/// What was found at a name that is not a regular file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Found {
    /// A symbolic link, dangling or not.
    Symlink,
    /// A pipe (FIFO).
    Pipe,
    /// A UNIX socket.
    Socket,
    /// A character device.
    CharDevice,
    /// A block device.
    BlockDevice,
    /// A directory.
    Directory,
    /// None of the kinds above, and not a regular file.
    Unknown,
}

impl Found {
    /// What `kind` is, or `None` when it is a regular file.
    #[must_use]
    pub fn of(kind: FileType) -> Option<Self> {
        use std::os::unix::fs::FileTypeExt as _;
        if kind.is_file() {
            None
        } else if kind.is_symlink() {
            Some(Self::Symlink)
        } else if kind.is_dir() {
            Some(Self::Directory)
        } else if kind.is_fifo() {
            Some(Self::Pipe)
        } else if kind.is_socket() {
            Some(Self::Socket)
        } else if kind.is_char_device() {
            Some(Self::CharDevice)
        } else if kind.is_block_device() {
            Some(Self::BlockDevice)
        } else {
            Some(Self::Unknown)
        }
    }

    /// The kind's word, as a refusal names it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Pipe => "pipe",
            Self::Socket => "socket",
            Self::CharDevice => "char_device",
            Self::BlockDevice => "block_device",
            Self::Directory => "directory",
            Self::Unknown => "unknown",
        }
    }

    /// The kind in words an operator can act on.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::Symlink => "a symbolic link",
            Self::Pipe => "a pipe (FIFO)",
            Self::Socket => "a UNIX socket",
            Self::CharDevice => "a character device",
            Self::BlockDevice => "a block device",
            Self::Directory => "a directory",
            Self::Unknown => "a file of a kind this host does not name",
        }
    }
}

/// Every refusal of [`open_read`] and [`open_read_write`], each named.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadRefusal {
    /// `read_not_regular`: the name is not a regular file; `found` says what
    /// it is. Nothing was read through it and nothing waited on it.
    NotRegular {
        /// The name.
        path: PathBuf,
        /// What is there.
        found: Found,
    },
    /// `io`: the host refused an operation. An absent name is this, with the
    /// host's `NotFound` ([`Self::is_missing`]).
    Io {
        /// The name.
        path: PathBuf,
        /// What was being done.
        action: &'static str,
        /// The host's refusal.
        source: io::Error,
    },
}

impl ReadRefusal {
    /// The refusal's word.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::NotRegular { .. } => "read_not_regular",
            Self::Io { .. } => "io",
        }
    }

    /// The name the refusal is about.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::NotRegular { path, .. } | Self::Io { path, .. } => path,
        }
    }

    /// What was found at the name, when it is not a regular file.
    #[must_use]
    pub const fn found(&self) -> Option<Found> {
        match self {
            Self::NotRegular { found, .. } => Some(*found),
            Self::Io { .. } => None,
        }
    }

    /// Whether nothing has the name: the one refusal a reader may take for
    /// "not there". A dangling link is NOT missing: it is
    /// [`Found::Symlink`].
    #[must_use]
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Io { source, .. } if source.kind() == ErrorKind::NotFound)
    }
}

impl fmt::Display for ReadRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = self.word();
        match self {
            Self::NotRegular { path, found } => write!(
                f,
                "{word} ({}): {} is {}, not a regular file; a reader opens nothing else, so \
                 nothing was read through it and nothing waited on it",
                found.word(),
                path.display(),
                found.sentence()
            ),
            Self::Io {
                path,
                action,
                source,
            } => write!(f, "{word}: {action} {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for ReadRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::NotRegular { .. } => None,
        }
    }
}

/// The verified `O_NOFOLLOW | O_NONBLOCK` of this target, or `None` where
/// [`crate::open_flags`] has no verified value.
#[cfg(any(
    target_os = "macos",
    all(
        any(target_os = "linux", target_os = "android"),
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
const FLAGS: Option<i32> = Some(crate::open_flags::O_NOFOLLOW_NONBLOCK);

/// No verified flags on this target: every open is refused `Unsupported`.
#[cfg(not(any(
    target_os = "macos",
    all(
        any(target_os = "linux", target_os = "android"),
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
const FLAGS: Option<i32> = None;

/// Opens the existing regular file `path` for reading: never through a final
/// symbolic link, never waiting, never anything but a regular file (the
/// module's table).
///
/// # Errors
///
/// [`ReadRefusal::NotRegular`] naming what is there, and the host's refusal
/// as [`ReadRefusal::Io`] (an absent name among them).
pub fn open_read(path: &Path) -> Result<File, ReadRefusal> {
    open_read_len(path).map(|(file, _len)| file)
}

/// [`open_read`], with the file's length taken from the same `fstat` that
/// proved it a regular file, so a caller that needs both makes no second
/// metadata call.
///
/// # Errors
///
/// As [`open_read`].
pub fn open_read_len(path: &Path) -> Result<(File, u64), ReadRefusal> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    open_regular(&mut options, path)
}

/// Opens the EXISTING regular file `path` for reading and writing under the
/// same rules: never created, never through a final symbolic link, never
/// waiting, never anything but a regular file. For a file a writer updates in
/// place by positional writes (a year index of [`crate::verdict`]); a file a
/// writer creates is born by [`crate::staging::create`] instead.
///
/// # Errors
///
/// As [`open_read`].
pub fn open_read_write(path: &Path) -> Result<(File, u64), ReadRefusal> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true);
    open_regular(&mut options, path)
}

/// The one door of [`open_read_len`] and [`open_read_write`].
fn open_regular(options: &mut fs::OpenOptions, path: &Path) -> Result<(File, u64), ReadRefusal> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let io = |action: &'static str, source: io::Error| ReadRefusal::Io {
        path: path.to_path_buf(),
        action,
        source,
    };
    let Some(flags) = FLAGS else {
        return Err(io(
            "open",
            io::Error::new(
                ErrorKind::Unsupported,
                "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
            ),
        ));
    };
    let file = match options.custom_flags(flags).open(path) {
        Ok(file) => file,
        Err(why) if why.kind() == ErrorKind::NotFound => return Err(io("open", why)),
        Err(why) => return Err(name_the_refusal(path, why)),
    };
    let meta = file.metadata().map_err(|why| io("stat", why))?;
    match Found::of(meta.file_type()) {
        None => Ok((file, meta.len())),
        Some(found) => Err(ReadRefusal::NotRegular {
            path: path.to_path_buf(),
            found,
        }),
    }
}

/// An open the host refused, named by what is at the name when that is the
/// reason: `ELOOP` from `O_NOFOLLOW` at a final link, `ENXIO` or
/// `EOPNOTSUPP` at a socket, `EACCES` at a device, `EISDIR` at a directory
/// opened to write. One `lstat`, on this failure path only. Anything else
/// (a regular file the host will not open, a name the `lstat` cannot see)
/// keeps the host's own refusal.
///
/// Public so a door that must CREATE (a lock file, which
/// [`open_read_write`] never does) names its refusals in the same words.
///
/// Under `cfg(test)` each call is counted on its thread, so the cost the
/// first paragraph states (no look on any other path) is measured.
#[must_use]
pub fn name_the_refusal(path: &Path, why: io::Error) -> ReadRefusal {
    #[cfg(test)]
    tests::count_look();
    match fs::symlink_metadata(path)
        .ok()
        .as_ref()
        .map(Metadata::file_type)
        .and_then(Found::of)
    {
        Some(found) => ReadRefusal::NotRegular {
            path: path.to_path_buf(),
            found,
        },
        None => ReadRefusal::Io {
            path: path.to_path_buf(),
            action: "open",
            source: why,
        },
    }
}

/// The directory of a store root that holds its bar files.
pub const BARS_DIR: &str = "bars";

/// Every refusal of [`refuse_linked_store_root`], each named.
#[derive(Debug)]
#[non_exhaustive]
pub enum StoreRootRefusal {
    /// `linked_store_root`: the root's `bars` is a symbolic link, so its
    /// months are another store's.
    Linked {
        /// The store root.
        root: PathBuf,
        /// `<root>/bars`, the link.
        bars: PathBuf,
        /// Where the link points, as it is written, or `None` when it could
        /// not be read.
        target: Option<PathBuf>,
    },
    /// `store_bars_not_a_directory`: `<root>/bars` is there and is neither a
    /// directory nor a link.
    NotADirectory {
        /// `<root>/bars`.
        bars: PathBuf,
        /// What is there, in words: [`Found::sentence`], or "a regular file".
        what: &'static str,
    },
    /// `io`: the host refused to say what `<root>/bars` is.
    Io {
        /// `<root>/bars`.
        bars: PathBuf,
        /// The host's refusal.
        source: io::Error,
    },
}

impl StoreRootRefusal {
    /// The refusal's word.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Linked { .. } => "linked_store_root",
            Self::NotADirectory { .. } => "store_bars_not_a_directory",
            Self::Io { .. } => "io",
        }
    }
}

impl fmt::Display for StoreRootRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = self.word();
        match self {
            Self::Linked { root, bars, target } => write!(
                f,
                "{word}: {} is a symbolic link{}, so the bars of the store at {} are another \
                 store's; a writer here would write that store as a second writer nobody \
                 sees, and none does. Only a store whose bars directory is its own is \
                 written (RA-298)",
                bars.display(),
                target
                    .as_ref()
                    .map_or_else(String::new, |to| format!(" to {}", to.display())),
                root.display()
            ),
            Self::NotADirectory { bars, what } => write!(
                f,
                "{word}: {} is {what}, not the directory that holds the store's bars; \
                 nothing was written",
                bars.display()
            ),
            Self::Io { bars, source } => write!(
                f,
                "{word}: cannot tell what {} is ({source}), so nothing is written through it",
                bars.display()
            ),
        }
    }
}

impl std::error::Error for StoreRootRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Linked { .. } | Self::NotADirectory { .. } => None,
        }
    }
}

/// Refuses a store root a writer may not write: one whose `bars` is a
/// symbolic link (`linked_store_root`) or something other than a directory.
/// A root with no `bars` yet is a new store, and may be written: the writer
/// makes the directory.
///
/// Asked with `symlink_metadata`, which never follows. The root itself may be
/// a link: where an operator keeps the store is the operator's to place
/// ([`crate::file`] says the same of its walk).
///
/// # Errors
///
/// [`StoreRootRefusal::Linked`], [`StoreRootRefusal::NotADirectory`], and
/// [`StoreRootRefusal::Io`] when the host will not say.
pub fn refuse_linked_store_root(root: &Path) -> Result<(), StoreRootRefusal> {
    let bars = root.join(BARS_DIR);
    match fs::symlink_metadata(&bars) {
        Ok(meta) => match Found::of(meta.file_type()) {
            Some(Found::Directory) => Ok(()),
            Some(Found::Symlink) => Err(StoreRootRefusal::Linked {
                root: root.to_path_buf(),
                target: fs::read_link(&bars).ok(),
                bars,
            }),
            Some(found) => Err(StoreRootRefusal::NotADirectory {
                bars,
                what: found.sentence(),
            }),
            None => Err(StoreRootRefusal::NotADirectory {
                bars,
                what: "a regular file",
            }),
        },
        Err(why) if why.kind() == ErrorKind::NotFound => Ok(()),
        Err(source) => Err(StoreRootRefusal::Io { bars, source }),
    }
}

#[cfg(test)]
#[path = "safe_open_tests.rs"]
mod tests;
