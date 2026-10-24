//! The copies register: every extra copy the operator names, with its bytes,
//! its reader and its proof; every second name of a live archive file flagged
//! red; and a retirement list the operator acts on by hand (CC-5, FM-607).
//!
//! # The failure this surfaces
//!
//! A third of the drive held copies nothing read and no page showed: a
//! renamed third copy of every options day zip, partial downloads, an
//! extracted CSV tree and plain-text proof lists. Seven partial-download
//! names were hard links of the live vendor year zips, so a downloader that
//! resumed into one would have changed a vendor original in place, and these
//! are the only originals. `/disk.json` counts the store, not the work
//! folders, so the space and the hazard were both invisible.
//!
//! # What it never does
//!
//! It never deletes, moves, renames, truncates, creates or writes a file.
//! Every touch of the disk goes through [`Fs`], whose three methods read:
//! `lstat`, a directory listing, and a bounded read of a regular file through
//! `store::safe_open` (never through a link, never waiting on a pipe). The
//! retirement list is TEXT for the operator, with no command in it:
//! [`RETIREMENT_SENTENCE`]. R4-06's no-delete scan lists this module
//! (`crates/pull/tests/no_scheduler_delete.rs`).
//!
//! # Where its roots come from (no typed path, no default)
//!
//! * The COPY roots: the setting `disk.extra_copy_roots`
//!   ([`store::settings::Key::ExtraCopyRoots`], read by
//!   [`store::settings::copy_roots`]). Unset is [`Wait::Unset`], a named wait
//!   ([`UNSET_WAIT`]): nothing is walked and nothing is answered as if it
//!   were measured (D-7040).
//! * The LIVE roots ([`LiveKind`]): the CM archive, the options archives and
//!   the tick store through the one rule every `BRUTEX_GDFL_*` root is read
//!   by ([`crate::gdfl_run::source_root`]); the `_v3` ledger
//!   ([`crate::gdfl_v3::V3_LEDGER_VAR`]); the store root its caller serves;
//!   and the settings `source.gdfl` and `source.masters`. One unset is named
//!   in [`Config::unset_live`], never guessed (D-7041).
//!
//! # One walk, and what it compares
//!
//! Every root is walked as itself only: a directory that IS another
//! configured root (by its `(st_dev, st_ino)`, so a second spelling of the
//! same folder is still recognised) is skipped where it is met and named.
//! Links are counted and never followed, so a link loop cannot loop. The
//! walked live roots are the ones whose files a writer could change in place
//! ([`LiveKind::walked`]). A HAZARD is a name under a copy root whose file
//! shares `(st_dev, st_ino)` with a name under a walked live root, compared
//! within one walk only: an inode number is unique per volume, and a
//! volume's `st_dev` is not stable across mounts, so neither is ever
//! compared across walks or alone (D-7042, amendment (e)).
//!
//! # Bounded, refused by name past the bound
//!
//! Each root's walk stops at [`MAX_WALK_ENTRIES`] directory entries or a
//! directory deeper than [`MAX_DEPTH`], and the root is then answered
//! REFUSED by name with no figure, never as a silent partial. A subfolder
//! that cannot be listed, or a name that vanishes mid-walk, is a named fault
//! of the root, which is then not walked whole and is kept off the
//! retirement list (D-7043). The walk is O(entries under the roots): it is
//! NOT O(1) and is named in `docs/06-limits.md` L-CC5-1.
//!
//! # Served from memory, re-walked only when a root's own stat changes
//!
//! [`answer`] walks on the first request after start and on a request that
//! finds a root's own stat, or the configured roots, changed; every other
//! request is a fixed number of `lstat`s, O(roots), and a clone of the held
//! answer. A root's own stat changes only when its DIRECT entries change, so
//! a link made deeper in the tree is not seen until the next walk: every
//! answer carries when it was walked, how old the walk is and which roots it
//! walked ([`Served`]), and `cli disk-copies` always walks afresh (D-7046,
//! amendment (f)). Requests that arrive while a walk runs never wait on it:
//! they are served the held answer marked as being re-walked, or, when none
//! is held, [`Served::Walking`].
//!
//! # Reader and proof
//!
//! The reader rule and the proof rule are [`reader_of`] and [`read_receipt`]
//! (D-7044, D-7045); the retirement list is [`retirement`] and
//! [`retirement_text`] (D-7047).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Write as _};
use std::io::{self, Read as _};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

/// The telemetry target of every event this module puts.
pub const TARGET: &str = "pull.copies";

/// The setting's name, as a file and the wire spell it.
pub const KEY: &str = "disk.extra_copy_roots";

/// What an unset `disk.extra_copy_roots` answers: a named wait, never a
/// default (D-7040). The settings route serves the same sentence as the
/// key's source.
pub const UNSET_WAIT: &str = "not set: no copy root is named until one is saved (POST /settings \
                              disk.extra_copy_roots); a named wait, never a default, so nothing \
                              is walked";

/// The fixed sentence of every retirement list (D-7047).
pub const RETIREMENT_SENTENCE: &str = "the operator removes these by hand; the app never does";

/// The most directory entries one root's walk visits before it stops and
/// the root is refused by name (D-7043). A resource bound, not a data
/// threshold: about 24 times the largest copy root measured on 13 Oct
/// (82,073 files), and one walk of it is one `lstat` per entry.
pub const MAX_WALK_ENTRIES: u64 = 2_000_000;

/// The deepest directory one root's walk descends to, counted from the root
/// as 0 (D-7043).
pub const MAX_DEPTH: usize = 64;

/// The most rows of one list an answer carries (hazards, faults, receipts,
/// second names elsewhere): the first by path, with the whole count beside
/// them, so a list is never cut silently (D-7043).
pub const MAX_LISTED: usize = 64;

/// The most receipts one root has read; the rest are counted, not read
/// (D-7045).
pub const MAX_RECEIPTS_READ: usize = 64;

/// The largest receipt read whole; a larger one is named `too_big` and is
/// no proof (D-7045). About 16 times a `VERIFIED.tsv` of every options day
/// zip of a year at a few hundred bytes a row.
pub const MAX_RECEIPT_BYTES: u64 = 16 * 1_024 * 1_024;

/// The longest statement kept from a receipt's first line, in characters.
pub const MAX_STATEMENT_CHARS: usize = 200;

/// The tops of the tick store its readers open: `gdfl_tickstore`'s
/// `DayTable::walk(&self.root, "cm", ..)` and `gdfl_nfo`'s
/// `DayTable::walk(&self.root, "options", ..)`. A folder elsewhere under
/// the tick store root is read by nothing that names it (D-7044).
/// `tick_store_tops_are_the_readers_own` binds this list to those two
/// lines, so a reader that grows a top fails the build here.
pub const TICK_STORE_READ_TOPS: [&str; 2] = ["cm", "options"];

/// The folder of the `_v3` ledger under each year of its root
/// (`<ROOT>/<yyyy>/_v3/`, [`crate::gdfl_v3`]).
pub const V3_FOLDER: &str = "_v3";

// ═══ The filesystem seam ═══════════════════════════════════════════════════

/// What is at a name, as one `lstat` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A directory.
    Dir,
    /// A regular file.
    File,
    /// A symbolic link: counted, never followed.
    Link,
    /// A pipe, a socket or a device: counted, never opened.
    Other,
}

/// One `lstat`'s answer: the fields the register compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta {
    /// What is at the name.
    pub kind: Kind,
    /// `st_dev`.
    pub dev: u64,
    /// `st_ino`.
    pub ino: u64,
    /// `st_nlink`.
    pub nlink: u64,
    /// `st_size`, bytes.
    pub size: u64,
    /// `st_mtime`, seconds.
    pub mtime_s: i64,
    /// `st_mtime_nsec`.
    pub mtime_ns: i64,
    /// `st_ctime`, seconds.
    pub ctime_s: i64,
    /// `st_ctime_nsec`.
    pub ctime_ns: i64,
}

impl Meta {
    /// The file's identity within one walk: `(st_dev, st_ino)`.
    #[must_use]
    pub const fn id(&self) -> (u64, u64) {
        (self.dev, self.ino)
    }

    /// The root's own stat, as the cache compares it (D-7046).
    #[must_use]
    pub const fn stamp(&self) -> Stamp {
        Stamp {
            dev: self.dev,
            ino: self.ino,
            mtime_s: self.mtime_s,
            mtime_ns: self.mtime_ns,
            ctime_s: self.ctime_s,
            ctime_ns: self.ctime_ns,
        }
    }
}

