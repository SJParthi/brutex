//! The vendor's loose day zips, the third GDFL source kind, `dayzips`
//! (D-4815, D-4834): standalone day zips as the vendor's daily download
//! leaves them, read in place for every kind. The operator's copy is the
//! 12 zips of 2026-09-25..30 (design §2.1), which the two archives do not
//! hold; this reader is what lets the import and the cold verify (V-2b) read
//! them beside the tick store's copy of the same days.
//!
//! # The layout, and nothing else (D-4834)
//!
//! `<ROOT>/{INDICES,STOCKS,OPTIONS}/<MON_yyyy>/<name>.zip`, where `<name>` is
//! the day's own folder name (`GFDLCM_INDICES_TICK_25092026`,
//! `GFDLCM_STOCK_TICK_25092026`, `GFDLNFO_TICK_25092026`). The root is walked
//! ONCE, when the source is opened, links never followed, at most
//! [`DAYZIPS_WALK_CAP`] entries; every name is filed in one table by kind and
//! day, so a day is one hash probe. Each day zip is opened through the
//! store's one reading door and read with the same central-directory walk
//! and member read as the archives ([`zip_entries`], [`member_bytes`]):
//! nothing is extracted.
//!
//! # The same rule for every copy of a day (D-4818)
//!
//! A day with two copies, wherever each lies (another month folder, a
//! `... (1).zip` beside it), is `archive_duplicate_day`; a lone copy under
//! another folder than its name's is `day_zip_misfiled`, naming where it was
//! found and where it belongs; a zip that is not one is refused for its day
//! by what is wrong with it. Each refuses that day only and never closes it
//! (`cm_cause`), so moving or removing the copy lets a re-run fill it. A name
//! with a day prefix whose date is not real, or outside the calendar's
//! measured span, is `day_zip_name_invalid` in the census, never filed. Every
//! other entry is a stray, counted, the first [`STRAY_NAMED`] named, never
//! opened.
//!
//! # Cost
//!
//! Opening: O(entries under the root), at most [`DAYZIPS_WALK_CAP`], once
//! per source (design §10.2 L-IA-8). A day: one hash probe, one open, one
//! central-directory read, O(its entries). A file: one local header and one
//! inflate, as the archives.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::calendar::DayKind;
use crate::gdfl_archive::{MONTHS, ZipLocator, member_bytes, zip_entries};
use crate::gdfl_cm::{CmKind, CmRefusal, CmScope, CmSource, DayListing, ListedFile};
use crate::gdfl_nfo::{NfoDay, NfoSource, ZipFileLocator};
use crate::session::Day;

/// The most entries the walk of a `dayzips` root visits (D-4834). The
/// operator's copy holds 12 zips in 9 folders; a vendor's daily download
/// adds three zips a day, so 100,000 is decades of them.
pub const DAYZIPS_WALK_CAP: usize = 100_000;

/// The days a filed name can carry: [`named`] files a day only inside the
/// calendar's sourced span, `SOURCED_FIRST_DAY..=SOURCED_LAST_DAY` (any other
/// day is `DayKind::Unmeasured` there, so `day_zip_name_invalid`), and that
/// span is 3,287 days (2018-01-01 to 2026-12-31). The assertion below fails
/// the build the day the span moves, so this bound is never stale.
const SOURCED_SPAN_DAYS: usize = 3_287;

// THE LITERAL AND AN ASSERTION, NO CAST (the calendar's own `DAYS` pattern):
// an `as usize` of the span would be a sign-losing cast on a path that never
// needs one.
const _: () =
    assert!(crate::calendar::SOURCED_LAST_DAY - crate::calendar::SOURCED_FIRST_DAY + 1 == 3_287);

/// The most keys a walk can file in its day table: one a kind a day of the
/// sourced span, 3 x 3,287 = 9,861 (BASE-GATE11, D-6624). The walk sizes the
/// table ONCE, before its first entry, for the smaller of this and its
/// entry cap (a key is at least one entry), so no insert rehashes (docs/07
/// law 2) and the table's room never depends on what the root holds.
pub const DAYZIPS_KEYS_MAX: usize = LooseKind::ALL.len() * SOURCED_SPAN_DAYS;

/// How many misfiled or invalid names the census names before it only
/// counts (D-4817: ≤ 16 named per reason).
pub const DAYZIPS_NAMED: usize = 16;

/// How many stray entries the census names (design §8 A19: ≤ 4 named in the
/// run line).
pub const STRAY_NAMED: usize = 4;

