//! The `_v3` extraction ledger, read only, as evidence and never as a source
//! (D-4814, D-4867; IA-3-OPT).
//!
//! `<ROOT>/<yyyy>/_v3/` beside the operator's options tick folders is a
//! separate tool's verify-and-place ledger (design §2.1): per options day zip
//! a manifest `manifest/<hex>.ok.tsv` and an ok marker `ok/<hex>.ok`, where
//! `<hex>` is the lowercase hex of the day zip's name inside its year zip,
//! `<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.zip` ([`NfoZips::day_zip_name`]),
//! plus `owner/`, `runs/`, `fsid`, `lock.guard` and `place.lock`. No bar is
//! ever read from it. This module opens the manifest and the ok marker of a
//! day read-only and nothing else: it never writes, creates, renames,
//! removes, locks or flocks anything, and never names `lock.guard` or
//! `place.lock` (`the_v3_ledger_is_never_opened_for_writing` pins the
//! source).
//!
//! # The root comes from configuration only (D-4867, `CLAUDE.md` §8)
//!
//! [`V3_LEDGER_VAR`] is read once, through the one rule every
//! `BRUTEX_GDFL_*` source root is read by ([`crate::gdfl_run::source_root`],
//! D-4801). Unset or empty, every day answers
//! `v3: absent (BRUTEX_GDFL_V3_LEDGER unset)`: there is no default path.
//!
//! # What the ledger holds, as measured (20 Oct 2026, read only)
//!
//! A manifest line is exactly 6 tab fields: the path
//! `<MON_yyyy>/<day folder>/Options/<T>.NFO.csv` with forward slashes even
//! where the day zip uses backslashes, the size in decimal, the CRC-32 as 8
//! hex digits, a field that is 1 on exactly the 2 directory lines of
//! 2026-09-24 and 0 elsewhere, a field that is 0 everywhere, and the source
//! day zip's name. One line per day-zip entry, directories included, each
//! ending in a newline. Fields 4 and 5 are parsed as decimal and kept as
//! read; their meaning is **UNVERIFIED** (design §3.11) and nothing compares
//! or interprets them.
//!
//! An ok marker is the line `x3-marker 1` and then one `key=value` line each
//! for `name_hex`, `crc`, `size`, `manifest_sha256`, `day_hex`, `files`,
//! `bytes` and `run`. Measured over every marker of the ledger
//! (`the_real_v3_ledger_agrees_with_the_real_year_zips`): `manifest_sha256`
//! is the manifest's sha256, `files` its count of lines whose path does not
//! end in `/`, `bytes` the sum of its sizes, `name_hex` the hex of the day
//! zip's name and `day_hex` of that name without `.zip`; `crc` and `size`
//! are the **day zip's** CRC-32 and size as its year zip states them, not
//! the manifest's (the design's "crc, size and sha256 of the manifest" is
//! corrected by that measurement). A manifest is trusted only when its
//! marker agrees with it on all five; otherwise the day is
//! [`V3::Unverified`] by name, never compared.
//!
//! # The comparison: O(entries of the day) plus one sha256
//!
//! A day-zip listing entry maps to a manifest path as `<MON_yyyy>/` plus the
//! entry with every `\` turned to `/`; directory entries are kept on both
//! sides (design §6.1, :2221). Line `i` of a trusted manifest is held to
//! entry `i` of the listing, path, size and CRC-32, and its source to the
//! day zip's name; the day zip's own size, where the source read one, to the
//! marker's `size`. [`V3::Agrees`] names the lines; [`V3::Disagrees`] the
//! count of lines that differ and the first difference. One read of each of
//! the two files (at most [`MANIFEST_MAX_BYTES`] and [`MARKER_MAX_BYTES`]),
//! one sha256 of the manifest and one pass over its lines: no map, no sort,
//! no allocation per line. `docs/06-limits.md` carries the measured row.

use std::ffi::OsString;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::gdfl_nfo::{NfoDay, NfoZips};
use crate::session::Day;

/// The variable that names the ledger's root: the directory holding one
/// `<yyyy>/_v3/` per year. No default (D-4867).
pub const V3_LEDGER_VAR: &str = "BRUTEX_GDFL_V3_LEDGER";

/// The largest manifest read: 64 MiB, about 20 times the largest measured
/// (26,820 lines). A larger one is [`V3::Unverified`], never read whole.
pub const MANIFEST_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// The largest ok marker read: 4 KiB, about 10 times the measured 400 bytes.
pub const MARKER_MAX_BYTES: u64 = 4 * 1024;