/// What one bounded read of a regular file answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bytes {
    /// The whole file.
    Whole(Vec<u8>),
    /// The file is longer than the bound and was not read.
    TooBig(u64),
    /// It could not be read: not a regular file, gone, or the host refused.
    Refused(String),
}

/// Every way the register touches the disk, and nothing that writes: a
/// method that could delete, move, rename, create or write does not exist
/// here, so no caller of [`walk`] can reach one (D-7047).
pub trait Fs: Sync {
    /// `lstat(path)`: never follows a final link.
    ///
    /// # Errors
    ///
    /// The host's refusal.
    fn lstat(&self, path: &Path) -> io::Result<Meta>;

    /// Calls `each` with every name in `dir` until it answers `false`.
    ///
    /// # Errors
    ///
    /// The host's refusal to list `dir`, or to read one of its entries.
    fn list(&self, dir: &Path, each: &mut dyn FnMut(OsString) -> bool) -> io::Result<()>;

    /// The bytes of the regular file `path`, at most `max` of them.
    fn read(&self, path: &Path, max: u64) -> Bytes;
}

/// The host's filesystem, read only.
#[derive(Debug, Clone, Copy, Default)]
pub struct Host;

impl Fs for Host {
    fn lstat(&self, path: &Path) -> io::Result<Meta> {
        use std::os::unix::fs::MetadataExt as _;
        let meta = std::fs::symlink_metadata(path)?;
        let kind = meta.file_type();
        Ok(Meta {
            kind: if kind.is_dir() {
                Kind::Dir
            } else if kind.is_file() {
                Kind::File
            } else if kind.is_symlink() {
                Kind::Link
            } else {
                Kind::Other
            },
            dev: meta.dev(),
            ino: meta.ino(),
            nlink: meta.nlink(),
            size: meta.size(),
            mtime_s: meta.mtime(),
            mtime_ns: meta.mtime_nsec(),
            ctime_s: meta.ctime(),
            ctime_ns: meta.ctime_nsec(),
        })
    }

    fn list(&self, dir: &Path, each: &mut dyn FnMut(OsString) -> bool) -> io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            if !each(entry?.file_name()) {
                break;
            }
        }
        Ok(())
    }

    fn read(&self, path: &Path, max: u64) -> Bytes {
        let (file, len) = match store::safe_open::open_read_len(path) {
            Ok(opened) => opened,
            Err(refusal) => return Bytes::Refused(refusal.to_string()),
        };
        if len > max {
            return Bytes::TooBig(len);
        }
        let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
        match file.take(max.saturating_add(1)).read_to_end(&mut bytes) {
            Ok(_) if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max => {
                Bytes::TooBig(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            }
            Ok(_) => Bytes::Whole(bytes),
            Err(why) => Bytes::Refused(why.to_string()),
        }
    }
}

// ═══ The configuration ═════════════════════════════════════════════════════

/// The kinds of LIVE root: the sources the operator's data is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiveKind {
    /// The GDFL capital-market day zips (`BRUTEX_GDFL_CM_ARCHIVE`).
    CmArchive,
    /// The GDFL options year zips (`BRUTEX_GDFL_NFO_ARCHIVES`).
    NfoArchives,
    /// Where GDFL files are delivered (the setting `source.gdfl`).
    SourceGdfl,
    /// The verified tick store (`BRUTEX_GDFL_TICKSTORE`).
    TickStore,
    /// The `_v3` extraction ledger (`BRUTEX_GDFL_V3_LEDGER`).
    V3Ledger,
    /// The exchange masters (the setting `source.masters`, else
    /// `BRUTEX_MASTERS`).
    SourceMasters,
    /// The bar store the caller serves.
    Store,
}

impl LiveKind {
    /// Every kind, in the order an answer lists them.
    pub const ALL: [Self; 7] = [
        Self::CmArchive,
        Self::NfoArchives,
        Self::SourceGdfl,
        Self::TickStore,
        Self::V3Ledger,
        Self::SourceMasters,
        Self::Store,
    ];

    /// The kind's word on the wire.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::CmArchive => "cm_archive",
            Self::NfoArchives => "nfo_archives",
            Self::SourceGdfl => "source_gdfl",
            Self::TickStore => "tick_store",
            Self::V3Ledger => "v3_ledger",
            Self::SourceMasters => "source_masters",
            Self::Store => "store",
        }
    }

    /// Whether its files are walked for second names: the vendor archives
    /// and the tick store, whose files a resumed write would change in place
    /// and which a walk can afford. The ledger, the masters and the store are
    /// not walked; a second name of one of their files is reported as a
    /// second name outside every walked root, never as safe (D-7042).
    #[must_use]
    pub const fn walked(self) -> bool {
        matches!(
            self,
            Self::CmArchive | Self::NfoArchives | Self::SourceGdfl | Self::TickStore
        )
    }

    /// Whether this kind's reader reads the folder `rel` below its root
    /// (D-7044). The archives, the delivery folder, the masters and the store
    /// are read as whole trees: a folder inside one is never offered for
    /// retirement. The tick store's readers open only
    /// [`TICK_STORE_READ_TOPS`]; the ledger's opens `<yyyy>/_v3/` under its
    /// root (a `<yyyy>` folder holds one).
    #[must_use]
    pub fn reads(self, rel: &Path) -> bool {
        let mut parts = rel.components().filter_map(|part| match part {
            Component::Normal(name) => Some(name),
            _ => None,
        });
        match self {
            Self::TickStore => parts
                .next()
                .is_some_and(|top| TICK_STORE_READ_TOPS.iter().any(|t| OsStr::new(t) == top)),
            Self::V3Ledger => {
                let year = parts.next().is_some_and(|year| {
                    year.len() == 4 && year.as_encoded_bytes().iter().all(u8::is_ascii_digit)
                });
                year && parts.next().is_none_or(|folder| folder == OsStr::new(V3_FOLDER))
            }
            Self::CmArchive
            | Self::NfoArchives
            | Self::SourceGdfl
            | Self::SourceMasters
            | Self::Store => true,
        }
    }
}

/// One configured live root.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LiveRoot {
    /// Its kind.
    pub kind: LiveKind,
    /// The folder.
    pub path: PathBuf,
    /// What names it: a variable, a setting or the caller's store root.
    pub named_by: String,
}

/// The roots one walk reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Config {
    /// The copy roots, as the setting orders them.
    pub copy_roots: Vec<PathBuf>,
    /// The live roots that are configured.
    pub live: Vec<LiveRoot>,
    /// The live roots that are not, each named with why.
    pub unset_live: Vec<String>,
}

/// Why nothing was walked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wait {
    /// `disk.extra_copy_roots` is unset ([`UNSET_WAIT`]).
    Unset,
    /// Its stored value does not read (a hand-edited generation).
    Invalid(String),
}

impl fmt::Display for Wait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unset => write!(f, "{KEY} is {UNSET_WAIT}"),
            Self::Invalid(why) => write!(
                f,
                "{KEY} does not read ({why}); save it again: nothing is walked meanwhile"
            ),
        }
    }
}

/// What the caller knows of its own configuration: the stored value of the
/// copy-roots setting, the two source settings, the store root it serves,
/// and the process environment.
pub struct Sources<'a> {
    /// `disk.extra_copy_roots` as the newest generation holds it.
    pub copy_roots: Option<&'a str>,
    /// `source.gdfl` as the newest generation holds it.
    pub source_gdfl: Option<&'a str>,
    /// `source.masters` as the newest generation holds it.
    pub source_masters: Option<&'a str>,
    /// The bar store the caller serves.
    pub store_root: &'a Path,
    /// The environment.
    pub env: &'a dyn Fn(&str) -> Option<OsString>,
}