/// The three trees a `dayzips` root holds, by its folder names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LooseKind {
    /// `INDICES/`: the capital-market index day zips.
    Indices,
    /// `STOCKS/`: the capital-market share day zips.
    Stocks,
    /// `OPTIONS/`: the options day zips.
    Options,
}

impl LooseKind {
    /// Every kind, in the order the folders are named.
    pub const ALL: [Self; 3] = [Self::Indices, Self::Stocks, Self::Options];

    /// The kind's top folder under the root.
    #[must_use]
    pub const fn folder(self) -> &'static str {
        match self {
            Self::Indices => "INDICES",
            Self::Stocks => "STOCKS",
            Self::Options => "OPTIONS",
        }
    }

    /// The kind's day-folder prefix, as its own zips' names carry it.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Indices => CmKind::Indices.descriptor().folder_prefix,
            Self::Stocks => CmKind::Stocks.descriptor().folder_prefix,
            Self::Options => crate::gdfl_nfo::FOLDER_PREFIX,
        }
    }

    /// The loose kind of a capital-market tree.
    #[must_use]
    pub const fn of(kind: CmKind) -> Self {
        match kind {
            CmKind::Indices => Self::Indices,
            CmKind::Stocks => Self::Stocks,
        }
    }

    /// Where the day zip of this kind on `day` belongs, under the root:
    /// `INDICES/SEP_2026/GFDLCM_INDICES_TICK_25092026.zip`.
    #[must_use]
    pub fn path_of(self, day: Day) -> String {
        let month = MONTHS
            .get(usize::from(day.month()).saturating_sub(1))
            .copied()
            .unwrap_or_default();
        format!(
            "{}/{month}_{:04}/{}{:02}{:02}{:04}.zip",
            self.folder(),
            day.year(),
            self.prefix(),
            day.day(),
            day.month(),
            day.year()
        )
    }
}

/// What one file name is to the table.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Named {
    /// A kind's prefix, a real day inside the calendar's span, then `.zip`
    /// directly (`own`) or after anything else (a copy, `... (1).zip`).
    Day {
        kind: LooseKind,
        day: Day,
        own: bool,
    },
    /// A kind's prefix and `.zip` around a date that is not real or lies
    /// outside the calendar's span: `day_zip_name_invalid`.
    Invalid,
    /// Anything else.
    Stray,
}

/// What `name` is (design §8 A16 to A20). O(the name's length).
fn named(name: &str) -> Named {
    let Some(stem) = name.strip_suffix(".zip") else {
        return Named::Stray;
    };
    for kind in LooseKind::ALL {
        let Some(rest) = stem.strip_prefix(kind.prefix()) else {
            continue;
        };
        let Some((date, after)) = rest.split_at_checked(8) else {
            return Named::Invalid;
        };
        let Some(day) = crate::gdfl_cm::ddmmyyyy(date) else {
            return Named::Invalid;
        };
        // A name outside the calendar's measured span names no day an import
        // can ask for (D-4834): counted and named, never filed.
        if crate::calendar::exchange_kind_of(i64::from(day.days_from_epoch()))
            == DayKind::Unmeasured
        {
            return Named::Invalid;
        }
        return Named::Day {
            kind,
            day,
            own: after.is_empty(),
        };
    }
    Named::Stray
}

/// What the walk found beside the days it filed (D-4817): counted, the
/// first of each named. Report only: a misfiled or duplicated day is refused
/// by name on its own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayZipsCensus {
    /// Day zips at their own place, one per day.
    pub held: usize,
    /// Copies of a day elsewhere: another folder, or another name.
    pub misfiled: usize,
    /// The first misfiled copies, under the root.
    pub misfiled_named: Vec<String>,
    /// Day-zip names whose day is not real or outside the calendar's span.
    pub invalid: usize,
    /// The first invalid names, under the root.
    pub invalid_named: Vec<String>,
    /// Entries that are not day zips, never opened.
    pub stray: usize,
    /// The first stray entries, under the root.
    pub stray_named: Vec<String>,
}

/// Counts one name under `count`, naming it while fewer than `cap` are.
fn note(count: &mut usize, named: &mut Vec<String>, cap: usize, name: &str) {
    *count += 1;
    if named.len() < cap {
        named.push(name.to_owned());
    }
}

/// Every copy of one kind's day the walk found.
#[derive(Debug, Default, Clone)]
struct Copies {
    /// The copy at its own place, when there is one.
    own: Option<PathBuf>,
    /// The first copy anywhere else, under the root.
    elsewhere: Option<String>,
    /// How many copies, wherever they lie.
    count: usize,
}