/// An ok marker's first line, as measured on every marker.
const MARKER_HEAD: &str = "x3-marker 1";

/// The keys an ok marker states, each once, in no required order.
const MARKER_KEYS: [&str; 8] = [
    "name_hex",
    "crc",
    "size",
    "manifest_sha256",
    "day_hex",
    "files",
    "bytes",
    "run",
];

/// What the ledger says of one options day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum V3 {
    /// No evidence: the variable is unset, or the ledger holds no manifest
    /// and no ok marker for the day zip. Named, never an error of the day.
    Absent {
        /// Why, in words: `BRUTEX_GDFL_V3_LEDGER unset`, or the day zip the
        /// ledger has no manifest for.
        why: String,
    },
    /// A manifest that is not trusted: its ok marker is missing, unreadable,
    /// malformed or disagrees with it. Never compared, so never
    /// [`V3::Agrees`].
    Unverified {
        /// Why, in words.
        why: String,
    },
    /// A trusted manifest with a line that is not 6 well-formed fields,
    /// named by its one-based line number; never skipped.
    Malformed {
        /// The one-based line.
        line: usize,
        /// What is wrong with it.
        why: String,
    },
    /// A trusted manifest that agrees with the listing line for line.
    Agrees {
        /// Its lines, directories included.
        lines: usize,
    },
    /// A trusted manifest that does not.
    Disagrees {
        /// How many lines differ (the day zip's size counted as one).
        count: usize,
        /// The first difference.
        first: Difference,
    },
}

impl V3 {
    /// The day block's byte for this answer (design §5, :2877): 0 absent or
    /// unset, 1 agrees, 2 disagrees, 3 unverified (a malformed manifest is
    /// not one that can be trusted).
    #[must_use]
    pub const fn code(&self) -> u8 {
        match self {
            Self::Absent { .. } => 0,
            Self::Agrees { .. } => 1,
            Self::Disagrees { .. } => 2,
            Self::Unverified { .. } | Self::Malformed { .. } => 3,
        }
    }
}

impl core::fmt::Display for V3 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Absent { why } => write!(f, "v3: absent ({why})"),
            Self::Unverified { why } => write!(f, "v3: unverified ({why})"),
            Self::Malformed { line, why } => {
                write!(f, "v3: unverified (manifest line {line}: {why})")
            }
            Self::Agrees { lines } => write!(f, "v3: agrees ({lines} lines)"),
            Self::Disagrees { count, first } => {
                write!(f, "v3: disagrees ({count} lines; first {first})")
            }
        }
    }
}

/// One difference between a trusted manifest and the day's listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// The path it is at: the listing's, mapped, where the listing has the
    /// line; the manifest's where only the manifest has it; the day zip's
    /// name for its size.
    pub path: String,
    /// What differs.
    pub what: Differs,
}

impl core::fmt::Display for Difference {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.path, self.what)
    }
}

/// What differs at one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Differs {
    /// The listing has the line; the manifest ends before it.
    NotInManifest,
    /// The manifest has the line; the listing ends before it.
    NotInListing,
    /// The manifest names another path at this line.
    Path {
        /// The manifest's path.
        manifest: String,
    },
    /// The sizes differ.
    Size {
        /// The listing's.
        listed: u64,
        /// The manifest's.
        manifest: u64,
    },
    /// The CRC-32s differ.
    Crc {
        /// The listing's.
        listed: u32,
        /// The manifest's.
        manifest: u32,
    },
    /// The manifest files the line from another day zip.
    Source {
        /// The manifest's source.
        manifest: String,
    },
    /// The day zip's own size differs from the ok marker's.
    DayZipSize {
        /// The source's.
        listed: u64,
        /// The ok marker's.
        ledger: u64,
    },
}

impl core::fmt::Display for Differs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotInManifest => f.write_str("listed, not in the manifest"),
            Self::NotInListing => f.write_str("in the manifest, not listed"),
            Self::Path { manifest } => write!(f, "the manifest names {manifest} at this line"),
            Self::Size { listed, manifest } => {
                write!(f, "size {listed} listed, {manifest} in the manifest")
            }
            Self::Crc { listed, manifest } => {
                write!(f, "crc {listed:08x} listed, {manifest:08x} in the manifest")
            }
            Self::Source { manifest } => write!(f, "the manifest files it from {manifest}"),
            Self::DayZipSize { listed, ledger } => {
                write!(f, "day zip of {listed} bytes, {ledger} in the ok marker")
            }
        }
    }
}