/// The roots `sources` name (D-7040, D-7041), the live ones in
/// [`LiveKind::ALL`]'s order.
///
/// # Errors
///
/// [`Wait`] when the copy roots are unset or do not read.
pub fn config(sources: &Sources<'_>) -> Result<Config, Wait> {
    let value = sources.copy_roots.ok_or(Wait::Unset)?;
    let copy_roots = store::settings::copy_roots(value).map_err(Wait::Invalid)?;
    let mut live = Vec::with_capacity(LiveKind::ALL.len());
    let mut unset_live = Vec::with_capacity(LiveKind::ALL.len());
    for kind in LiveKind::ALL {
        let named = match kind {
            LiveKind::CmArchive => variable(sources, crate::gdfl_run::CM_ARCHIVE_VAR),
            LiveKind::NfoArchives => variable(sources, crate::gdfl_run::NFO_ARCHIVES_VAR),
            LiveKind::TickStore => variable(sources, crate::gdfl_run::TICKSTORE_VAR),
            LiveKind::V3Ledger => variable(sources, crate::gdfl_v3::V3_LEDGER_VAR),
            LiveKind::SourceGdfl => setting("source.gdfl", sources.source_gdfl, None),
            LiveKind::SourceMasters => setting(
                "source.masters",
                sources.source_masters,
                store::settings::Key::SourceMasters
                    .env()
                    .and_then(|var| (sources.env)(var).map(|value| (var, value))),
            ),
            LiveKind::Store => Ok((
                sources.store_root.to_path_buf(),
                "the store root this process serves".to_owned(),
            )),
        };
        match named {
            Ok((path, named_by)) => live.push(LiveRoot {
                kind,
                path,
                named_by,
            }),
            Err(why) => unset_live.push(format!("{}: {why}", kind.word())),
        }
    }
    Ok(Config {
        copy_roots,
        live,
        unset_live,
    })
}

/// The root a `BRUTEX_GDFL_*` variable names, through the one rule every such
/// root is read by: unset or empty is named, never defaulted.
fn variable(sources: &Sources<'_>, var: &'static str) -> Result<(PathBuf, String), String> {
    crate::gdfl_run::source_root(var, sources.env)
        .map(|path| (path, var.to_owned()))
        .map_err(|why| why.to_string())
}

/// The folder a setting names, else its environment fallback, else why not.
fn setting(
    name: &'static str,
    saved: Option<&str>,
    fallback: Option<(&'static str, OsString)>,
) -> Result<(PathBuf, String), String> {
    match (saved, fallback) {
        (Some(saved), _) => Ok((PathBuf::from(saved), name.to_owned())),
        (None, Some((var, value))) if !value.is_empty() => {
            Ok((PathBuf::from(value), format!("{var} ({name} is not set)")))
        }
        _ => Err(format!("{name} is not set, so nothing is compared with it")),
    }
}

/// What reads a copy root, when something does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reader {
    /// The live root's kind.
    pub kind: LiveKind,
    /// The live root.
    pub path: PathBuf,
    /// What names it.
    pub named_by: String,
    /// How it reads the copy root, in words.
    pub how: &'static str,
}

/// The copy root `copy`'s reader under `live`, and, when none reads it but
/// it lies inside a live root, that root (D-7044):
///
/// 1. a copy root that IS a live root, or holds one, has that root's reader:
///    removing it would remove the live root;
/// 2. one inside a live root, in a part that root's reader reads
///    ([`LiveKind::reads`]), has that reader;
/// 3. one inside a live root elsewhere has no reader by name, and the live
///    root is returned so the retirement list can say where it lies.
///
/// Comparison is by path components, never by inode: the configured paths
/// are the names a reader opens. At most [`LiveKind::ALL`] comparisons.
#[must_use]
pub fn reader_of(copy: &Path, live: &[LiveRoot]) -> (Option<Reader>, Option<LiveRoot>) {
    let mut inside = None;
    for root in live {
        if root.path.starts_with(copy) {
            return (
                Some(Reader {
                    kind: root.kind,
                    path: root.path.clone(),
                    named_by: root.named_by.clone(),
                    how: if root.path.as_path() == copy {
                        "the copy root IS this live root"
                    } else {
                        "the copy root holds this live root"
                    },
                }),
                None,
            );
        }
        if let Ok(rel) = copy.strip_prefix(&root.path) {
            if root.kind.reads(rel) {
                return (
                    Some(Reader {
                        kind: root.kind,
                        path: root.path.clone(),
                        named_by: root.named_by.clone(),
                        how: "the copy root lies in a part of this live root its reader reads",
                    }),
                    None,
                );
            }
            inside.get_or_insert_with(|| root.clone());
        }
    }
    (None, inside)
}

// ═══ The walk ══════════════════════════════════════════════════════════════

/// A root's own stat, as the cache compares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stamp {
    /// `st_dev`.
    pub dev: u64,
    /// `st_ino`.
    pub ino: u64,
    /// `st_mtime`.
    pub mtime_s: i64,
    /// `st_mtime_nsec`.
    pub mtime_ns: i64,
    /// `st_ctime`.
    pub ctime_s: i64,
    /// `st_ctime_nsec`.
    pub ctime_ns: i64,
}

/// What one root's `lstat` said before the walk.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RootStat {
    /// A directory, with its own stat.
    Dir(Stamp),
    /// Anything else, named.
    Not(String),
}

/// The own stat of `path`, as a root.
#[must_use]
pub fn root_stat(fs: &dyn Fs, path: &Path) -> RootStat {
    match fs.lstat(path) {
        Ok(meta) if meta.kind == Kind::Dir => RootStat::Dir(meta.stamp()),
        Ok(meta) => RootStat::Not(match meta.kind {
            Kind::Link => {
                "a symbolic link: never followed; name the folder it points to".to_owned()
            }
            _ => "not a directory".to_owned(),
        }),
        Err(why) if why.kind() == io::ErrorKind::NotFound => {
            RootStat::Not("absent: no such folder".to_owned())
        }
        Err(why) => RootStat::Not(format!("cannot be read: {why}")),
    }
}

/// Every configured root's own stat, copy roots first: the cache's key with
/// the configuration. O(roots): one `lstat` each.
#[must_use]
pub fn stamps(fs: &dyn Fs, config: &Config) -> Vec<RootStat> {
    config
        .copy_roots
        .iter()
        .chain(config.live.iter().map(|root| &root.path))
        .map(|path| root_stat(fs, path))
        .collect()
}

/// The first [`MAX_LISTED`] items by order, and the whole count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed<T: Ord> {
    /// The first items.
    pub first: BTreeSet<T>,
    /// How many were offered.
    pub count: u64,
}

impl<T: Ord> Default for Listed<T> {
    fn default() -> Self {
        Self {
            first: BTreeSet::new(),
            count: 0,
        }
    }
}

impl<T: Ord> Listed<T> {
    /// Offer one: O(log [`MAX_LISTED`]).
    pub fn offer(&mut self, item: T) {
        self.count = self.count.saturating_add(1);
        self.first.insert(item);
        if self.first.len() > MAX_LISTED {
            self.first.pop_last();
        }
    }
}

/// One regular file the walk met, as the merge needs it.
#[derive(Debug, Clone)]
struct Named {
    path: PathBuf,
    meta: Meta,
}

/// One root's walk, before the roots are compared.
#[derive(Debug, Default)]
struct Walked {
    entries: u64,
    files: u64,
    dirs: u64,
    links: u64,
    others: u64,
    vanished: u64,
    newest_mtime_s: Option<i64>,
    /// Bytes of the files with one name.
    single_bytes: u64,
    /// Every file with more than one name: its names in this root.
    shared: HashMap<(u64, u64), Vec<Named>>,
    /// Receipt names, the first by path.
    receipts: Listed<PathBuf>,
    /// Faults, the first by path: `(path, why)`.
    faults: Listed<(PathBuf, String)>,
    /// Other roots met inside this one and not walked here, in order.
    skipped: BTreeSet<PathBuf>,
    /// For a live archive: every file, by `(ino, size)`, for receipts.
    identities: Vec<(PathBuf, Meta)>,
}