/// The loose day zips under one root, walked once (D-4834).
#[derive(Debug, Clone)]
pub struct DayZips {
    root: PathBuf,
    days: HashMap<(LooseKind, Day), Copies>,
    census: DayZipsCensus,
    /// What every capital-market day listing files (D-3333).
    scope: Arc<CmScope>,
}

impl DayZips {
    /// Walks `root` once and files every loose day zip by kind and day.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveUnavailable`] when the root or a folder under it
    /// cannot be read, and [`CmRefusal::ArchiveMalformed`] when the root
    /// holds more than [`DAYZIPS_WALK_CAP`] entries (`dayzips_too_many_entries`
    /// in its words): no day is filed from a walk that did not finish.
    pub fn open(root: &Path) -> Result<Self, CmRefusal> {
        Self::open_capped(root, DAYZIPS_WALK_CAP)
    }

    /// [`DayZips::open`] with the walk stopped past `cap` entries rather
    /// than [`DAYZIPS_WALK_CAP`], so a test can reach the cap.
    pub(crate) fn open_capped(root: &Path, cap: usize) -> Result<Self, CmRefusal> {
        let unavailable = |why: std::io::Error| CmRefusal::ArchiveUnavailable { kind: why.kind() };
        // SIZED ONCE (D-6624): every key a walk of at most `cap` entries can
        // file, so the table never grows inside the walk.
        let mut days: HashMap<(LooseKind, Day), Copies> =
            HashMap::with_capacity(cap.min(DAYZIPS_KEYS_MAX));
        let mut census = DayZipsCensus::default();
        let mut stack = vec![(root.to_path_buf(), String::new())];
        let mut visited = 0_usize;
        while let Some((dir, rel)) = stack.pop() {
            for entry in std::fs::read_dir(&dir).map_err(unavailable)? {
                visited += 1;
                if visited > cap {
                    return Err(CmRefusal::ArchiveMalformed {
                        what: TOO_MANY_ENTRIES,
                    });
                }
                let entry = entry.map_err(unavailable)?;
                let kind = entry.file_type().map_err(unavailable)?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = if rel.is_empty() {
                    name.clone()
                } else {
                    format!("{rel}/{name}")
                };
                if kind.is_dir() && !name.starts_with('.') {
                    stack.push((entry.path(), path));
                    continue;
                }
                // A link, a pipe or a device at a day zip's name is still that
                // day's copy: the reading door refuses it by name when the day is
                // asked, so the day is never read as absent (FM-046's rule).
                match named(&name) {
                    Named::Day { kind, day, own } => {
                        let copies = days.entry((kind, day)).or_default();
                        copies.count += 1;
                        if own && path == kind.path_of(day) {
                            census.held += 1;
                            copies.own = Some(entry.path());
                        } else {
                            note(
                                &mut census.misfiled,
                                &mut census.misfiled_named,
                                DAYZIPS_NAMED,
                                &path,
                            );
                            let _first = copies.elsewhere.get_or_insert(path);
                        }
                    }
                    Named::Invalid => note(
                        &mut census.invalid,
                        &mut census.invalid_named,
                        DAYZIPS_NAMED,
                        &path,
                    ),
                    Named::Stray => note(
                        &mut census.stray,
                        &mut census.stray_named,
                        STRAY_NAMED,
                        &path,
                    ),
                }
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            days,
            census,
            scope: CmScope::swept_shared(),
        })
    }

    /// This source, its capital-market listings filing what `scope` files
    /// (D-3333).
    #[must_use]
    pub fn with_scope(self, scope: Arc<CmScope>) -> Self {
        Self { scope, ..self }
    }

    /// The root walked.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What the walk found beside the days it filed.
    #[must_use]
    pub const fn census(&self) -> &DayZipsCensus {
        &self.census
    }

    /// The day zip of `kind` on `day`, opened: `None` when the root holds no
    /// copy of it. One hash probe, then one open through the store's one
    /// reading door.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveDuplicateDay`] for two copies,
    /// [`CmRefusal::DayZipMisfiled`] for a lone copy elsewhere, and the
    /// refusals of opening it.
    fn zip(
        &self,
        kind: LooseKind,
        day: Day,
    ) -> Result<Option<(Arc<std::fs::File>, u64)>, CmRefusal> {
        let Some(copies) = self.days.get(&(kind, day)) else {
            return Ok(None);
        };
        if copies.count > 1 {
            return Err(CmRefusal::ArchiveDuplicateDay { day });
        }
        match (&copies.own, &copies.elsewhere) {
            (Some(path), _) => {
                let (file, len) = open_zip(path)?;
                Ok(Some((Arc::new(file), len)))
            }
            (None, Some(found)) => Err(CmRefusal::DayZipMisfiled {
                found: found.clone(),
                expected: kind.path_of(day),
            }),
            (None, None) => Ok(None),
        }
    }
}