/// The ledger as configuration names it, read once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum V3Ledger {
    /// [`V3_LEDGER_VAR`] names this root.
    At {
        /// The root, holding `<yyyy>/_v3/`.
        root: PathBuf,
    },
    /// [`V3_LEDGER_VAR`] is unset or empty.
    Unset,
}

/// One manifest line, as read.
struct Line<'a> {
    path: &'a str,
    size: u64,
    crc: u32,
    source: &'a str,
}

/// What a verified ok marker states that the comparison uses.
struct Marker {
    /// The day zip's size, as the year zip states it.
    zip_size: u64,
    /// The manifest's file lines.
    files: u64,
    /// The sum of the manifest's sizes.
    bytes: u64,
}

impl V3Ledger {
    /// The ledger `env` names, read through the same rule as every
    /// `BRUTEX_GDFL_*` source root (D-4801): unset or empty is
    /// [`V3Ledger::Unset`], never a default.
    #[must_use]
    pub fn from_env(env: &dyn Fn(&str) -> Option<OsString>) -> Self {
        crate::gdfl_run::source_root(V3_LEDGER_VAR, env)
            .map_or(Self::Unset, |root| Self::At { root })
    }

    /// The manifest of `day`'s zip, or `None` when unset.
    #[must_use]
    pub fn manifest_path(&self, day: Day) -> Option<PathBuf> {
        self.dir(day)
            .map(|(dir, hex)| dir.join("manifest").join(format!("{hex}.ok.tsv")))
    }

    /// The ok marker of `day`'s zip, or `None` when unset.
    #[must_use]
    pub fn marker_path(&self, day: Day) -> Option<PathBuf> {
        self.dir(day)
            .map(|(dir, hex)| dir.join("ok").join(format!("{hex}.ok")))
    }

    /// The year's `_v3` directory and the hex name of `day`'s zip.
    fn dir(&self, day: Day) -> Option<(PathBuf, String)> {
        match self {
            Self::At { root } => Some((
                root.join(format!("{:04}", day.year())).join("_v3"),
                hex(NfoZips::day_zip_name(day).as_bytes()),
            )),
            Self::Unset => None,
        }
    }

    /// What the ledger says of `day` against `listing`, its listing from the
    /// options source (`None` when the source holds no such day): one read
    /// of the manifest and of its ok marker, one sha256, one pass over the
    /// lines. Never an error: every fault is a named answer.
    #[must_use]
    pub fn check<L>(&self, day: Day, listing: Option<&NfoDay<L>>) -> V3 {
        let (Some(manifest), Some(marker)) = (self.manifest_path(day), self.marker_path(day))
        else {
            return V3::Absent {
                why: format!("{V3_LEDGER_VAR} unset"),
            };
        };
        let zip_name = NfoZips::day_zip_name(day);
        let unverified = |why: String| V3::Unverified { why };
        let bytes = match read_capped(&manifest, MANIFEST_MAX_BYTES) {
            Ok(bytes) => bytes,
            Err(Capped::Missing) => {
                return match read_capped(&marker, MARKER_MAX_BYTES) {
                    Err(Capped::Missing) => V3::Absent {
                        why: format!("no manifest for {zip_name}"),
                    },
                    _ => unverified(format!("an ok marker with no manifest for {zip_name}")),
                };
            }
            Err(why) => return unverified(format!("manifest {why}")),
        };
        let stated = match read_capped(&marker, MARKER_MAX_BYTES) {
            Ok(stated) => stated,
            Err(Capped::Missing) => return unverified("no ok marker".to_owned()),
            Err(why) => return unverified(format!("ok marker {why}")),
        };
        let marker = match verify_marker(&stated, &bytes, &zip_name) {
            Ok(marker) => marker,
            Err(why) => return unverified(why),
        };
        compare(&bytes, &marker, &zip_name, listing)
    }
}

/// Why a capped read gave no bytes.
enum Capped {
    /// The file does not exist.
    Missing,
    /// It holds more than the cap.
    Over(u64),
    /// It could not be read.
    Unreadable(std::io::ErrorKind),
}

impl core::fmt::Display for Capped {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing => f.write_str("missing"),
            Self::Over(cap) => write!(f, "over {cap} bytes"),
            Self::Unreadable(kind) => write!(f, "unreadable ({kind})"),
        }
    }
}