/// Walks `root` (D-7043): `Err` names why the root is refused, never a
/// partial count.
fn walk_root(
    fs: &dyn Fs,
    root: &Path,
    skip: &HashSet<(u64, u64)>,
    keep_identities: bool,
) -> Result<Walked, String> {
    let top = fs
        .lstat(root)
        .map_err(|why| format!("cannot be read: {why}"))?;
    if top.kind != Kind::Dir {
        return Err(match top.kind {
            Kind::Link => "a symbolic link: never followed; name the folder it points to".to_owned(),
            _ => "not a directory".to_owned(),
        });
    }
    let mut walked = Walked::default();
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        walked.dirs = walked.dirs.saturating_add(1);
        let mut names: Vec<OsString> = Vec::new();
        let mut over = false;
        let listed = fs.list(&dir, &mut |name| {
            walked.entries = walked.entries.saturating_add(1);
            if walked.entries > MAX_WALK_ENTRIES {
                over = true;
                return false;
            }
            names.push(name);
            true
        });
        if over {
            return Err(format!(
                "refused: more than {MAX_WALK_ENTRIES} entries (the walk's bound, D-7043); \
                 nothing of this root is counted. Name a smaller folder"
            ));
        }
        if let Err(why) = listed {
            if dir.as_path() == root {
                return Err(format!("cannot be listed: {why}"));
            }
            walked
                .faults
                .offer((dir.clone(), format!("a folder that cannot be listed: {why}")));
            continue;
        }
        for name in names {
            let path = dir.join(&name);
            let meta = match fs.lstat(&path) {
                Ok(meta) => meta,
                Err(why) if why.kind() == io::ErrorKind::NotFound => {
                    walked.vanished = walked.vanished.saturating_add(1);
                    walked
                        .faults
                        .offer((path, "vanished while the walk ran".to_owned()));
                    continue;
                }
                Err(why) => {
                    walked
                        .faults
                        .offer((path, format!("cannot be read: {why}")));
                    continue;
                }
            };
            match meta.kind {
                Kind::Dir if skip.get(&meta.id()).is_some() => {
                    walked.skipped.insert(path);
                }
                Kind::Dir if depth + 1 > MAX_DEPTH => {
                    return Err(format!(
                        "refused: a folder deeper than {MAX_DEPTH} levels ({}) (the walk's \
                         bound, D-7043); nothing of this root is counted",
                        quoted(&path)
                    ));
                }
                Kind::Dir => stack.push((path, depth + 1)),
                Kind::File => file_met(&mut walked, path, meta, &name, keep_identities),
                Kind::Link => walked.links = walked.links.saturating_add(1),
                Kind::Other => walked.others = walked.others.saturating_add(1),
            }
        }
    }
    walked.dirs = walked.dirs.saturating_sub(1);
    Ok(walked)
}

/// Files one regular file into `walked`.
fn file_met(walked: &mut Walked, path: PathBuf, meta: Meta, name: &OsStr, keep: bool) {
    walked.files = walked.files.saturating_add(1);
    walked.newest_mtime_s = Some(
        walked
            .newest_mtime_s
            .map_or(meta.mtime_s, |newest| newest.max(meta.mtime_s)),
    );
    if receipt_kind(name).is_some() {
        walked.receipts.offer(path.clone());
    }
    if keep {
        walked.identities.push((path.clone(), meta));
    }
    if meta.nlink > 1 {
        walked
            .shared
            .entry(meta.id())
            .or_default()
            .push(Named { path, meta });
    } else {
        walked.single_bytes = walked.single_bytes.saturating_add(meta.size);
    }
}

// ═══ Receipts and proof ════════════════════════════════════════════════════

/// The receipt shapes the register reads (D-7045, the 13 Oct receipts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReceiptKind {
    /// `ALL_VERIFIED.txt`: a run's statement that every configured archive
    /// passed its checks.
    AllVerified,
    /// `VERIFIED.tsv`: one row per item proved, with a `source_identity`
    /// column naming the source file.
    VerifiedTsv,
    /// `<stem>.prove.out`: a proof run's output, which must say
    /// `ALL-PROVED`.
    ProveOut,
}

impl ReceiptKind {
    /// The kind's word on the wire.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::AllVerified => "all_verified",
            Self::VerifiedTsv => "verified_tsv",
            Self::ProveOut => "prove_out",
        }
    }
}

/// The receipt kind a file name spells, or `None`.
#[must_use]
pub fn receipt_kind(name: &OsStr) -> Option<ReceiptKind> {
    let name = name.to_str()?;
    match name {
        "ALL_VERIFIED.txt" => Some(ReceiptKind::AllVerified),
        "VERIFIED.tsv" => Some(ReceiptKind::VerifiedTsv),
        _ if name.len() > ".prove.out".len() && name.ends_with(".prove.out") => {
            Some(ReceiptKind::ProveOut)
        }
        _ => None,
    }
}

/// What a receipt was found to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It states what it proves.
    States,
    /// Zero bytes, or only white space: it states nothing.
    Empty,
    /// It does not end in a line break: a write cut short.
    Torn,
    /// It is whole and says no proof (`.prove.out` without `ALL-PROVED`).
    NoClaim,
    /// Longer than [`MAX_RECEIPT_BYTES`]: not read.
    TooBig(u64),
    /// It could not be read, or is not UTF-8 text.
    Unreadable(String),
}

impl Verdict {
    /// The verdict's word on the wire.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::States => "states",
            Self::Empty => "empty",
            Self::Torn => "torn",
            Self::NoClaim => "no_claim",
            Self::TooBig(_) => "too_big",
            Self::Unreadable(_) => "unreadable",
        }
    }
}

/// One `source_identity` of a `VERIFIED.tsv` row:
/// `dev:inode:size:mtime_s:mtime_ns:ctime_s:ctime_ns`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceIdentity {
    /// The `st_dev` recorded: NEVER compared, since a remount changes it.
    pub dev: u64,
    /// `st_ino`.
    pub ino: u64,
    /// `st_size`.
    pub size: u64,
    /// `st_mtime`.
    pub mtime_s: i64,
    /// `st_mtime_nsec`.
    pub mtime_ns: i64,
    /// `st_ctime`.
    pub ctime_s: i64,
    /// `st_ctime_nsec`.
    pub ctime_ns: i64,
}

impl SourceIdentity {
    /// The identity `text` spells, or `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut fields = text.trim().split(':');
        let mut next = || fields.next();
        let identity = Self {
            dev: next()?.parse().ok()?,
            ino: next()?.parse().ok()?,
            size: next()?.parse().ok()?,
            mtime_s: next()?.parse().ok()?,
            mtime_ns: next()?.parse().ok()?,
            ctime_s: next()?.parse().ok()?,
            ctime_ns: next()?.parse().ok()?,
        };
        next().is_none().then_some(identity)
    }

    /// Whether `meta` is the file this identity recorded: inode, size and
    /// both times (D-7045). `st_dev` is not compared: the same file read
    /// after a remount has another one (the 13 Oct re-measure).
    #[must_use]
    pub const fn matches(&self, meta: &Meta) -> bool {
        self.ino == meta.ino
            && self.size == meta.size
            && self.mtime_s == meta.mtime_s
            && self.mtime_ns == meta.mtime_ns
            && self.ctime_s == meta.ctime_s
            && self.ctime_ns == meta.ctime_ns
    }
}

/// One receipt, read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Receipt {
    /// Where it is.
    pub path: PathBuf,
    /// Its shape.
    pub kind: ReceiptKind,
    /// What it was found to say: the verdict's word.
    pub verdict: &'static str,
    /// Why, for a verdict that is not `states`.
    pub why: Option<String>,
    /// Its statement: the first line, at most [`MAX_STATEMENT_CHARS`].
    pub statement: String,
    /// For `VERIFIED.tsv`: rows naming a source.
    pub sources: u64,
    /// Of those, rows whose source matches a walked live archive file today.
    pub matched: u64,
    /// The first row that does not match, in words.
    pub first_unmatched: Option<String>,
}

/// Reads one receipt (D-7045). A receipt states what it proves when it is
/// UTF-8 text ending in a line break whose first non-empty line is its
/// statement; a `.prove.out` must also hold the line `ALL-PROVED`. Each
/// `VERIFIED.tsv` row's `source_identity` is matched against `archives`, the
/// walked live archive files, by [`SourceIdentity::matches`].
#[must_use]
pub fn read_receipt(
    fs: &dyn Fs,
    path: &Path,
    kind: ReceiptKind,
    archives: &Archives,
) -> Receipt {
    let mut receipt = Receipt {
        path: path.to_path_buf(),
        kind,
        verdict: "states",
        why: None,
        statement: String::new(),
        sources: 0,
        matched: 0,
        first_unmatched: None,
    };
    let verdict = match fs.read(path, MAX_RECEIPT_BYTES) {
        Bytes::TooBig(len) => Verdict::TooBig(len),
        Bytes::Refused(why) => Verdict::Unreadable(why),
        Bytes::Whole(bytes) => match String::from_utf8(bytes) {
            Err(_) => Verdict::Unreadable("not UTF-8 text".to_owned()),
            Ok(text) => judge(&text, kind, archives, &mut receipt),
        },
    };
    receipt.verdict = verdict.word();
    receipt.why = match verdict {
        Verdict::States => None,
        Verdict::Empty => Some("it states nothing".to_owned()),
        Verdict::Torn => Some("it does not end in a line break: a write cut short".to_owned()),
        Verdict::NoClaim => Some(match kind {
            ReceiptKind::ProveOut => "no line ALL-PROVED".to_owned(),
            _ => "no proof stated".to_owned(),
        }),
        Verdict::TooBig(len) => Some(format!(
            "{len} bytes, past the {MAX_RECEIPT_BYTES} a receipt is read to"
        )),
        Verdict::Unreadable(why) => Some(why),
    };
    receipt
}