/// The walk of a root visited more than [`DAYZIPS_WALK_CAP`] entries.
const TOO_MANY_ENTRIES: &str = "dayzips_too_many_entries: the root holds more than 100,000 \
                                entries, so it is not a directory of loose day zips; no day \
                                is filed from a walk that did not finish";

/// A day zip's name at open is not a regular file.
const NOT_A_ZIP_FILE: &str = "a loose day zip's name is not a regular file (a link, a pipe, a \
                              device or a directory), which the reader never opens";

/// Opens a loose day zip through the store's one reading door
/// (`store::safe_open::open_read_len`): a link, a pipe or a device at its
/// name is refused, never followed or waited on; a zip gone since the walk
/// is `archive_unavailable` (not found), never a day not held.
fn open_zip(path: &Path) -> Result<(std::fs::File, u64), CmRefusal> {
    store::safe_open::open_read_len(path).map_err(|why| match why {
        store::safe_open::ReadRefusal::Io { source, .. } => CmRefusal::ArchiveUnavailable {
            kind: source.kind(),
        },
        _ => CmRefusal::ArchiveMalformed {
            what: NOT_A_ZIP_FILE,
        },
    })
}

/// Where one member of a loose capital-market day zip lies.
#[derive(Debug, Clone)]
pub struct LooseLocator {
    file: Arc<std::fs::File>,
    at: ZipLocator,
}

impl CmSource for DayZips {
    type Locator = LooseLocator;

    fn scope(&self) -> Arc<CmScope> {
        Arc::clone(&self.scope)
    }

    /// The day zip's central directory, every entry in its order with the
    /// size and CRC-32 it states. `None` when the root holds no copy.
    ///
    /// # Errors
    ///
    /// The refusals of [`DayZips`]'s day lookup and of [`zip_entries`].
    fn day(&self, kind: CmKind, day: Day) -> Result<Option<DayListing<LooseLocator>>, CmRefusal> {
        let Some((file, len)) = self.zip(LooseKind::of(kind), day)? else {
            return Ok(None);
        };
        let entries = zip_entries(&*file, 0, len)?;
        let mut listing =
            DayListing::scoped(kind, day, Arc::clone(&self.scope)).with_member_bytes(len);
        for entry in entries {
            listing.push(
                &entry.name,
                entry.len,
                entry.crc32,
                LooseLocator {
                    file: Arc::clone(&file),
                    at: entry.locator,
                },
            );
        }
        Ok(Some(listing))
    }

    /// The member's bytes, as the archives read them.
    ///
    /// # Errors
    ///
    /// Every refusal of [`member_bytes`].
    fn fetch(
        &self,
        _listing: &DayListing<LooseLocator>,
        file: &ListedFile<LooseLocator>,
    ) -> Result<Vec<u8>, CmRefusal> {
        member_bytes(&*file.locator.file, file.locator.at, &file.entry, file.len)
    }
}

impl NfoSource for DayZips {
    type Locator = ZipFileLocator;

    /// The options day zip's central directory. `None` when the root holds
    /// no copy.
    ///
    /// # Errors
    ///
    /// The refusals of [`DayZips`]'s day lookup and of [`zip_entries`].
    fn day(&self, day: Day) -> Result<Option<NfoDay<ZipFileLocator>>, CmRefusal> {
        let Some((file, len)) = self.zip(LooseKind::Options, day)? else {
            return Ok(None);
        };
        let entries = zip_entries(&*file, 0, len)?;
        let mut listing = NfoDay::new(day, entries.len()).with_member_bytes(len);
        for entry in entries {
            listing.push(
                &entry.name,
                entry.len,
                entry.crc32,
                ZipFileLocator::new(Arc::clone(&file), entry.locator),
            );
        }
        Ok(Some(listing))
    }

    fn fetch(&self, file: &ListedFile<ZipFileLocator>) -> Result<Vec<u8>, CmRefusal> {
        file.locator.bytes(&file.entry, file.len)
    }
}

#[cfg(test)]
#[path = "gdfl_dayzips_tests.rs"]
mod tests;