/// The bytes of `path`, opened read-only, at most `cap` of them.
fn read_capped(path: &Path, cap: u64) -> Result<Vec<u8>, Capped> {
    let unreadable = |why: std::io::Error| match why.kind() {
        std::io::ErrorKind::NotFound => Capped::Missing,
        kind => Capped::Unreadable(kind),
    };
    let file = std::fs::File::open(path).map_err(unreadable)?;
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > cap {
        return Err(Capped::Over(cap));
    }
    Ok(bytes)
}

/// The lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// The ok marker `stated`, held to the manifest `bytes` of the day zip
/// `zip_name`: its head, each of its keys once, its two names and the
/// manifest's sha256. Its `files` and `bytes` are held to the lines by
/// [`compare`], which counts them.
fn verify_marker(stated: &[u8], bytes: &[u8], zip_name: &str) -> Result<Marker, String> {
    let text = std::str::from_utf8(stated).map_err(|_| "ok marker is not UTF-8".to_owned())?;
    let mut lines = text.lines();
    if lines.next() != Some(MARKER_HEAD) {
        return Err(format!("ok marker does not start {MARKER_HEAD:?}"));
    }
    let mut values: [Option<&str>; MARKER_KEYS.len()] = [None; MARKER_KEYS.len()];
    for (at, line) in lines.enumerate() {
        let number = at + 2;
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("ok marker line {number} is not key=value"))?;
        let slot = MARKER_KEYS
            .iter()
            .position(|known| *known == key)
            .and_then(|index| values.get_mut(index))
            .ok_or_else(|| format!("ok marker line {number} names an unknown key {key:?}"))?;
        if slot.replace(value).is_some() {
            return Err(format!("ok marker states {key} twice"));
        }
    }
    let [name_hex, crc, size, sha, day_hex, files, total, run] = values;
    let (name_hex, crc, size, sha) = (
        need(name_hex, "name_hex")?,
        need(crc, "crc")?,
        need(size, "size")?,
        need(sha, "manifest_sha256")?,
    );
    let (day_hex, files, total) = (
        need(day_hex, "day_hex")?,
        need(files, "files")?,
        need(total, "bytes")?,
    );
    need(run, "run")?;
    if name_hex != hex(zip_name.as_bytes()) {
        return Err("ok marker names another day zip".to_owned());
    }
    let day_folder = zip_name.strip_suffix(".zip").unwrap_or(zip_name);
    if day_hex != hex(day_folder.as_bytes()) {
        return Err("ok marker names another day folder".to_owned());
    }
    if hex8(crc).is_none() {
        return Err("ok marker crc is not 8 hex digits".to_owned());
    }
    let decimal_of = |value: &str, key: &str| {
        decimal(value).ok_or_else(|| format!("ok marker {key} is not decimal"))
    };
    let marker = Marker {
        zip_size: decimal_of(size, "size")?,
        files: decimal_of(files, "files")?,
        bytes: decimal_of(total, "bytes")?,
    };
    if sha != format!("{:x}", Sha256::digest(bytes)) {
        return Err("manifest sha256 differs from its ok marker".to_owned());
    }
    Ok(marker)
}

/// The value an ok marker states for `key`, or why it states none.
fn need<'a>(value: Option<&'a str>, key: &str) -> Result<&'a str, String> {
    value.ok_or_else(|| format!("ok marker states no {key}"))
}

/// A decimal `u64` of ASCII digits only: no sign, no space, not empty.
fn decimal(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// A CRC-32 of exactly 8 hex digits.
fn hex8(text: &str) -> Option<u32> {
    if text.len() != 8 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(text, 16).ok()
}

/// Line `number` of a manifest, `text` without its newline, as 6 fields.
fn parse_line(text: &str, number: usize) -> Result<Line<'_>, V3> {
    let malformed = |why: String| V3::Malformed { line: number, why };
    let mut fields = text.split('\t');
    let (Some(path), Some(size), Some(crc), Some(fourth), Some(fifth), Some(source), None) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) else {
        let count = text.split('\t').count();
        return Err(malformed(format!("{count} tab fields, not 6")));
    };
    if path.is_empty() {
        return Err(malformed("an empty path".to_owned()));
    }
    let size = decimal(size).ok_or_else(|| malformed(format!("size {size:?} is not decimal")))?;
    let crc = hex8(crc).ok_or_else(|| malformed(format!("crc {crc:?} is not 8 hex digits")))?;
    // Fields 4 and 5: read, never interpreted (UNVERIFIED, design §3.11).
    for (name, value) in [("field 4", fourth), ("field 5", fifth)] {
        if decimal(value).is_none() {
            return Err(malformed(format!("{name} {value:?} is not decimal")));
        }
    }
    Ok(Line {
        path,
        size,
        crc,
        source,
    })
}