/// The verdict on a receipt's whole text.
fn judge(
    text: &str,
    kind: ReceiptKind,
    archives: &Archives,
    receipt: &mut Receipt,
) -> Verdict {
    if text.trim().is_empty() {
        return Verdict::Empty;
    }
    if !text.ends_with('\n') {
        return Verdict::Torn;
    }
    receipt.statement = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .chars()
        .take(MAX_STATEMENT_CHARS)
        .collect();
    match kind {
        ReceiptKind::ProveOut if !text.lines().any(|line| line.trim() == "ALL-PROVED") => {
            Verdict::NoClaim
        }
        ReceiptKind::VerifiedTsv => {
            sources_of(text, archives, receipt);
            Verdict::States
        }
        _ => Verdict::States,
    }
}

/// Counts a `VERIFIED.tsv`'s rows that name a source and the ones that match
/// a walked live archive file today.
fn sources_of(
    text: &str,
    archives: &Archives,
    receipt: &mut Receipt,
) {
    let mut lines = text.lines();
    let Some(column) = lines
        .next()
        .and_then(|header| header.split('\t').position(|name| name.trim() == "source_identity"))
    else {
        return;
    };
    for (row, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        receipt.sources = receipt.sources.saturating_add(1);
        let field = line.split('\t').nth(column).unwrap_or("");
        let found = SourceIdentity::parse(field).map(|identity| archives.find(&identity));
        match found {
            Some(Some(_)) => receipt.matched = receipt.matched.saturating_add(1),
            other => {
                receipt.first_unmatched.get_or_insert_with(|| {
                    let why = if other.is_none() {
                        format!("{field:?} is not dev:inode:size:mtime_s:mtime_ns:ctime_s:ctime_ns")
                    } else {
                        format!(
                            "{field:?} matches no walked live archive file by inode, size and \
                             times (dev not compared)"
                        )
                    };
                    format!("row {}: {why}", row + 2)
                });
            }
        }
    }
}

/// Whether a copy root's bytes have a proven twin (D-7045).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Twin {
    /// Every row of every stating `VERIFIED.tsv` names a source that matches
    /// a walked live archive file today.
    Proven {
        /// Rows matched.
        sources: u64,
    },
    /// No such proof: the words say why.
    NotProven(String),
}

impl Twin {
    /// The twin in words, as the retirement list says it.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::Proven { sources } => format!(
                "a proven twin: {sources} source row(s) of its VERIFIED.tsv receipt(s) match a \
                 live archive file today by inode, size, mtime and ctime (st_dev not compared)"
            ),
            Self::NotProven(why) => {
                format!("no proven twin ({why}): these bytes may be the ONLY copy")
            }
        }
    }
}

/// The twin the receipts of one root prove.
fn twin_of(receipts: &[Receipt]) -> Twin {
    let tsv: Vec<&Receipt> = receipts
        .iter()
        .filter(|r| r.kind == ReceiptKind::VerifiedTsv)
        .collect();
    if tsv.is_empty() {
        return Twin::NotProven("no VERIFIED.tsv receipt names a source".to_owned());
    }
    let mut sources = 0_u64;
    for receipt in &tsv {
        if receipt.verdict != "states" {
            return Twin::NotProven(format!(
                "{} is {}",
                quoted(&receipt.path),
                receipt.verdict
            ));
        }
        if receipt.sources == 0 {
            return Twin::NotProven(format!(
                "{} names no source_identity row",
                quoted(&receipt.path)
            ));
        }
        if receipt.matched != receipt.sources {
            return Twin::NotProven(format!(
                "{}: {} of {} source rows match ({})",
                quoted(&receipt.path),
                receipt.matched,
                receipt.sources,
                receipt.first_unmatched.as_deref().unwrap_or("")
            ));
        }
        sources = sources.saturating_add(receipt.sources);
    }
    Twin::Proven { sources }
}

// ═══ The register ══════════════════════════════════════════════════════════

/// One name under a copy root that is a second name of a walked live file:
/// a red row (D-7042).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hazard {
    /// The copy root's name of the file.
    pub path: PathBuf,
    /// The live root's name of the same file (the first by path).
    pub live: PathBuf,
    /// The live root's kind.
    pub kind: LiveKind,
    /// `st_dev`, within this walk.
    pub dev: u64,
    /// `st_ino`, within this walk.
    pub ino: u64,
    /// `st_nlink`.
    pub nlink: u64,
    /// The file's bytes, which are the live file's: removing this name
    /// frees none of them.
    pub bytes: u64,
    /// Whether the name is a partial download's (`.part`, `.tmp`): the
    /// resumed-write case.
    pub partial: bool,
}

/// What became of one copy root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Walked; `whole` is false when a fault was met (see `faults`).
    Walked {
        /// Every name was read.
        whole: bool,
    },
    /// Not walked, or refused past a bound: no figure is given.
    Refused(String),
}

/// One copy root, as the register lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyRoot {
    /// The folder.
    pub path: PathBuf,
    /// What became of it.
    pub state: State,
    /// Regular files.
    pub files: u64,
    /// Folders below the root.
    pub dirs: u64,
    /// Links met and not followed.
    pub links: u64,
    /// Pipes, sockets and devices met and not opened.
    pub others: u64,
    /// Bytes of its files, each file once whatever its names here.
    pub bytes: u64,
    /// Of those, bytes that are its own: not a live file's, and not counted
    /// already by an earlier copy root.
    pub extra_bytes: u64,
    /// The newest `st_mtime` of its files, seconds.
    pub newest_mtime_s: Option<i64>,
    /// Its reader, or `None`.
    pub reader: Option<Reader>,
    /// The live root it lies inside, when no reader reads it.
    pub inside: Option<LiveRoot>,
    /// Receipts read, the first by path.
    pub receipts: Vec<Receipt>,
    /// Receipts found, read or not.
    pub receipt_count: u64,
    /// Its proof: the first receipt that states one.
    pub proof: Option<PathBuf>,
    /// Whether its bytes have a proven twin.
    pub twin: Twin,
    /// Names here that are second names of walked live files (each a hazard).
    pub linked_live: u64,
    /// Files with more names than the walk found: the first by path.
    pub linked_elsewhere: Listed<PathBuf>,
    /// Faults: the first by path, `(path, why)`.
    pub faults: Listed<(PathBuf, String)>,
    /// Other roots met inside it and not walked as part of it, in order.
    pub skipped: BTreeSet<PathBuf>,
}

impl CopyRoot {
    /// Whether it lies on the retirement list: walked whole, nothing reads it
    /// (D-7047).
    #[must_use]
    pub const fn retirable(&self) -> bool {
        matches!(self.state, State::Walked { whole: true }) && self.reader.is_none()
    }
}

/// One live root, as the register lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveReport {
    /// The root.
    pub root: LiveRoot,
    /// Whether it was walked for second names.
    pub walked: bool,
    /// Why not, or why it was refused.
    pub why: Option<String>,
    /// Files walked.
    pub files: u64,
}

/// One walk's register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Register {
    /// When the walk began, UNIX microseconds (the wall clock).
    pub walked_at_micros: i64,
    /// How long it took, microseconds (a monotonic clock).
    pub walk_micros: u64,
    /// The copy roots, in the setting's order.
    pub roots: Vec<CopyRoot>,
    /// The live roots, configured or not.
    pub live: Vec<LiveReport>,
    /// The live roots that are not configured, named.
    pub unset_live: Vec<String>,
    /// Second names of live files under the copy roots: the first by path.
    pub hazards: Listed<Hazard>,
    /// The copy roots' own bytes, every root walked whole or not (a lower
    /// bound when one was not).
    pub extra_bytes: u64,
}

impl Register {
    /// Whether every copy root and every walked live root was walked whole:
    /// only then is "no hazard" a statement rather than a hope.
    #[must_use]
    pub fn whole(&self) -> bool {
        self.roots
            .iter()
            .all(|root| matches!(root.state, State::Walked { whole: true }))
            && self.live.iter().all(|live| !live.walked || live.why.is_none())
    }
}

/// The current wall clock, UNIX microseconds (negative before 1970).
#[must_use]
pub fn now_micros() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_micros()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_micros())
            .map_or(i64::MIN, i64::saturating_neg),
    }
}

/// One live root's walk, or why it was not walked.
type LiveWalk = Result<Walked, String>;

/// The live archive files by `(st_ino, st_size)`, which a receipt's
/// `source_identity` is matched against (D-7045).
#[derive(Debug, Default)]
pub struct Archives {
    by_ino_size: HashMap<(u64, u64), Vec<(PathBuf, Meta)>>,
}

impl Archives {
    /// An empty table, pre-sized for `files` files.
    #[must_use]
    pub fn with_capacity(files: usize) -> Self {
        Self {
            by_ino_size: HashMap::with_capacity(files),
        }
    }

    /// Files one live archive file.
    pub fn add(&mut self, path: PathBuf, meta: Meta) {
        self.by_ino_size
            .entry((meta.ino, meta.size))
            .or_insert_with(|| Vec::with_capacity(1))
            .push((path, meta));
    }

    /// The walked live archive file `identity` names: one hash probe, then
    /// the files sharing that inode and size, which are one on a sound tree.
    #[must_use]
    pub fn find(&self, identity: &SourceIdentity) -> Option<&Path> {
        for (path, meta) in self.by_ino_size.get(&(identity.ino, identity.size))? {
            if identity.matches(meta) {
                return Some(path);
            }
        }
        None
    }
}

/// Walks every root of `config` once and compares them (D-7042..D-7045).
/// The roots are walked in parallel, one thread each; the merge is in the
/// setting's order, so the answer does not depend on which finished first
/// (`CLAUDE.md` §3 rule 5).
#[must_use]
pub fn walk(fs: &dyn Fs, config: &Config, walked_at_micros: i64) -> Register {
    let started = Instant::now();
    let ids: Vec<Option<(u64, u64)>> = config
        .copy_roots
        .iter()
        .chain(config.live.iter().map(|root| &root.path))
        .map(|path| match fs.lstat(path) {
            Ok(meta) if meta.kind == Kind::Dir => Some(meta.id()),
            _ => None,
        })
        .collect();
    // EVERY OTHER ROOT IS SKIPPED WHERE IT IS MET, by its `(st_dev, st_ino)`,
    // so each root is walked as itself only and no file is two roots'.
    let skip_for = |own: usize| -> HashSet<(u64, u64)> {
        let mine = ids.get(own).copied().flatten();
        let mut skip = HashSet::with_capacity(ids.len());
        for (at, id) in ids.iter().enumerate() {
            if let Some(id) = id
                && at != own
                && Some(*id) != mine
            {
                skip.insert(*id);
            }
        }
        skip
    };
    let copies = config.copy_roots.len();
    let joined = |handle: std::thread::ScopedJoinHandle<'_, Result<Walked, String>>| {
        handle
            .join()
            .unwrap_or_else(|_| Err("the walk's thread panicked".to_owned()))
    };
    let (copy_walks, live_walks): (Vec<Result<Walked, String>>, Vec<Option<LiveWalk>>) =
        std::thread::scope(|scope| {
            let copy_handles: Vec<_> = config
                .copy_roots
                .iter()
                .enumerate()
                .map(|(at, path)| {
                    let skip = skip_for(at);
                    scope.spawn(move || walk_root(fs, path, &skip, false))
                })
                .collect();
            let live_handles: Vec<_> = config
                .live
                .iter()
                .enumerate()
                .map(|(at, root)| {
                    let skip = skip_for(copies + at);
                    let keep = root.kind != LiveKind::TickStore;
                    root.kind
                        .walked()
                        .then(|| scope.spawn(move || walk_root(fs, &root.path, &skip, keep)))
                })
                .collect();
            (
                copy_handles.into_iter().map(joined).collect(),
                live_handles
                    .into_iter()
                    .map(|handle| handle.map(joined))
                    .collect(),
            )
        });
    let (live, mut links) = live_side(config, live_walks);
    links.index_copies(&copy_walks);
    let mut hazards: Listed<Hazard> = Listed::default();
    let mut roots = Vec::with_capacity(copies);
    let mut extra_bytes = 0_u64;
    for (at, (path, walk)) in config.copy_roots.iter().zip(copy_walks).enumerate() {
        let root = copy_root_of(fs, config, &links, at, path, walk, &mut hazards);
        extra_bytes = extra_bytes.saturating_add(root.extra_bytes);
        roots.push(root);
    }
    Register {
        walked_at_micros,
        walk_micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
        roots,
        live,
        unset_live: config.unset_live.clone(),
        hazards,
        extra_bytes,
    }
}

/// What the merge knows of every shared file: its live names, every name
/// the walk found, the copy root that counts its bytes, and the live archive
/// files for the receipts.
#[derive(Debug, Default)]
struct Links {
    live_names: HashMap<(u64, u64), Vec<(PathBuf, LiveKind)>>,
    all_names: HashMap<(u64, u64), HashSet<PathBuf>>,
    counted_in: HashMap<(u64, u64), usize>,
    archives: Archives,
}

impl Links {
    /// Files every copy root's shared files: their names, and which root
    /// counts their bytes (the first in the setting's order).
    fn index_copies(&mut self, copy_walks: &[Result<Walked, String>]) {
        for (at, walked) in copy_walks.iter().enumerate() {
            let Ok(walked) = walked else {
                continue;
            };
            for (id, names) in &walked.shared {
                self.all_names
                    .entry(*id)
                    .or_insert_with(|| HashSet::with_capacity(names.len()))
                    .extend(names.iter().map(|named| named.path.clone()));
                self.counted_in.entry(*id).or_insert(at);
            }
        }
    }
}

/// The live roots' reports, and what their walks found.
fn live_side(config: &Config, live_walks: Vec<Option<LiveWalk>>) -> (Vec<LiveReport>, Links) {
    let files: usize = live_walks
        .iter()
        .map(|walk| match walk {
            Some(Ok(walked)) => walked.identities.len().saturating_add(walked.shared.len()),
            _ => 0,
        })
        .sum();
    let mut links = Links {
        live_names: HashMap::with_capacity(files),
        all_names: HashMap::with_capacity(files),
        counted_in: HashMap::with_capacity(files),
        archives: Archives::with_capacity(files),
    };
    let mut live = Vec::with_capacity(config.live.len());
    for (root, walk) in config.live.iter().zip(live_walks) {
        let (walked, why, files) = match walk {
            None => (
                false,
                Some(
                    "not walked for second names (a ledger, the masters or the store: a second \
                     name of one of its files reads as linked elsewhere)"
                        .to_owned(),
                ),
                0,
            ),
            Some(Err(why)) => (true, Some(why), 0),
            Some(Ok(walked)) => {
                for (id, names) in walked.shared {
                    for named in names {
                        links
                            .all_names
                            .entry(id)
                            .or_insert_with(|| HashSet::with_capacity(2))
                            .insert(named.path.clone());
                        links
                            .live_names
                            .entry(id)
                            .or_insert_with(|| Vec::with_capacity(2))
                            .push((named.path.clone(), root.kind));
                        if root.kind != LiveKind::TickStore {
                            links.archives.add(named.path, named.meta);
                        }
                    }
                }
                for (path, meta) in walked.identities {
                    if meta.nlink <= 1 {
                        links.archives.add(path, meta);
                    }
                }
                let why = walked.faults.first.first().map(|(path, why)| {
                    format!(
                        "{} fault(s) while walked, the first: {}: {why}",
                        walked.faults.count,
                        quoted(path)
                    )
                });
                (true, why, walked.files)
            }
        };
        live.push(LiveReport {
            root: root.clone(),
            walked,
            why,
            files,
        });
    }
    (live, links)
}