/// The trusted manifest `bytes` of `zip_name` against `listing`: one pass
/// over the lines, each held to the listing's entry at the same place.
fn compare<L>(bytes: &[u8], marker: &Marker, zip_name: &str, listing: Option<&NfoDay<L>>) -> V3 {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(why) => {
            let before = bytes.get(..why.valid_up_to()).unwrap_or_default();
            return V3::Malformed {
                line: before.split(|&byte| byte == b'\n').count(),
                why: "not UTF-8".to_owned(),
            };
        }
    };
    let month = zip_name.split('/').next().unwrap_or(zip_name);
    let mapped = |entry: &str| format!("{month}/{}", entry.replace('\\', "/"));
    let entries = listing.map_or(&[][..], NfoDay::entries);
    let mut listed = entries.iter();
    let (mut lines, mut files, mut total, mut count) = (0_usize, 0_u64, 0_u64, 0_usize);
    let mut first: Option<Difference> = None;
    // Only the first difference is built: a line that agrees allocates
    // nothing, and one that differs after the first only counts.
    let mut differ = |make: &dyn Fn() -> Difference| {
        count += 1;
        if first.is_none() {
            first = Some(make());
        }
    };
    let mut rest = text;
    while !rest.is_empty() {
        lines += 1;
        let Some((body, after)) = rest.split_once('\n') else {
            return V3::Malformed {
                line: lines,
                why: "torn: no newline at its end".to_owned(),
            };
        };
        rest = after;
        let line = match parse_line(body, lines) {
            Ok(line) => line,
            Err(malformed) => return malformed,
        };
        if !line.path.ends_with('/') {
            files += 1;
        }
        total = total.saturating_add(line.size);
        // At most one difference a line: its place first, then its size,
        // its CRC-32 and the day zip it names.
        let Some(entry) = listed.next() else {
            differ(&|| Difference {
                path: line.path.to_owned(),
                what: Differs::NotInListing,
            });
            continue;
        };
        let at = |what: Differs| Difference {
            path: line.path.to_owned(),
            what,
        };
        if !maps_to(month, &entry.entry, line.path) {
            differ(&|| Difference {
                path: mapped(&entry.entry),
                what: Differs::Path {
                    manifest: line.path.to_owned(),
                },
            });
        } else if entry.len != line.size {
            let (listed, manifest) = (entry.len, line.size);
            differ(&|| at(Differs::Size { listed, manifest }));
        } else if entry.crc32 != line.crc {
            let (listed, manifest) = (entry.crc32, line.crc);
            differ(&|| at(Differs::Crc { listed, manifest }));
        } else if line.source != zip_name {
            differ(&|| {
                at(Differs::Source {
                    manifest: line.source.to_owned(),
                })
            });
        }
    }
    for entry in listed {
        differ(&|| Difference {
            path: mapped(&entry.entry),
            what: Differs::NotInManifest,
        });
    }
    if let Some(listed) = listing.and_then(NfoDay::member_bytes)
        && listed != marker.zip_size
    {
        let ledger = marker.zip_size;
        differ(&|| Difference {
            path: zip_name.to_owned(),
            what: Differs::DayZipSize { listed, ledger },
        });
    }
    if (files, total) != (marker.files, marker.bytes) {
        return V3::Unverified {
            why: format!(
                "ok marker states {} files of {} bytes, the manifest {files} of {total}",
                marker.files, marker.bytes
            ),
        };
    }
    match first {
        None => V3::Agrees { lines },
        Some(first) => V3::Disagrees { count, first },
    }
}

/// Whether the listing entry `entry`, mapped as `<month>/` plus the entry
/// with every `\` turned to `/` (design §6.1, :2221), is the manifest's
/// `path`: compared byte for byte, nothing allocated.
fn maps_to(month: &str, entry: &str, path: &str) -> bool {
    let Some(rest) = path
        .strip_prefix(month)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return false;
    };
    rest.len() == entry.len()
        && rest
            .bytes()
            .zip(entry.bytes())
            .all(|(manifest, listed)| manifest == if listed == b'\\' { b'/' } else { listed })
}

#[cfg(test)]
#[path = "gdfl_v3_tests.rs"]
mod tests;