/// One copy root, compared with the live roots and the other copy roots.
fn copy_root_of(
    fs: &dyn Fs,
    config: &Config,
    links: &Links,
    at: usize,
    path: &Path,
    walk: Result<Walked, String>,
    hazards: &mut Listed<Hazard>,
) -> CopyRoot {
    let (reader, inside) = reader_of(path, &config.live);
    let walked = match walk {
        Ok(walked) => walked,
        Err(why) => return refused_root(path, why, reader, inside),
    };
    let mut bytes = walked.single_bytes;
    let mut extra = walked.single_bytes;
    let mut linked_live = 0_u64;
    let mut linked_elsewhere: Listed<PathBuf> = Listed::default();
    for (id, names) in &walked.shared {
        let Some(first) = names.first() else {
            continue;
        };
        bytes = bytes.saturating_add(first.meta.size);
        let seen = links.all_names.get(id).map_or(0, HashSet::len);
        if u64::try_from(seen).unwrap_or(u64::MAX) < first.meta.nlink {
            let smallest = names.iter().map(|n| &n.path).min().unwrap_or(&first.path);
            linked_elsewhere.offer(smallest.clone());
        }
        let Some(lives) = links.live_names.get(id) else {
            if links.counted_in.get(id) == Some(&at) {
                extra = extra.saturating_add(first.meta.size);
            }
            continue;
        };
        // THE BYTES ARE THE LIVE FILE'S, so none is this root's own; and each
        // name here that is not itself a live name is red.
        let Some((live_path, kind)) = lives.iter().min_by(|a, b| a.0.cmp(&b.0)) else {
            continue;
        };
        for named in names {
            if lives.iter().any(|(live, _)| *live == named.path) {
                continue;
            }
            linked_live = linked_live.saturating_add(1);
            hazards.offer(Hazard {
                path: named.path.clone(),
                live: live_path.clone(),
                kind: *kind,
                dev: id.0,
                ino: id.1,
                nlink: named.meta.nlink,
                bytes: named.meta.size,
                partial: is_partial(&named.path),
            });
        }
    }
    let receipts: Vec<Receipt> = walked
        .receipts
        .first
        .iter()
        .take(MAX_RECEIPTS_READ)
        .filter_map(|receipt| {
            let kind = receipt_kind(receipt.file_name()?)?;
            Some(read_receipt(fs, receipt, kind, &links.archives))
        })
        .collect();
    let proof = receipts
        .iter()
        .filter(|receipt| receipt.verdict == "states")
        .map(|receipt| receipt.path.clone())
        .next();
    let twin = twin_of(&receipts);
    CopyRoot {
        path: path.to_path_buf(),
        state: State::Walked {
            whole: walked.faults.count == 0,
        },
        files: walked.files,
        dirs: walked.dirs,
        links: walked.links,
        others: walked.others,
        bytes,
        extra_bytes: extra,
        newest_mtime_s: walked.newest_mtime_s,
        reader,
        inside,
        receipts,
        receipt_count: walked.receipts.count,
        proof,
        twin,
        linked_live,
        linked_elsewhere,
        faults: walked.faults,
        skipped: walked.skipped,
    }
}

/// A copy root that was not walked.
fn refused_root(
    path: &Path,
    why: String,
    reader: Option<Reader>,
    inside: Option<LiveRoot>,
) -> CopyRoot {
    CopyRoot {
        path: path.to_path_buf(),
        state: State::Refused(why),
        files: 0,
        dirs: 0,
        links: 0,
        others: 0,
        bytes: 0,
        extra_bytes: 0,
        newest_mtime_s: None,
        reader,
        inside,
        receipts: Vec::new(),
        receipt_count: 0,
        proof: None,
        twin: Twin::NotProven("the root was not walked".to_owned()),
        linked_live: 0,
        linked_elsewhere: Listed::default(),
        faults: Listed::default(),
        skipped: BTreeSet::new(),
    }
}

/// Whether a name is a partial download's: `.part` or `.tmp` at its end.
#[must_use]
pub fn is_partial(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == OsStr::new("part") || ext == OsStr::new("tmp"))
}

/// A path quoted with every control character escaped, so a name holding a
/// line break stays one line of text (D-7047).
#[must_use]
#[allow(
    clippy::unnecessary_debug_formatting,
    reason = "the escaping IS the point: a name holding a line break or a byte that is not \
              UTF-8 is written exactly, on one line, and never as a lossy guess"
)]
pub fn quoted(path: &Path) -> String {
    format!("{:?}", path.as_os_str())
}

// ═══ The retirement list ═══════════════════════════════════════════════════

/// One line of the retirement list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retire {
    /// The folder or name.
    pub path: PathBuf,
    /// The bytes its removal would free.
    pub bytes: u64,
    /// Files under it.
    pub files: u64,
    /// Why it is listed.
    pub why: String,
    /// Whether its bytes have a proven twin, in words.
    pub twin: String,
    /// What to take care of, when anything.
    pub caution: Option<String>,
    /// Whether it is a hazard row (a second name of a live file).
    pub hazard: bool,
}

/// The retirement list (D-7047): every copy root walked whole that nothing
/// reads, with the bytes it would free and whether those bytes are the only
/// copy; then every hazard name, which frees nothing and removes the
/// in-place-change hazard. A root that holds a live root, lies in a read
/// part of one, or was not walked whole is never on it.
#[must_use]
pub fn retirement(register: &Register) -> Vec<Retire> {
    let mut list: Vec<Retire> = register
        .roots
        .iter()
        .filter(|root| root.retirable())
        .map(|root| Retire {
            path: root.path.clone(),
            bytes: root.extra_bytes,
            files: root.files,
            why: "no configured root, setting or store step names it".to_owned(),
            twin: root.twin.words(),
            caution: retire_caution(root),
            hazard: false,
        })
        .collect();
    list.extend(register.hazards.first.iter().map(|hazard| Retire {
        path: hazard.path.clone(),
        bytes: 0,
        files: 1,
        why: format!(
            "a second name of the live {} file {} (st_dev {}, st_ino {}, nlink {})",
            hazard.kind.word(),
            quoted(&hazard.live),
            hazard.dev,
            hazard.ino,
            hazard.nlink
        ),
        twin: "frees no byte: the data stays at the live name".to_owned(),
        caution: Some(
            "removing this NAME removes the in-place-change hazard and leaves the live file's \
             data intact"
                .to_owned(),
        ),
        hazard: true,
    }));
    list
}

/// What to take care of when retiring `root`.
fn retire_caution(root: &CopyRoot) -> Option<String> {
    let mut words = Vec::with_capacity(3);
    if let Some(live) = &root.inside {
        words.push(format!(
            "it lies inside the live {} root {}: retire only this folder, never its parent",
            live.kind.word(),
            quoted(&live.path)
        ));
    }
    if root.linked_live > 0 {
        words.push(format!(
            "{} name(s) in it are second names of live files (the hazard rows): their bytes are \
             the live files' and are not counted here",
            root.linked_live
        ));
    }
    if root.linked_elsewhere.count > 0 {
        words.push(format!(
            "{} file(s) in it have another name outside every walked root",
            root.linked_elsewhere.count
        ));
    }
    (!words.is_empty()).then(|| words.join("; "))
}

/// The retirement list as plain text, for the operator to read and act on
/// by hand. It holds no command: each path is quoted with control
/// characters escaped (D-7047).
#[must_use]
pub fn retirement_text(register: &Register) -> String {
    let list = retirement(register);
    let mut text = format!(
        "RETIREMENT LIST, walked {}: {RETIREMENT_SENTENCE}.\n",
        ist_text(register.walked_at_micros)
    );
    if list.is_empty() {
        text.push_str("Nothing to retire: every copy root walked has a reader or was not walked whole.\n");
    }
    for (n, retire) in list.iter().enumerate() {
        let _ = writeln!(
            text,
            "{}. {}{}\n   bytes freed: {} in {} file(s)\n   why: {}\n   twin: {}",
            n + 1,
            if retire.hazard { "[HAZARD] " } else { "" },
            quoted(&retire.path),
            retire.bytes,
            retire.files,
            retire.why,
            retire.twin
        );
        if let Some(caution) = &retire.caution {
            let _ = writeln!(text, "   take care: {caution}");
        }
    }
    if register.hazards.count > u64::try_from(register.hazards.first.len()).unwrap_or(0) {
        let _ = writeln!(
            text,
            "... and {} more hazard name(s); the register lists the first {MAX_LISTED} by path.",
            register.hazards.count
                - u64::try_from(register.hazards.first.len()).unwrap_or(0)
        );
    }
    text
}

/// UNIX microseconds as `YYYY-MM-DD HH:MM:SS IST`.
#[must_use]
pub fn ist_text(micros: i64) -> String {
    let secs = micros.div_euclid(1_000_000).saturating_add(19_800);
    let (year, month, day) = telemetry::civil_from_days(secs.div_euclid(86_400));
    let of_day = secs.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} IST",
        of_day / 3_600,
        of_day % 3_600 / 60,
        of_day % 60
    )
}

/// One event a register puts, its fields as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Its level.
    pub level: telemetry::Level,
    /// What it says.
    pub message: &'static str,
    /// Its fields.
    pub fields: Vec<(&'static str, String)>,
}

/// The events a register puts (D-7048): one `error` per hazard listed (each
/// is audited, `telemetry::is_audited`), one `warn` per copy root or walked
/// live root not walked whole, so `/logs` shows every refusal by name, and
/// one `info` summary.
#[must_use]
pub fn notes(register: &Register) -> Vec<Note> {
    let mut notes = Vec::with_capacity(register.hazards.first.len() + register.roots.len() + 1);
    for hazard in &register.hazards.first {
        notes.push(Note {
            level: telemetry::Level::Error,
            message: "live archive file hard-linked under another name",
            fields: vec![
                ("path", quoted(&hazard.path)),
                ("live", quoted(&hazard.live)),
                ("kind", hazard.kind.word().to_owned()),
                ("dev", hazard.dev.to_string()),
                ("ino", hazard.ino.to_string()),
                ("nlink", hazard.nlink.to_string()),
            ],
        });
    }
    for root in &register.roots {
        let why = match &root.state {
            State::Refused(why) => why.clone(),
            State::Walked { whole: false } => format!("{} fault(s) while walked", root.faults.count),
            State::Walked { whole: true } => continue,
        };
        notes.push(Note {
            level: telemetry::Level::Warn,
            message: "copy root not walked whole",
            fields: vec![("path", quoted(&root.path)), ("why", why)],
        });
    }
    for live in &register.live {
        if let (true, Some(why)) = (live.walked, &live.why) {
            notes.push(Note {
                level: telemetry::Level::Warn,
                message: "live root not walked whole",
                fields: vec![
                    ("path", quoted(&live.root.path)),
                    ("kind", live.root.kind.word().to_owned()),
                    ("why", why.clone()),
                ],
            });
        }
    }
    notes.push(Note {
        level: telemetry::Level::Info,
        message: "copies register walked",
        fields: vec![
            ("roots", register.roots.len().to_string()),
            ("extra_bytes", register.extra_bytes.to_string()),
            ("hazards", register.hazards.count.to_string()),
            ("walk_us", register.walk_micros.to_string()),
            ("whole", register.whole().to_string()),
        ],
    });
    notes
}

/// Puts [`notes`].
pub fn note_walked(register: &Register) {
    for note in notes(register) {
        let event = note.fields.iter().fold(
            telemetry::Event::new(note.level, TARGET, note.message),
            |event, (key, value)| event.with(key, telemetry::Value::Str(value)),
        );
        let _dropped = telemetry::emit(&event);
    }
}

// ═══ Served from memory ════════════════════════════════════════════════════

/// One held register and what it was walked for.
#[derive(Debug)]
struct Held {
    config: Config,
    stamps: Vec<RootStat>,
    register: Arc<Register>,
    at: Instant,
}

/// The register a process serves: walked when first asked, re-walked only
/// when the configuration or a root's own stat changes (D-7046).
#[derive(Debug, Default)]
pub struct Cache {
    held: Mutex<Option<Held>>,
    walking: Mutex<()>,
}

/// What [`answer`] served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Served {
    /// A register, with how old it is and whether it was walked by this
    /// request.
    Register {
        /// The register.
        register: Arc<Register>,
        /// Milliseconds since its walk finished, from a monotonic clock.
        age_ms: u64,
        /// Whether this request walked it.
        fresh: bool,
        /// Why this request walked, or why the held one is being re-walked.
        why: String,
        /// Whether another request is re-walking it now: this one is
        /// served the held register meanwhile.
        rewalking: bool,
    },
    /// The first walk since start is running in another request; nothing is
    /// held yet.
    Walking,
}

impl Cache {
    /// An empty cache.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            held: Mutex::new(None),
            walking: Mutex::new(()),
        }
    }

    /// The held register's extra bytes, never walking (for `/disk.json`).
    #[must_use]
    pub fn held(&self) -> Option<(Arc<Register>, u64)> {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|held| {
                (
                    Arc::clone(&held.register),
                    u64::try_from(held.at.elapsed().as_millis()).unwrap_or(u64::MAX),
                )
            })
    }
}

/// The process's one cache, which `GET /disk/copies.json` serves and
/// [`total`] reads.
#[must_use]
pub fn process_cache() -> &'static Cache {
    static CACHE: Cache = Cache::new();
    &CACHE
}

/// The register for `config` (D-7046): the held one when the configuration
/// and every root's own stat are as they were, otherwise a walk by this
/// request. A request that finds another walking is never made to wait for
/// it.
pub fn answer(cache: &Cache, fs: &dyn Fs, config: &Config, now: &dyn Fn() -> i64) -> Served {
    let stamps = stamps(fs, config);
    let why = {
        let held = cache.held.lock().unwrap_or_else(PoisonError::into_inner);
        match held.as_ref() {
            Some(held) if held.config == *config && held.stamps == stamps => {
                return Served::Register {
                    register: Arc::clone(&held.register),
                    age_ms: u64::try_from(held.at.elapsed().as_millis()).unwrap_or(u64::MAX),
                    fresh: false,
                    why: "served from memory: the roots and their own stats are unchanged"
                        .to_owned(),
                    rewalking: false,
                };
            }
            Some(held) if held.config != *config => {
                "the configured roots changed since the last walk".to_owned()
            }
            Some(held) => changed_root(config, &held.stamps, &stamps),
            None => "the first request since start".to_owned(),
        }
    };
    let Ok(_walking) = cache.walking.try_lock() else {
        let held = cache.held.lock().unwrap_or_else(PoisonError::into_inner);
        return held.as_ref().map_or(Served::Walking, |held| Served::Register {
            register: Arc::clone(&held.register),
            age_ms: u64::try_from(held.at.elapsed().as_millis()).unwrap_or(u64::MAX),
            fresh: false,
            why: format!("being re-walked by another request ({why}); this is the last walk"),
            rewalking: true,
        });
    };
    // LOOK AGAIN UNDER THE WALK'S LOCK: a request that walked while this one
    // compared may have held exactly this answer already, and a second walk
    // of the same roots would be work done twice.
    if let Some(held) = cache
        .held
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .filter(|held| held.config == *config && held.stamps == stamps)
    {
        return Served::Register {
            register: Arc::clone(&held.register),
            age_ms: u64::try_from(held.at.elapsed().as_millis()).unwrap_or(u64::MAX),
            fresh: false,
            why: "served from memory: another request walked these roots a moment ago"
                .to_owned(),
            rewalking: false,
        };
    }
    let register = Arc::new(walk(fs, config, now()));
    note_walked(&register);
    *cache.held.lock().unwrap_or_else(PoisonError::into_inner) = Some(Held {
        config: config.clone(),
        stamps,
        register: Arc::clone(&register),
        at: Instant::now(),
    });
    Served::Register {
        register,
        age_ms: 0,
        fresh: true,
        why,
        rewalking: false,
    }
}

/// The first root whose own stat differs, in words.
fn changed_root(config: &Config, before: &[RootStat], now: &[RootStat]) -> String {
    config
        .copy_roots
        .iter()
        .chain(config.live.iter().map(|root| &root.path))
        .zip(before.iter().zip(now))
        .find(|(_, (was, is))| was != is)
        .map_or_else(
            || "a root's own stat changed".to_owned(),
            |(path, _)| format!("the own stat of {} changed", quoted(path)),
        )
}

/// What [`total`] answers for `/disk.json` (R4-29's one field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Total {
    /// The extra bytes of the last walk.
    Measured {
        /// Bytes.
        bytes: u64,
        /// When the walk began, UNIX microseconds.
        walked_at_micros: i64,
        /// Whether every root was walked whole (a lower bound when not).
        whole: bool,
    },
    /// No walk is held: the register was never asked since start, or its
    /// setting is unset.
    NotWalked,
}

/// The extra copies' bytes the held register counts: never a walk, one lock
/// (R4-29 calls this from `/disk.json`, amendment (b)).
#[must_use]
pub fn total(cache: &Cache) -> Total {
    cache
        .held()
        .map_or(Total::NotWalked, |(register, _)| Total::Measured {
            bytes: register.extra_bytes,
            walked_at_micros: register.walked_at_micros,
            whole: register.whole(),
        })
}

#[cfg(test)]
#[path = "copies_register_tests.rs"]
mod tests;
