//! The census as it is on disk right now: the per-request read, its cache
//! keyed on what each vendor manifest's `stat` said, and the stamps that make
//! a cached census trustworthy.
//!
//! [`vendor_census`] and [`VendorCensus::rows_of`](crate::census::VendorCensus)
//! are the two interfaces the later census work (WP-30, WP-37) codes against:
//! one vendor's census, and one O(1) probe of it that keeps "this census does
//! not hold the month" apart from "there is no census to ask". The probe is
//! the one `Manifest::entry` lookup `rows_for` makes, which the census rows of
//! `crates/api/benches/ratio.rs` time flat from 1,000 to 100,000 entries.
//!
//! # Warm at start, re-read in the background (R3-09)
//!
//! The server calls [`start`] once it listens: every vendor's census, and
//! for each offered feed the facet index, the index map and the import
//! journal's count, are built on a thread of their own, each listed by name
//! on `/census/warm.json` as `warming`, then `ready` (or `failed`, with the
//! reason), with the time its build took and the census generation it stood
//! on; every build is also a line in the log (`api.census`, "cache built").
//! From then on a request whose vendor's manifest moved -- an import day
//! closed, a pull committed, a compaction, a repair -- is NOT made to read
//! it: it is answered from the kept read, and says so on every census
//! answer (`x-brutex-census-refreshing: 1`, `x-brutex-census-age-ms`), while
//! one background thread per vendor reads the change (by its tail where it
//! can) and builds the derived answers again, then serves it current. A
//! vendor with no kept read at all (a request before the warm reached it)
//! waits for that one read rather than repeating it. DB-G80, RA-032,
//! RA-046, RA-093, RA-094. What stays on a request thread: a facet index or
//! index map a request reaches in the moment between a re-read landing and
//! the background build over it (whichever thread takes the lock first
//! builds it; moved by the appended entries, or, after a new epoch, a
//! compaction or a rename, built over the segment's keys in memory) --
//! never a manifest read. Named for `docs/06-limits.md`.
//!
//! # Moved, not rewritten
//!
//! Every item below except those two interfaces -- [`VendorCensusNow`],
//! [`vendor_census`], [`RowsOf`] and the `rows_of` impl, which are new -- was
//! cut out of `server.rs` verbatim by the WP-01 seam and is mounted there as a
//! child module, so it reads that file's private helpers through `super::`
//! exactly as it did when it lived in it, and `server.rs` imports each name
//! back. The only edits to the moved items are visibility qualifiers
//! (`pub(super)`) on items another module now has to reach. No route answers
//! differently because of the move.

use super::Site;
use crate::census;
use brutex_core::vendor::Vendor;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

/// What `Site::census` holds: the last read of every vendor together, and the
/// per-vendor cache every read is made through. See [`Cached`].
///
/// A `Mutex<Option<_>>` so `Site::new` starts it as `Mutex::new(None)`; the
/// first reader fills it.
pub type CensusCache = Mutex<Option<Cached>>;

/// The census cache: one entry per vendor, keyed on that vendor's own
/// manifest stamp, and the all-vendor read assembled from them. D-3327 item 1.
///
/// # Why per vendor (DB-059, DB-G06, DB-G79)
///
/// This was ONE entry keyed on every vendor's stamp together. The Zerodha
/// minute pull rewrites `zerodha.man` every 0.5 to 2 s, so every such commit
/// made the next reader of ANY feed re-read every manifest, `gdfl.man`'s
/// 268 MB among them, re-sort every held entry, and re-encode and re-hash a
/// 21 MB `/store.json?feed=gdfl` body whose census had not changed: p50
/// 700 ms, p99 816 ms, measured during the pull on 6 Oct 2026. Now:
///
/// * each vendor has its own cell ([`Vendors`]), keyed on its own manifest's
///   stamp, so a Zerodha commit leaves the GDFL cell a hit;
/// * a cell that misses is rebuilt by ONE reader while the others wait for
///   it (single-flight), and the rebuild extends the previous census by the
///   appended entries alone wherever that reaches the same census
///   ([`census::read_vendor_after`]);
/// * the all-vendor read every older caller takes ([`census_now`]) is
///   assembled from the cells, so its miss costs the changed vendor's tail
///   and a few reference counts, and the merged entry list behind it is
///   sorted only when a caller reads it ([`HeldEntries`]).
#[derive(Debug, Default)]
pub struct Cached {
    /// The last all-vendor read, the stamps it was read under, and its
    /// entries. What every [`census_now`] caller is handed.
    every: Option<(
        CensusStamps,
        Arc<Vec<census::VendorCensus>>,
        Arc<HeldEntries>,
    )>,
    /// One cell per vendor.
    vendors: Arc<Vendors>,
}

impl Cached {
    /// The stamps the all-vendor read was kept under, if one was kept.
    #[cfg(test)]
    pub(crate) fn kept_stamps(&self) -> Option<CensusStamps> {
        self.every.as_ref().map(|(stamps, _, _)| stamps.clone())
    }

    /// Whether `vendor`'s own cell keeps a read.
    #[cfg(test)]
    pub(crate) fn keeps_vendor(&self, vendor: Vendor) -> bool {
        self.vendors
            .cells
            .get(vendor as usize)
            .is_some_and(|cell| cell.current().is_some())
    }

    /// A cache holding exactly this all-vendor read under `stamps`, and no
    /// vendor cell: what a test plants to pin what a stale key serves.
    #[cfg(test)]
    pub(crate) fn planted(
        stamps: CensusStamps,
        censuses: Arc<Vec<census::VendorCensus>>,
        entries: Arc<HeldEntries>,
    ) -> Self {
        Self {
            every: Some((stamps, censuses, entries)),
            vendors: Arc::default(),
        }
    }
}

/// Every vendor's cell, indexed by `vendor as usize`.
#[derive(Debug, Default)]
pub(crate) struct Vendors {
    cells: [Cell; Vendor::ALL.len()],
    /// Set once, by [`start`]: the site a background re-read runs for. While
    /// it is unset a reader whose vendor's manifest moved re-reads it itself,
    /// as every reader did before R3-09 (one-shot callers and the tests that
    /// pin a write being seen by the very next read); once it is set, that
    /// reader is answered from the kept census, with its age, and the re-read
    /// runs on a thread of its own ([`refresh_in_background`]). R3-09,
    /// DB-G80, RA-032.
    background: OnceLock<std::sync::Weak<Site>>,
    /// Set once, by [`start`]: the feeds whose derived answers (the facet
    /// index, the index map, the import journal's count) are built by the
    /// warm and by every background re-read. Every vendor's census itself is
    /// warmed and re-read whatever this holds.
    derived: OnceLock<Vec<Vendor>>,
    /// What every cache [`start`] warms says of itself, by name and feed:
    /// warming, ready, refreshing or failed. What `/census/warm.json` lists.
    warmth: Mutex<Vec<Warmth>>,
    /// Set while [`start`]'s warm thread runs: from just before it is
    /// spawned until it ends (by return or by panic). What [`warming_since`]
    /// asks, so a cache whose warm thread never started or died is never
    /// reported warming for the life of the process.
    warm_running: std::sync::atomic::AtomicBool,
}

/// One vendor's cache: its last kept census, the lock one rebuild holds, and
/// whether a background re-read of it is running.
#[derive(Debug, Default)]
struct Cell {
    slot: Mutex<Option<Kept>>,
    build: Mutex<()>,
    /// Set while a background re-read of this vendor is queued or running,
    /// so a burst of readers that all see the moved stamp starts ONE.
    refreshing: std::sync::atomic::AtomicBool,
}

/// A kept read: the slot, and the stamp it is kept under.
///
/// The stamp sits beside the slot rather than in it, so a re-read that finds
/// the census unchanged (a stamp that moved with no entry appended: a
/// `touch`, a `chmod`, a header rewritten to the same counters) keeps the
/// SAME slot under the new stamp, and every answer derived from the slot
/// (the encoded bodies, the facet index, the audit rollup) stays kept.
#[derive(Debug, Clone)]
struct Kept {
    stamp: ManifestStamp,
    root_is_dir: bool,
    slot: Arc<Slot>,
    /// When the read this slot stands for began: the census is no older than
    /// the manifest as it was then. What a stale answer's age is counted from.
    read_at: std::time::SystemTime,
    /// How long that read took, in milliseconds, on whichever thread made it:
    /// what `/census/warm.json` reports as the census's build time (R3-09).
    read_ms: u64,
}

/// Milliseconds since `began`, saturated.
fn ms_since(began: std::time::Instant) -> u64 {
    u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// How current the census a reader was handed is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Freshness {
    /// The kept census was read under the stamp this reader just took: it is
    /// the census on disk.
    Current,
    /// The manifest moved since the kept census was read, and the reader was
    /// answered from it while a background re-read runs. `read_at` is when
    /// that read began; the stamp it was read under travels beside it
    /// ([`vendor_slot_fresh`]), so this stays a small `Copy` value.
    Stale { read_at: std::time::SystemTime },
}

impl Freshness {
    /// How far behind the disk this answer may be, in milliseconds: zero for
    /// a current census, else the time since the served read began. An upper
    /// bound: the manifest moved at some instant after that read.
    pub(crate) fn age_ms(self) -> u64 {
        match self {
            Self::Current => 0,
            Self::Stale { read_at, .. } => {
                u64::try_from(read_at.elapsed().unwrap_or_default().as_millis()).unwrap_or(u64::MAX)
            }
        }
    }

    /// Whether the answer was served from the previous read while a newer one
    /// is being read.
    pub(crate) const fn is_stale(self) -> bool {
        matches!(self, Self::Stale { .. })
    }
}

impl Cell {
    fn current(&self) -> Option<Kept> {
        self.slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// One vendor's census as one read left it.
///
/// Shared, never copied: every reader of an unchanged vendor is handed the
/// same `Arc`, and the derived answers (the sorted entries here, the facet
/// index and the encoded bodies elsewhere) are kept against it. A new slot
/// is made only for a census that differs from the kept one
/// ([`census::VendorCensus::same_read`]).
#[derive(Debug)]
pub(crate) struct Slot {
    census: census::VendorCensus,
    entries: OnceLock<Arc<Vec<(census::Series, store::path::YearMonth)>>>,
    /// When the read this slot holds began: an answer built from it while a
    /// newer read's index is built is no older than this (R3-09).
    read_at: std::time::SystemTime,
}

impl Slot {
    /// A slot no cell keeps, over `census`: what a test of a cache keyed on
    /// slots stands one read up with.
    #[cfg(test)]
    pub(crate) fn detached(census: census::VendorCensus) -> Arc<Self> {
        Arc::new(Self {
            census,
            entries: OnceLock::new(),
            read_at: std::time::SystemTime::now(),
        })
    }

    /// When the read this slot holds began.
    pub(crate) const fn read_at(&self) -> std::time::SystemTime {
        self.read_at
    }

    /// The census this slot holds.
    pub(crate) const fn census(&self) -> &census::VendorCensus {
        &self.census
    }

    /// This vendor's held entries, newest month first, sorted once per slot
    /// and shared after that.
    pub(crate) fn entries(&self) -> Arc<Vec<(census::Series, store::path::YearMonth)>> {
        Arc::clone(
            self.entries
                .get_or_init(|| Arc::new(census::held_entries(std::slice::from_ref(&self.census)))),
        )
    }
}

/// The merged held entries of every vendor, sorted the first time a caller
/// reads them and never again for the same read.
///
/// Most [`census_now`] callers never read them (`/feeds.json`, `/audit.json`,
/// `/calendar.json`, the pull's row count), and sorting every vendor's
/// entries on every miss was part of what a Zerodha commit cost them.
#[derive(Debug)]
pub struct HeldEntries {
    censuses: Arc<Vec<census::VendorCensus>>,
    sorted: OnceLock<Vec<(census::Series, store::path::YearMonth)>>,
}

impl HeldEntries {
    fn of(censuses: &Arc<Vec<census::VendorCensus>>) -> Self {
        Self {
            censuses: Arc::clone(censuses),
            sorted: OnceLock::new(),
        }
    }
}

impl std::ops::Deref for HeldEntries {
    type Target = Vec<(census::Series, store::path::YearMonth)>;

    fn deref(&self) -> &Self::Target {
        self.sorted
            .get_or_init(|| census::held_entries(&self.censuses))
    }
}

impl PartialEq for HeldEntries {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

/// What [`census_now`] hands back: every vendor's census and their merged
/// entries, shared not copied.
///
/// Named SEPARATELY from [`CensusCache`] because the cache also carries the
/// stamps that make these two trustworthy, and a caller receiving them without
/// the stamps cannot tell a current answer from one taken before the last
/// pull. Two names, two meanings.
pub type CensusNow = (Arc<Vec<census::VendorCensus>>, Arc<HeldEntries>);

/// The census as it is on disk RIGHT NOW, not as it was at startup.
///
/// # Why this is read per request
///
/// `Site` loads the census once when the process starts and holds it. That was
/// right when a pull was a separate invocation, and became wrong the moment the
/// served binary could pull: a run through this process writes 772 bar files
/// and 278,112 records, and every page went on showing the startup snapshot.
/// An operator watching a backfill saw nothing move, which reads as a broken
/// run rather than a stale cache.
///
/// Each request stats every vendor's manifest; a vendor whose stamp moved is
/// re-read through its own cell ([`Cached`]), and only that vendor.
pub(crate) fn census_now(site: &Site) -> CensusNow {
    census_now_stamped(site).1
}

/// [`census_now`], and the stamps that census is no older than.
///
/// # Why a caller needs the stamps
///
/// On a miss they are the stamps taken BEFORE the read, and on a hit the ones
/// just taken, which equal the key the census was kept under. Either way the
/// census was read no earlier than they were taken.
///
/// `calendar_of::cached` keeps a calendar derived from a census's months, and
/// it took its key itself: one `stat` of the manifest, AFTER the caller's
/// census had been read. A pull that installed a manifest between the two had
/// the calendar derived from the OLDER census's months kept under the NEWER
/// modified time, and every later request hit it until the manifest was next
/// written, which can be the next day's pull. That is the order
/// [`census_now_reading`] refuses for the census itself. So the three callers
/// that derive a calendar take the census through here and hand `cached` the
/// feed's modified time from these stamps ([`CensusStamps::modified`]). A
/// manifest installed after them moves the next request's stamp, which then
/// misses. D-0695.
///
/// # They come with a census the cache refused, too
///
/// A read [`read_as_stamped`] refuses to keep is still this request's answer,
/// and these are still the stamps taken before it, which that read
/// contradicts: a store root gone after `read_all`'s own check reads every
/// feed "absent" under a stamp of a manifest that exists. (Since D-0695's
/// eighth repair that row is handed to the request unreadable, not absent:
/// see [`refuse_contradicted_absences`].) A key taken from them for such a
/// row kept the calendar derived from it under a time the fault had not
/// moved, and served it after the fault had ended, until the manifest was
/// next written. So [`CensusStamps::modified`] takes the row as well, and
/// hands a time only for one its stamp could have read. D-0695.
pub(crate) fn census_now_stamped(site: &Site) -> (CensusStamps, CensusNow) {
    census_now_kept(site, manifest_stamps, |root| {
        // THE STAMP EACH ROW WAS READ UNDER, where that is not the one just
        // taken: a vendor answered from its previous read while a background
        // re-read runs (R3-09). The all-vendor read is then kept under the
        // stamps its rows were read under, never under newer ones it does
        // not reflect, or it would be served stale after the re-read landed.
        let mut kept_under: Option<Vec<(usize, ManifestStamp)>> = None;
        let rows = census::read_all_through(root, |vendor| {
            let (slot, _freshness, kept) =
                vendor_slot_fresh(site, vendor, vendor_stamp(root, vendor), true);
            if let Some(stamp) = kept {
                kept_under
                    .get_or_insert_with(Vec::new)
                    .push((vendor as usize, stamp));
            }
            slot.census.clone()
        });
        (rows, kept_under)
    })
}

/// The vendors' cells of `site`, made on first use.
fn vendors_of(site: &Site) -> Arc<Vendors> {
    let mut held = site.census.lock().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(&held.get_or_insert_with(Cached::default).vendors)
}

/// One vendor's census, read through its own cell.
///
/// A hit is one lock and a reference count. A miss takes the cell's build
/// lock, so concurrent readers of one changed vendor wait for one rebuild
/// rather than each repeating it, and checks again under it: the reader that
/// rebuilt has kept a slot every waiter then hits. The rebuild extends the
/// previous census where it can ([`census::read_vendor_after`]). A census
/// `stamp` could not have read ([`stamp_could_read`]) is handed to this
/// reader and not kept, exactly as the all-vendor read is not (D-0695).
///
/// # Answered from the kept read while a background re-read runs (R3-09)
///
/// Once [`start`] has run, a miss on a vendor that HAS a kept read is not
/// rebuilt by the reader: the reader is handed the kept slot, marked
/// [`Freshness::Stale`] with the time it was read at (and the stamp it was
/// read under beside it, the third value), and the
/// re-read runs on a thread of its own ([`refresh_in_background`]). So no
/// request thread reads a manifest that changed -- whole after a restart, a
/// compaction or a tail past its floor, or by its tail after an import day
/// -- and a cheap route never waits behind one (DB-G80: `/feeds.json` stalled
/// 550 ms behind a census miss on another connection). A vendor with no kept
/// read at all (before the warm at start has read it) is still read by the
/// first reader, under the single-flight lock the warm holds while it reads,
/// so it waits for that read rather than repeating it.
fn vendor_slot_fresh(
    site: &Site,
    vendor: Vendor,
    stamp: ManifestStamp,
    root_is_dir: bool,
) -> (Arc<Slot>, Freshness, Option<ManifestStamp>) {
    let vendors = vendors_of(site);
    let Some(cell) = vendors.cells.get(vendor as usize) else {
        // `Vendor::ALL` sizes the array and `vendor as usize` indexes it, so
        // this cannot be reached; a read that keeps nothing is still right.
        return (
            Arc::new(Slot {
                read_at: std::time::SystemTime::now(),
                census: fresh_read(&site.store_root, vendor, None),
                entries: OnceLock::new(),
            }),
            Freshness::Current,
            None,
        );
    };
    let matches = |kept: &Kept| kept.stamp == stamp && kept.root_is_dir == root_is_dir;
    let current = cell.current();
    if let Some(kept) = current.as_ref().filter(|kept| matches(kept)) {
        return (Arc::clone(&kept.slot), Freshness::Current, None);
    }
    if let Some(kept) = current
        && refresh_in_background(&vendors, vendor)
    {
        return (
            kept.slot,
            Freshness::Stale {
                read_at: kept.read_at,
            },
            Some(kept.stamp),
        );
    }
    (
        rebuild(&site.store_root, cell, vendor, stamp, root_is_dir),
        Freshness::Current,
        None,
    )
}

/// One vendor's census read from disk, extending `previous` where that
/// reaches the same census, or the root's refusal for that vendor.
fn fresh_read(
    root: &std::path::Path,
    vendor: Vendor,
    previous: Option<&census::VendorCensus>,
) -> census::VendorCensus {
    #[cfg(test)]
    slow_read::pause(root);
    census::root_refusal(root).map_or_else(
        || census::read_vendor_after(root, vendor, previous),
        |why| census::unreadable_root_row(root, vendor, &why),
    )
}

/// The one rebuild of `cell` under `stamp`: the single-flight lock taken, the
/// kept read handed back if a rebuild that held it first reached `stamp`,
/// else read (extending the kept read where it can) and kept if `stamp` could
/// have read it. What the first reader of a cold vendor runs, and what a
/// background re-read runs.
fn rebuild(
    root: &std::path::Path,
    cell: &Cell,
    vendor: Vendor,
    stamp: ManifestStamp,
    root_is_dir: bool,
) -> Arc<Slot> {
    let matches = |kept: &Kept| kept.stamp == stamp && kept.root_is_dir == root_is_dir;
    let _single_flight = cell.build.lock().unwrap_or_else(PoisonError::into_inner);
    let previous = cell.current();
    if let Some(kept) = previous.as_ref().filter(|kept| matches(kept)) {
        return Arc::clone(&kept.slot);
    }
    let read_at = std::time::SystemTime::now();
    let began = std::time::Instant::now();
    let census = fresh_read(
        root,
        vendor,
        previous.as_ref().map(|kept| &kept.slot.census),
    );
    let read_ms = ms_since(began);
    // THE SAME READ IS THE SAME SLOT: a stamp that moved with nothing
    // appended keeps every answer derived from the kept slot.
    let slot = match previous {
        Some(kept) if kept.slot.census.same_read(&census) => kept.slot,
        _ => Arc::new(Slot {
            census,
            entries: OnceLock::new(),
            read_at,
        }),
    };
    if stamp_could_read(stamp, root_is_dir, &slot.census.state) {
        *cell.slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(Kept {
            stamp,
            root_is_dir,
            slot: Arc::clone(&slot),
            read_at,
            read_ms,
        });
    }
    slot
}

/// How long the read `cell` keeps took, or zero when it keeps none.
fn kept_read_ms(cell: &Cell) -> u64 {
    cell.current().map_or(0, |kept| kept.read_ms)
}

/// Clears a cell's `refreshing` flag when the background re-read ends, by
/// return or by panic, so a re-read that died never leaves the vendor served
/// stale for the life of the process: the next reader starts another.
struct Refreshing<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for Refreshing<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Starts a background re-read of `vendor` unless one is already running,
/// and says whether the reader may be answered from the kept read: `true`
/// when a re-read is running (this one or an earlier one), `false` when the
/// cache is not in background mode or the thread could not be started -- the
/// reader then reads itself, which is slow and never wrong.
///
/// The thread loops until the vendor's stamp holds still across one re-read,
/// so a manifest that moved again while it read is read again before the
/// thread ends; each pass is the tail appended since the last (or the whole
/// file when the tail cannot reach the same census), then the answers
/// derived from it ([`warm_derived`]), all off every request.
fn refresh_in_background(vendors: &Arc<Vendors>, vendor: Vendor) -> bool {
    use std::sync::atomic::Ordering;
    let Some(site) = vendors.background.get().and_then(std::sync::Weak::upgrade) else {
        return false;
    };
    let Some(cell) = vendors.cells.get(vendor as usize) else {
        return false;
    };
    if cell.refreshing.swap(true, Ordering::AcqRel) {
        return true;
    }
    let mine = Arc::clone(vendors);
    let spawned = std::thread::Builder::new()
        .name(format!("brutex-census-{}", vendor.as_str()))
        .spawn(move || {
            let Some(cell) = mine.cells.get(vendor as usize) else {
                return;
            };
            let _clear = Refreshing(&cell.refreshing);
            note_warmth(&mine, CENSUS, vendor, WarmState::Refreshing, None);
            loop {
                let (stamp, root_is_dir) = read_census(&site, &mine, cell, vendor);
                let again = manifest_stamps(&site.store_root);
                if again.manifests.get(vendor as usize) == Some(&stamp)
                    && again.root_is_dir == root_is_dir
                {
                    break;
                }
            }
        });
    if let Err(why) = spawned {
        cell.refreshing.store(false, Ordering::Release);
        note_warmth(
            vendors,
            CENSUS,
            vendor,
            WarmState::Failed,
            Some(format!(
                "the background re-read could not be started ({why}); readers read it themselves"
            )),
        );
        return false;
    }
    true
}

/// One vendor's census as the cache holds it right now: what
/// [`vendor_census`] hands back beside the stamps it was read under.
///
/// # A handle, not a copy
///
/// A held census shares its manifest's entry index behind an `Arc`, so the
/// row here is a few reference counts, and the slot beside it is what the
/// derived caches (the sorted entries, the facet index, the encoded bodies)
/// are kept against.
#[derive(Debug)]
pub(crate) struct VendorCensusNow {
    /// The vendor's row, as this request is to be answered.
    census: census::VendorCensus,
    /// The cell's slot it came from.
    slot: Arc<Slot>,
    /// Whether that slot is the census on disk, or the previous read served
    /// while a background re-read runs (R3-09).
    freshness: Freshness,
}

impl VendorCensusNow {
    /// The vendor's census row.
    ///
    /// Always `Some` since the per-vendor cache (D-3327): the row is read for
    /// the vendor asked. The `Option` is kept so a caller written against the
    /// WP-01 interface does not change.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "the WP-01 signature every caller was written against; always `Some` now"
    )]
    pub(crate) const fn census(&self) -> Option<&census::VendorCensus> {
        Some(&self.census)
    }

    /// The slot the row came from: one per kept read of this vendor.
    pub(crate) const fn slot(&self) -> &Arc<Slot> {
        &self.slot
    }

    /// Whether the row is the census on disk or the previous read, and how
    /// old (R3-09). Every answer built from it says so in
    /// [`freshness_headers`].
    pub(crate) const fn freshness(&self) -> Freshness {
        self.freshness
    }
}

/// `x-brutex-census-age-ms`: how far behind the manifest on disk the census
/// an answer was built from may be, in milliseconds; `0` when it is the
/// census on disk. Never absent on a census-backed answer, so a reader never
/// has to infer currency from a missing header (R3-09).
pub(crate) const CENSUS_AGE_HEADER: &str = "x-brutex-census-age-ms";

/// `x-brutex-census-refreshing`: `1` when the answer was built from the
/// previous read while a background re-read runs, `0` otherwise.
pub(crate) const CENSUS_REFRESHING_HEADER: &str = "x-brutex-census-refreshing";

/// The two headers above, for `freshness`, added to `headers`.
pub(crate) fn freshness_headers(headers: &mut axum::http::HeaderMap, freshness: Freshness) {
    let _age = headers.insert(
        axum::http::HeaderName::from_static(CENSUS_AGE_HEADER),
        axum::http::HeaderValue::from(freshness.age_ms()),
    );
    let _refreshing = headers.insert(
        axum::http::HeaderName::from_static(CENSUS_REFRESHING_HEADER),
        axum::http::HeaderValue::from_static(if freshness.is_stale() { "1" } else { "0" }),
    );
}

/// One vendor's census, read through ITS OWN cell and nothing else, and the
/// stamps it is no older than.
///
/// # What a caller of one feed pays (D-3327 item 1)
///
/// One `stat` per vendor (the stamps, so a calendar kept from this row is
/// kept under its own manifest's time, [`CensusStamps::modified`]), and this
/// vendor's cell: a hit, or this vendor's rebuild alone. Another vendor's
/// commit does not move this vendor's stamp, so `/store.json?feed=gdfl` is a
/// hit while Zerodha's pull commits. A row its stamp contradicts is handed
/// over unreadable, by the same rule the all-vendor read applies
/// ([`refuse_contradicted_absences`]).
///
/// # A row served while its re-read runs (R3-09)
///
/// The stamps handed back are the ones the row was READ under: for a row
/// served from the previous read ([`Freshness::Stale`]) that vendor's stamp
/// is the kept one, never the newer stamp it does not reflect, so a calendar
/// kept from it is kept under the time it stands for (D-0695).
pub(crate) fn vendor_census(site: &Site, vendor: Vendor) -> (CensusStamps, VendorCensusNow) {
    let mut stamps = manifest_stamps(&site.store_root);
    let stamp = stamps
        .manifests
        .get(vendor as usize)
        .copied()
        .unwrap_or(ManifestStamp::Missing);
    let (slot, freshness, kept) = vendor_slot_fresh(site, vendor, stamp, stamps.root_is_dir);
    if let Some(kept) = kept
        && let Some(at) = stamps.manifests.get_mut(vendor as usize)
    {
        *at = kept;
    }
    let mut census = slot.census.clone();
    refuse_contradicted_absences(&stamps, std::slice::from_mut(&mut census));
    if matches!(census.state, census::Census::Absent) && !freshness.is_stale() {
        refuse_contradicted_absences(
            &manifest_stamps(&site.store_root),
            std::slice::from_mut(&mut census),
        );
    }
    (
        stamps,
        VendorCensusNow {
            census,
            slot,
            freshness,
        },
    )
}

/// The census of one feed, as a cache [`start`] warms and a background
/// re-read keeps current.
pub(crate) const CENSUS: &str = "census";

/// The `/census/facets.json` index of one feed, every segment (`facets.rs`).
pub(crate) const FACETS: &str = "facets index";

/// The `/indexmap.json` keeper of one feed's held index symbols
/// (`indexmap::HeldIndexKeeper`).
pub(crate) const INDEX_MAP: &str = "index map";

/// The import journal's snapshot and the count `/store.json`'s
/// `x-brutex-census-behind` is built from (`store_route::behind`): the
/// journal read whole once, then by its tail.
pub(crate) const JOURNAL: &str = "import journal";

/// Every cache name [`start`] warms for a feed it is given, in the order
/// `/census/warm.json` lists and the order they are built in: the census
/// first, then what is derived from it, cheapest first ([`warm_derived`]).
pub(crate) const WARMED: [&str; 4] = [CENSUS, JOURNAL, INDEX_MAP, FACETS];

/// What one warmed cache says of itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WarmState {
    /// Not yet read since the process started; a reader that asks now waits
    /// for the warm (single-flight) rather than repeating it.
    Warming,
    /// Read, and current as of its last read.
    Ready,
    /// A background re-read is running; readers are answered from the
    /// previous read, with its age ([`Freshness`]).
    Refreshing,
    /// The warm or the re-read could not be done; `reason` says why, and
    /// readers read it themselves.
    Failed,
}

impl WarmState {
    /// The word the wire carries.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Warming => "warming",
            Self::Ready => "ready",
            Self::Refreshing => "refreshing",
            Self::Failed => "failed",
        }
    }
}

/// One warmed cache's row: what `/census/warm.json` lists, one per (name,
/// feed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Warmth {
    /// One of [`WARMED`].
    pub(crate) name: &'static str,
    /// The feed it is kept for.
    pub(crate) vendor: Vendor,
    /// What it says of itself.
    pub(crate) state: WarmState,
    /// When it entered that state.
    pub(crate) since: std::time::SystemTime,
    /// How long its last build took, once it has finished one.
    pub(crate) took_ms: Option<u64>,
    /// The census generation its last build stood on.
    pub(crate) generation: Option<u64>,
    /// The census entries its last build stood on.
    pub(crate) entries: Option<u64>,
    /// Why, when it failed.
    pub(crate) reason: Option<String>,
}

/// Sets `name` of `vendor` to `state` (and `reason`), adding its row the
/// first time. One lock and a scan of at most three rows per feed.
fn note_warmth(
    vendors: &Vendors,
    name: &'static str,
    vendor: Vendor,
    state: WarmState,
    reason: Option<String>,
) {
    let mut rows = vendors
        .warmth
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let now = std::time::SystemTime::now();
    if let Some(row) = rows
        .iter_mut()
        .find(|row| row.name == name && row.vendor == vendor)
    {
        if row.state != state {
            row.since = now;
        }
        row.state = state;
        row.reason = reason;
        return;
    }
    rows.push(Warmth {
        name,
        vendor,
        state,
        since: now,
        took_ms: None,
        generation: None,
        entries: None,
        reason,
    });
}

/// `name` of `vendor` is ready, built in the time since `began`, over the
/// census in `slot`.
fn note_ready(vendors: &Vendors, name: &'static str, vendor: Vendor, took_ms: u64, slot: &Slot) {
    // A CENSUS THAT WAS READ AND REFUSED IS NOT "READY": the read finished,
    // and what it holds is the refusal, named, which every reader is served
    // until the file reads again. An absent census is ready, and empty.
    let refused = matches!(slot.census.state, census::Census::Unreadable { .. });
    let (state, reason) = if refused {
        (WarmState::Failed, Some(slot.census.note()))
    } else {
        (WarmState::Ready, None)
    };
    note_warmth(vendors, name, vendor, state, reason);
    let generation = slot.census.generation();
    let entries = slot.census.counters().map(|(_, _, entries)| entries);
    {
        let mut rows = vendors
            .warmth
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(row) = rows
            .iter_mut()
            .find(|row| row.name == name && row.vendor == vendor)
        {
            row.took_ms = Some(took_ms);
            row.generation = generation;
            row.entries = entries;
        }
    }
    // EVERY BUILD IS A LINE IN THE LOG (the operator's rule 3: every state
    // change saved and searchable): which cache, which feed, how long, and
    // the census it stood on.
    if telemetry::admits(telemetry::Level::Info, "api.census") {
        let mut event = telemetry::Event::new(telemetry::Level::Info, "api.census", "cache built")
            .with("cache", telemetry::Value::Str(name))
            .with("feed", telemetry::Value::Str(vendor.as_str()))
            .with("state", telemetry::Value::Str(state.word()))
            .with("ms", telemetry::Value::Uint(took_ms));
        // A census with no generation (absent, refused) says nothing here
        // rather than a zero it never published.
        if let Some(generation) = generation {
            event = event.with("generation", telemetry::Value::Uint(generation));
        }
        if let Some(entries) = entries {
            event = event.with("entries", telemetry::Value::Uint(entries));
        }
        let _written_or_not = telemetry::emit(&event);
    }
}

/// Every warmed cache's row, as `/census/warm.json` lists them: in [`WARMED`]
/// order per feed, feeds in the order [`start`] was given them.
pub(crate) fn warmth(site: &Site) -> Vec<Warmth> {
    vendors_of(site)
        .warmth
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Whether [`start`] has put this site's census in background mode.
pub(crate) fn in_background(site: &Site) -> bool {
    vendors_of(site).background.get().is_some()
}

/// Warms every vendor's census and, for each of `feeds`, every cache in
/// [`WARMED`], off the request path, and puts the census in background mode:
/// from here on a reader whose vendor's manifest moved is answered from the
/// kept read while it is read again on a thread of its own (R3-09; RA-032,
/// RA-046, RA-093, RA-094).
///
/// Every row is `warming` before this returns, so `/census/warm.json` never
/// lists a cache as missing while the thread has not reached it. Every
/// vendor's census is warmed, not only `feeds`', because the all-vendor read
/// ([`census_now`]: `/feeds.json`, `/calendar.json`, `/audit.json`) reads
/// every vendor's manifest, and a vendor left cold would be read whole by the
/// first of those requests.
///
/// # Errors
///
/// The thread could not be started. Every cache then stays `warming` until a
/// reader reads it, which it does itself: slow, never wrong. The caller
/// prints why.
pub(crate) fn start(site: &super::Loaded, feeds: &[Vendor]) -> std::io::Result<()> {
    let vendors = vendors_of(site);
    let _first = vendors.background.set(Arc::downgrade(site));
    let _derived = vendors.derived.set(feeds.to_vec());
    // LISTED IN THE ORDER THEY ARE BUILT: each offered feed's census and what
    // is derived from it, then every other vendor's census.
    for &feed in feeds {
        for name in WARMED {
            note_warmth(&vendors, name, feed, WarmState::Warming, None);
        }
    }
    for vendor in Vendor::ALL
        .into_iter()
        .filter(|vendor| !feeds.contains(vendor))
    {
        note_warmth(&vendors, CENSUS, vendor, WarmState::Warming, None);
    }
    let site = super::Loaded::clone(site);
    vendors
        .warm_running
        .store(true, std::sync::atomic::Ordering::Release);
    let theirs = Arc::clone(&vendors);
    let spawned = std::thread::Builder::new()
        .name("brutex-warm-census".to_owned())
        .spawn(move || {
            let _ended = WarmRunning(&theirs.warm_running);
            warm(&site, &theirs);
        });
    if spawned.is_err() {
        vendors
            .warm_running
            .store(false, std::sync::atomic::Ordering::Release);
    }
    spawned.map(drop)
}

/// Clears [`Vendors::warm_running`] when the warm thread ends, by return or
/// by panic.
struct WarmRunning<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for WarmRunning<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// When cache `name` of `vendor` began warming, while [`start`]'s warm
/// thread is running and has not built it yet; `None` once it is built, or
/// when no warm thread runs (never started, or it ended or died), so a
/// reader then builds it itself rather than being told "warming" for ever.
pub(crate) fn warming_since(
    site: &Site,
    name: &'static str,
    vendor: Vendor,
) -> Option<std::time::SystemTime> {
    let vendors = vendors_of(site);
    if !vendors
        .warm_running
        .load(std::sync::atomic::Ordering::Acquire)
    {
        return None;
    }
    let rows = vendors
        .warmth
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    rows.iter()
        .find(|row| row.name == name && row.vendor == vendor && row.state == WarmState::Warming)
        .map(|row| row.since)
}

/// The feeds [`start`] was given, whose derived answers are warmed.
fn derived_feeds(vendors: &Vendors) -> &[Vendor] {
    vendors.derived.get().map_or(&[], Vec::as_slice)
}

/// One read of `vendor`'s census under the stamp taken now, through the
/// single-flight lock a cold reader would wait on, noted ready with the
/// read's own time, then the answers derived from it. What the warm and
/// every background re-read run; hands back the stamps it read under.
fn read_census(
    site: &Site,
    vendors: &Vendors,
    cell: &Cell,
    vendor: Vendor,
) -> (ManifestStamp, bool) {
    let stamps = manifest_stamps(&site.store_root);
    let stamp = stamps
        .manifests
        .get(vendor as usize)
        .copied()
        .unwrap_or(ManifestStamp::Missing);
    let slot = rebuild(&site.store_root, cell, vendor, stamp, stamps.root_is_dir);
    note_ready(vendors, CENSUS, vendor, kept_read_ms(cell), &slot);
    warm_derived(site, vendors, vendor, &slot);
    (stamp, stamps.root_is_dir)
}

/// Whether `slot` is the read `vendor`'s cell keeps NOW: one lock and a
/// reference count. What a derived index asks before it moves back over an
/// older read ([`super::facets::index`]; R3-09).
pub(crate) fn keeps(site: &Site, vendor: Vendor, slot: &Arc<Slot>) -> bool {
    vendors_of(site)
        .cells
        .get(vendor as usize)
        .and_then(Cell::current)
        .is_some_and(|kept| Arc::ptr_eq(&kept.slot, slot))
}

/// [`start`]'s thread: one thread per offered feed, in parallel, each its
/// census then what is derived from it (a feed never waits behind another
/// feed's build), and every other vendor's census here meanwhile. Ends when
/// all have; logs the whole warm's time.
fn warm(site: &super::Loaded, vendors: &Arc<Vendors>) {
    let began = std::time::Instant::now();
    let feeds = derived_feeds(vendors).to_vec();
    let mut feed_threads = Vec::with_capacity(feeds.len());
    for &feed in &feeds {
        let (theirs, mine) = (super::Loaded::clone(site), Arc::clone(vendors));
        let spawned = std::thread::Builder::new()
            .name(format!("brutex-census-warm-{}", feed.as_str()))
            .spawn(move || warm_one(&theirs, &mine, feed));
        match spawned {
            Ok(handle) => feed_threads.push(handle),
            // NO THREAD, NO SKIP: this thread warms the feed itself.
            Err(_) => warm_one(site, vendors, feed),
        }
    }
    for vendor in Vendor::ALL
        .into_iter()
        .filter(|vendor| !feeds.contains(vendor))
    {
        warm_one(site, vendors, vendor);
    }
    for handle in feed_threads {
        if handle.join().is_err() {
            // A WARM THAT PANICKED leaves its rows as they were (`warming`,
            // or the last state it reached), and readers read for themselves.
            warn_line!("a census warm thread panicked; its caches are read by their first request");
        }
    }
    let _dropped_when_filtered = telemetry::emit_if!(
        telemetry::Level::Info,
        "api.census",
        "caches warmed at start",
        "feeds" => telemetry::Value::count(feeds.len()),
        "ms" => telemetry::Value::Uint(ms_since(began)),
    );
}

/// One vendor's warm: [`read_census`] through its cell.
fn warm_one(site: &Site, vendors: &Vendors, vendor: Vendor) {
    if let Some(cell) = vendors.cells.get(vendor as usize) {
        let _stamps = read_census(site, vendors, cell, vendor);
    }
}

/// The answers derived from one read of `vendor`'s census, built now so no
/// request builds them, for a feed [`start`] was given, cheapest first so a
/// request after a restart pays none of them: the import journal's count
/// (which every census answer's `x-brutex-census-behind` reads: 156 ms for
/// GDFL's on the shadow, paid by the first roll-up when it came last), the
/// index map's held index symbols, then the facet index of every segment,
/// moved forward by the entries appended since it last moved (or built once
/// over a census read whole; seconds at GDFL's size, during which its routes
/// answer "warming", `facets::index_or_previous`).
fn warm_derived(site: &Site, vendors: &Vendors, vendor: Vendor, slot: &Arc<Slot>) {
    if !derived_feeds(vendors).contains(&vendor) {
        return;
    }
    let began = std::time::Instant::now();
    let behind = super::store_route::behind(site, vendor);
    match behind.unread {
        Some(why) => note_warmth(vendors, JOURNAL, vendor, WarmState::Failed, Some(why)),
        None => note_ready(vendors, JOURNAL, vendor, ms_since(began), slot),
    }
    let began = std::time::Instant::now();
    match crate::indexmap::warm(site, vendor) {
        Ok(()) => note_ready(vendors, INDEX_MAP, vendor, ms_since(began), slot),
        Err(why) => note_warmth(vendors, INDEX_MAP, vendor, WarmState::Failed, Some(why)),
    }
    let began = std::time::Instant::now();
    for segment in super::facets::SEGMENTS {
        super::facets::index(site, vendor, segment, slot, |_| ());
    }
    note_ready(vendors, FACETS, vendor, ms_since(began), slot);
}

/// `/census/warm.json`'s body: every warmed cache by name and feed, its
/// state, how long it has been in it, its last build's time, and the census
/// generation and entries that build stood on.
pub(crate) fn warm_json(site: &Site) -> String {
    use std::fmt::Write as _;
    let now = std::time::SystemTime::now();
    let rows = warmth(site);
    let mut out = String::with_capacity(64 + rows.len() * 160);
    let _ = write!(out, "{{\"background\":{},\"caches\":[", in_background(site));
    for (at, row) in rows.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let ms = |value: Option<u64>| value.map_or_else(|| "null".to_owned(), |v| v.to_string());
        let _ = write!(
            out,
            "{{\"name\":\"{}\",\"feed\":\"{}\",\"state\":\"{}\",\"since_ms\":{},\"took_ms\":{},\
             \"generation\":{},\"entries\":{},\"reason\":{}}}",
            row.name,
            row.vendor.as_str(),
            row.state.word(),
            u64::try_from(
                now.duration_since(row.since)
                    .unwrap_or_default()
                    .as_millis()
            )
            .unwrap_or(u64::MAX),
            ms(row.took_ms),
            ms(row.generation),
            ms(row.entries),
            row.reason
                .as_deref()
                .map_or_else(|| "null".to_owned(), crate::render::json_string),
        );
    }
    out.push_str("]}");
    out
}

/// `GET /census/warm.json`: [`warm_json`], from memory, on the async worker
/// (no file is read: one lock and at most three rows per feed).
pub(crate) async fn census_warm_json(
    axum::extract::State(site): axum::extract::State<super::Loaded>,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let mut headers = axum::http::HeaderMap::new();
    let _type = headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    let _cache = headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    (axum::http::StatusCode::OK, headers, warm_json(&site))
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
#[path = "census_warm_tests.rs"]
mod warm_tests;

/// A pause inside every census read under one store root, set by a test
/// that must make a read slow on demand: what decides whether a reader
/// waited for it. Keyed on the root so no other test's reads are slowed.
#[cfg(test)]
pub(crate) mod slow_read {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    /// The roots whose reads pause, and for how many milliseconds.
    static PAUSES: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());

    /// Every census read under `root` from now pauses `ms` milliseconds;
    /// zero stops it.
    pub(crate) fn set(root: &Path, ms: u64) {
        let mut pauses = PAUSES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        pauses.retain(|(at, _)| at != root);
        if ms > 0 {
            pauses.push((root.to_path_buf(), ms));
        }
    }

    /// Pauses, if a test asked for it under `root`.
    pub(crate) fn pause(root: &Path) {
        let ms = PAUSES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find(|(at, _)| at == root)
            .map_or(0, |(_, ms)| *ms);
        if ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }
}

/// What one census probe answers for one instrument-month key.
///
/// [`census::VendorCensus::rows_for`] answers `Option<u64>`, and its `None` is
/// two different facts: the census loaded and does not hold that month, or
/// there is no census to ask at all. `/instruments.json` drew the second as the
/// first -- every count a zero over a counter that would not load -- until
/// D-0124 put the census state on the wire beside it. A caller that has to say
/// which of the two it met asks [`census::VendorCensus::rows_of`] instead, and
/// cannot fold them together without writing the arm that does it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the WP-01 probe the All sources work (WP-37) codes against. Its \
                  production caller, `/instruments.json`'s per-symbol fold, now \
                  reads the facet index (D-3327); the tests below prove the probe"
    )
)]
pub(crate) enum RowsOf {
    /// The census loaded and holds this many rows for the key.
    Held(u64),
    /// The census loaded and holds no entry for the key.
    NotHeld,
    /// There is no census to ask: the manifest is absent or unreadable.
    /// [`census::VendorCensus::state`] says which, and
    /// [`census::VendorCensus::note`] says why in its own words.
    NoCensus,
}

impl census::VendorCensus {
    /// How many rows this vendor holds for one month of one instrument, or why
    /// that cannot be answered. See [`RowsOf`].
    ///
    /// One hash probe, the same one [`Self::rows_for`] makes. A degraded census
    /// answers from the generation it recovered, as `rows_for` does, and
    /// [`Self::degraded`] says so: degradation is a fact about the census, not
    /// about one key.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "see `RowsOf`: the WP-01 probe kept for WP-37, proven by the tests below"
        )
    )]
    pub(crate) fn rows_of(&self, key: &pull::manifest::EntryKey) -> RowsOf {
        match self.state {
            census::Census::Held { ref manifest } => manifest
                .entry(key)
                .map_or(RowsOf::NotHeld, |entry| RowsOf::Held(entry.rows)),
            census::Census::Absent | census::Census::Unreadable { .. } => RowsOf::NoCensus,
        }
    }
}

/// [`census_now_stamped`], with the read a miss rebuilds from passed in.
///
/// Production passes [`census::read_all`] and nothing else. The parameter is
/// here so a test can install a newer manifest AFTER the read and before the
/// cache is written -- the one interleaving that decides whether the stamps
/// must be taken BEFORE the read -- on every run, rather than by racing threads
/// and meeting it only when the scheduler happens to: a review measured the
/// racing test catching stamps-after-the-read in 7 of 20 runs. Taken after the
/// read, the stamps would name the newer image while the census held the older
/// one, and every later request would hit the stale census for as long as the
/// manifest stayed unchanged. D-0695.
#[cfg(test)]
pub(super) fn census_now_reading(
    site: &Site,
    read: impl FnOnce(&std::path::Path) -> Vec<census::VendorCensus>,
) -> (CensusStamps, CensusNow) {
    census_now_stamping(site, manifest_stamps, read)
}

/// [`census_now_reading`], with the stamping passed in as well.
///
/// Production passes [`manifest_stamps`] and nothing else. The parameter is
/// here so a test can hand the request stamps a store root that changes while
/// they are taken makes: five manifests stamped missing while the root was
/// away, under a root seen as a directory because it was back for its own
/// check. No test can make a real root do that on demand; a review met it by
/// renaming a root aside and back in a tight loop. D-0695.
#[cfg(test)]
pub(super) fn census_now_stamping(
    site: &Site,
    stamp: impl FnMut(&std::path::Path) -> CensusStamps,
    read: impl FnOnce(&std::path::Path) -> Vec<census::VendorCensus>,
) -> (CensusStamps, CensusNow) {
    census_now_kept(site, stamp, |root| (read(root), None))
}

/// The stamp each of these vendors' rows was read under, where it is not the
/// one the request took: `(vendor as usize, stamp)` for a row served from its
/// previous read while a background re-read runs (R3-09).
type KeptUnder = Option<Vec<(usize, ManifestStamp)>>;

/// [`census_now_stamping`], with the read saying which rows it served from a
/// previous read ([`KeptUnder`]). Those rows' stamps replace the request's in
/// the stamps the read is checked, kept and handed back under, so a census
/// assembled from a stale row is never kept under a stamp it does not
/// reflect -- kept so, it would be served after the re-read landed, for as
/// long as the manifest then stood still.
fn census_now_kept(
    site: &Site,
    mut stamp: impl FnMut(&std::path::Path) -> CensusStamps,
    read: impl FnOnce(&std::path::Path) -> (Vec<census::VendorCensus>, KeptUnder),
) -> (CensusStamps, CensusNow) {
    // CACHED ON THE MANIFESTS' OWN STAMPS, because this is FIVE per-request
    // paths and it reads the whole store on every one of them.
    //
    // `census::read_all` does `std::fs::read` of every vendor's entire manifest
    // -- capped at 256 MiB each -- and `held_entries` then sorts and dedups
    // every entry it found. `census.rs` says so against itself: "O(entries log
    // entries), ON FOUR PER-REQUEST PATHS", and names them. `/calendar.json`
    // reached the same work by its own `read_all`, which made five.
    //
    // AND THREE MORE GET ROUTES STILL CALLED `read_all` DIRECTLY after this
    // cache existed: both branches of `/calendar.json`, `/gaps.json` through
    // `peer_calendar`, and `/bars` through `locate_series` whenever `?exchange=`
    // or `?segment=` is omitted. Each read every vendor's whole manifest on
    // every request, so their cost grew with the store. They come through here
    // now. D-0686, and `docs/06-limits.md`'s D-0686 section for what a request
    // still costs after it.
    //
    // MEASURED consequence, recorded in `store_json`'s own doc: 377,735 bytes
    // for one feed, on a page the console polls every five seconds.
    //
    // THE INVALIDATION IS THE SAME ONE `calendar_of::cached` ALREADY USES: a
    // manifest is the only thing that can change what a census says, so the
    // modified-time of each vendor's manifest is exactly the key. A pull
    // rewrites them and the next request rebuilds; nothing else can go stale.
    // A permission change could, because it moves no modified time and does
    // decide whether a manifest reads, so since D-0695 the key also carries the
    // status-change time from the same `stat`; see `manifest_stamps`.
    //
    // The lock is released BEFORE `read_all`, so a rebuild never serialises the
    // other requests -- the mistake that would turn a cache into a global
    // bottleneck, and the reason `calendar_of` scopes its guard the same way.
    let stamps = stamp(&site.store_root);
    {
        // READ THROUGH A POISONED LOCK rather than around it: a panic while
        // holding it means another request died, the map is still readable, and
        // refusing to look would make one panicked request cost every later one
        // a full store read for the life of the process.
        let held = site
            .census
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((at, censuses, entries)) =
            held.as_ref().and_then(|cached| cached.every.as_ref())
            && *at == stamps
        {
            // A REFCOUNT BUMP, not a whole-store memcpy under the guard. See
            // `CensusCache` for what this used to cost and why the `Arc` is not
            // decoration.
            return (
                stamps,
                (
                    std::sync::Arc::clone(censuses),
                    std::sync::Arc::clone(entries),
                ),
            );
        }
    }

    // THE STAMPS ABOVE ARE OLDER THAN THIS READ, AND MUST BE. A manifest
    // installed between the two keys a newer census under an older stamp, which
    // the next request's stamp no longer matches, so it reads again: stale for
    // at most that one request. The other order keys an older census under a
    // newer stamp, which is stale until the manifest next changes. See
    // `census_now_reading`'s doc.
    let (mut censuses, kept_under) = read(&site.store_root);
    let stale = kept_under.unwrap_or_default();
    let mut stamps = stamps;
    for &(vendor, kept) in &stale {
        if let Some(at) = stamps.manifests.get_mut(vendor) {
            *at = kept;
        }
    }
    // AND A ROW READ ABSENT THAT ITS STAMP CONTRADICTS IS NOT ANSWERED AS AN
    // EMPTY FEED, to this request either. It is served unreadable, naming why.
    // See `refuse_contradicted_absences`. D-0695.
    refuse_contradicted_absences(&stamps, &mut censuses);
    // AND AN ABSENCE STILL STANDING IS STAMPED AGAIN, AFTER THE READ, because
    // the stamps before it can tear: a root away for the five manifest `stat`
    // calls and back for its own check is an empty store's key. An absence
    // those stamps vouch for is served only if these could have read it too.
    // Five or six more `stat` calls, and only on a miss that read an absence.
    // See `refuse_contradicted_absences`. D-0695.
    if censuses
        .iter()
        .any(|row| matches!(row.state, census::Census::Absent))
    {
        // A row served from its previous read is checked against the stamp
        // it was read under, not a newer one it was never meant to reflect.
        let mut again = stamp(&site.store_root);
        for &(vendor, kept) in &stale {
            if let Some(at) = again.manifests.get_mut(vendor) {
                *at = kept;
            }
        }
        refuse_contradicted_absences(&again, &mut censuses);
    }
    let censuses = std::sync::Arc::new(censuses);
    let entries = std::sync::Arc::new(HeldEntries::of(&censuses));
    // AND ONLY A CENSUS THE STAMPS COULD HAVE READ IS KEPT UNDER THEM. A read
    // that contradicts its own key -- a root gone for this one read, a manifest
    // stamped present and read absent, an I/O error that moved no time -- is
    // this request's answer and nobody else's. Kept, it outlived the fault for
    // as long as the stamps stood still, because the fault had not moved them:
    // a 503, or an empty store, over a store that reads. See `read_as_stamped`.
    if !read_as_stamped(&stamps, &censuses) {
        return (stamps, (censuses, entries));
    }
    {
        let mut held = site
            .census
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cached = held.get_or_insert_with(Cached::default);
        // Another cold reader may have finished the same manifest snapshot
        // while this request was reading. Keep its canonical Arc: replacing
        // it with identical data would invalidate the encoded-response cache
        // and repeat the full body encoding and hash for every cold reader.
        if let Some((at, current_censuses, current_entries)) = cached.every.as_ref()
            && *at == stamps
        {
            return (
                stamps,
                (
                    std::sync::Arc::clone(current_censuses),
                    std::sync::Arc::clone(current_entries),
                ),
            );
        }
        cached.every = Some((
            stamps.clone(),
            std::sync::Arc::clone(&censuses),
            std::sync::Arc::clone(&entries),
        ));
    }
    (stamps, (censuses, entries))
}

/// What every vendor manifest's `stat` said, in a fixed order.
///
/// # Why a vector and not one stamp
///
/// A census is over ALL vendors, so any one of them being rewritten makes it
/// stale. A calendar is derived per vendor, so `calendar_of::cached` is keyed on
/// one vendor's modified time, taken from these stamps by
/// [`CensusStamps::modified`]. It took its own `stat` until D-0695's fifth
/// repair, one taken after the census; see `census_now_stamped`.
///
/// A vendor whose manifest does not exist contributes a stamp (`None`, and
/// since D-0695 [`ManifestStamp::Missing`]) rather than being skipped, so a
/// manifest APPEARING changes the key. Skipping it would make the
/// first pull for a new vendor invisible to every cached page.
///
/// # And whether the store root answered, because `None` alone cannot say
///
/// A missing manifest and an unreachable store root both stat as `None`, while
/// [`census::read_all`] answers the first "absent" and the second "unreadable".
/// Keyed on the manifests alone, a census cached in one state was served in the
/// other -- a detached volume reported as a fresh store, or a reattached empty
/// one as an outage -- until a manifest appeared. So when NO manifest answered,
/// the root is asked once whether it is a directory, which is the question
/// `read_all` asks first. A manifest that answered already proves the root did,
/// so a store that holds anything pays nothing for this: five `stat` calls, and
/// a sixth only when all five found nothing.
///
/// # And WHY a manifest did not answer, because "missing" and "refused" differ
///
/// The key above still folded every `stat` error into the one `None` a missing
/// manifest has, while [`census::read_vendor`] answers `NotFound` "absent" and
/// every other error "unreadable". A manifest directory that is a regular file
/// (`ENOTDIR`) or one this process may not search (`EACCES`) therefore keyed
/// exactly as an empty store: a census cached "absent" was served "absent"
/// over the fault and `/calendar.json` answered 200 with no sessions, and a
/// census cached "unreadable" outlived the repair and answered 503 over an
/// empty store. Each manifest is now keyed on what its one `stat` said -- a
/// time, missing, or the kind of fault -- so the same call per vendor draws the
/// three lines `read_vendor` draws. The root is still asked only when no
/// manifest answered with a time, exactly the condition it was asked on
/// before, so this costs no syscall. D-0695.
///
/// # And when the manifest FILE's own status changed, because a mode is not a write
///
/// A manifest that exists was keyed on its modified time alone, and `chmod` and
/// `chown` do not move that time. A manifest file this process could not read
/// when the cache was cold -- written by a pull run as another user -- was
/// cached "unreadable" under a time the permission repair left where it was, so
/// `/calendar.json` kept answering 503 and `/bars` kept refusing over a store
/// that could be read again, until the next pull or a restart. The other
/// direction served "held" over a file that had stopped being readable. The
/// same `stat` also carries the status-change time, which both calls move, so
/// it is now part of the key: no syscall is added. It narrows D-0686's
/// same-modified-time gap as well, because every write moves it too; what is
/// left is two changes inside one tick of the filesystem's clock. D-0695.
pub(super) fn manifest_stamps(store_root: &std::path::Path) -> CensusStamps {
    let manifests: Vec<ManifestStamp> = Vendor::ALL
        .iter()
        .map(|vendor| vendor_stamp(store_root, *vendor))
        .collect();
    let root_is_dir = manifests.iter().any(ManifestStamp::answered)
        || std::fs::metadata(store_root).is_ok_and(|meta| meta.is_dir());
    CensusStamps {
        manifests,
        root_is_dir,
    }
}

/// What one vendor's census files' `stat` calls said: [`manifest_stamps`]'s
/// entry for one vendor, and what the vendor's own cell is keyed on.
///
/// # A migrated vendor is stamped by its partitions (D-3316 item 7)
///
/// One `lstat` of `manifest/v4/<vendor>/` comes first, because its existence
/// decides which census [`census::read_vendor_after`] reads. A directory there
/// is stamped by itself, each segment directory and each segment's current
/// epoch ([`PartitionStamp`]): an append moves the epoch's times, a compaction
/// installs a new epoch by rename and moves its directory's. Anything else at
/// that name is stamped as the file it is, or as the fault its `lstat` met,
/// which is what the reader refuses it as. No directory there is the single
/// file, stamped exactly as before version 4.
pub(super) fn vendor_stamp(store_root: &std::path::Path, vendor: Vendor) -> ManifestStamp {
    let v4 = pull::manifest::vendor_dir(store_root, vendor);
    match std::fs::symlink_metadata(&v4) {
        Ok(meta) if meta.is_dir() => {
            ManifestStamp::Partitioned(PartitionStamp::of(store_root, vendor, &meta))
        }
        Ok(meta) => stamp_of(Ok(meta)),
        Err(absent)
            if matches!(
                absent.kind(),
                std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::NotADirectory
                    | std::io::ErrorKind::InvalidFilename
            ) =>
        {
            stamp_of(std::fs::metadata(
                store_root
                    .join("manifest")
                    .join(format!("{}.man", vendor.as_str())),
            ))
        }
        Err(why) => ManifestStamp::Faulted(why.kind()),
    }
}

/// One file's `stat`, as a stamp: its times, missing, or the fault's kind.
fn stamp_of(meta: std::io::Result<std::fs::Metadata>) -> ManifestStamp {
    match meta.and_then(|meta| {
        Ok(ManifestStamp::At {
            modified: meta.modified()?,
            changed: status_changed(&meta),
        })
    }) {
        Ok(stamp) => stamp,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => ManifestStamp::Missing,
        Err(why) => ManifestStamp::Faulted(why.kind()),
    }
}

/// A migrated vendor's stamp: its directory, and each segment's directory
/// and current epoch, in [`pull::manifest::SEGMENTS`] order. D-3316, D-3327.
///
/// Costs one `lstat`, then per segment one listing of a directory that holds
/// one file per compaction and one `stat` of the current epoch: seven calls
/// for a migrated vendor, against one for a single file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PartitionStamp {
    /// The vendor directory's own times.
    dir: (std::time::SystemTime, StatusChanged),
    /// Each segment's.
    segments: [SegmentStamp; 3],
}

/// One segment's part of a [`PartitionStamp`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SegmentStamp {
    /// No such directory: an empty partition.
    Missing,
    /// Its directory's times, and its current epoch's number and times, or
    /// `None` when it holds no epoch yet.
    Listed {
        dir: (std::time::SystemTime, StatusChanged),
        epoch: Option<(u32, std::time::SystemTime, StatusChanged)>,
    },
    /// The directory or its epoch could not be listed or measured, by kind.
    Faulted(std::io::ErrorKind),
}

impl PartitionStamp {
    fn of(store_root: &std::path::Path, vendor: Vendor, meta: &std::fs::Metadata) -> Self {
        let times = |meta: &std::fs::Metadata| {
            meta.modified()
                .map(|modified| (modified, status_changed(meta)))
        };
        let segment = |segment| -> SegmentStamp {
            let dir = pull::manifest::segment_dir(store_root, vendor, segment);
            let listed = match std::fs::metadata(&dir) {
                Ok(listed) => listed,
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                    return SegmentStamp::Missing;
                }
                Err(why) => return SegmentStamp::Faulted(why.kind()),
            };
            let measured = times(&listed).and_then(|dir_times| {
                let epoch = match census::current_epoch(&dir)? {
                    None => None,
                    Some(epoch) => {
                        let file = std::fs::metadata(pull::manifest::epoch_path(
                            store_root, vendor, segment, epoch,
                        ))?;
                        let (modified, changed) = times(&file)?;
                        Some((epoch, modified, changed))
                    }
                };
                Ok(SegmentStamp::Listed {
                    dir: dir_times,
                    epoch,
                })
            });
            measured.unwrap_or_else(|why| SegmentStamp::Faulted(why.kind()))
        };
        let [index, cash, fno] = pull::manifest::SEGMENTS;
        Self {
            dir: times(meta).unwrap_or((std::time::SystemTime::UNIX_EPOCH, status_changed(meta))),
            segments: [segment(index), segment(cash), segment(fno)],
        }
    }

    /// The latest modified time among everything stamped: what a calendar
    /// derived from this census is kept under ([`CensusStamps::modified`]).
    /// Every append and every installed epoch sets one of these times to the
    /// moment it was written, so the latest moves whenever the census does.
    fn modified(&self) -> std::time::SystemTime {
        self.segments
            .iter()
            .flat_map(|segment| match *segment {
                SegmentStamp::Listed { dir, epoch } => {
                    [Some(dir.0), epoch.map(|(_, modified, _)| modified)]
                }
                SegmentStamp::Missing | SegmentStamp::Faulted(_) => [None, None],
            })
            .flatten()
            .fold(self.dir.0, std::cmp::max)
    }
}

/// Whether every census in `censuses` is one `stamps` could have read, with
/// nothing changed between the two.
///
/// # Why the cache asks, since D-0695
///
/// The stamps are taken BEFORE the read, and must be (see
/// [`census_now_reading`]). A change between the two is therefore met by the
/// next request, whose stamps differ -- unless the change undid itself first. A
/// store root gone when `read_all` asked for it and back before the next
/// request, a root gone just after `read_all`'s own check (every manifest then
/// reads `NotFound`, which is "absent"), or a manifest read that failed with an
/// error no time records (`EIO`, `EMFILE`) all leave every stamp where it was.
/// Cached, that census was served as the store's for as long as the manifests
/// stood still: all five feeds "unreadable", so `/calendar.json` answered 503
/// and `/bars` refused, or all five "absent", so `/calendar.json` answered 200
/// with no sessions and `/bars` said no feed holds the name. A review drove
/// both through [`census_now_reading`] with the root moved aside inside the
/// read.
///
/// So a read is kept only when each census is what its own stamp says the disk
/// would give:
///
/// * `read_all`'s root refusal, only under a root the stamps did not see as a
///   directory, and any other census only under one they did;
/// * absent, only where the stamp found no manifest;
/// * held, or refused by this reader (past the size bound, or bytes that do not
///   decode), only where the stamp found one -- its bytes are what the
///   modified and status-change times vouch for;
/// * an I/O refusal where the stamp found a manifest, only for the kinds its
///   own status decides: `PermissionDenied`, which a `chmod` or `chown` ends
///   by moving the status-change time, and `IsADirectory`, which only a
///   replacement ends. Any other kind moved no time, so no stamp can say when
///   it ends;
/// * an I/O refusal where the stamp itself failed, only of the same kind: the
///   stamp then records the fault, and its end moves the stamp.
///
/// Anything else is served to the request that read it and not kept, so the
/// next request reads again. A row of it read absent that its own stamp could
/// not have read is served unreadable, not absent: see
/// [`refuse_contradicted_absences`]. A fault that persists is then read, and
/// its `api.census` warning emitted, on every request until it ends -- the cost
/// `docs/06-limits.md`'s D-0695 section states. A census row out of
/// `Vendor::ALL` order, or a count other than one per vendor, is never kept
/// either: `census::read_all` yields neither.
///
/// `PermissionDenied` is kept on the word of the file's own status. The same
/// error from a directory on the way, made unsearchable after the stamp and
/// searchable again before the next request, moves no stamp, and is served
/// until the manifest next changes. `docs/06-limits.md` states that too.
pub(super) fn read_as_stamped(stamps: &CensusStamps, censuses: &[census::VendorCensus]) -> bool {
    censuses.len() == stamps.manifests.len()
        && censuses
            .iter()
            .zip(&stamps.manifests)
            .zip(brutex_core::vendor::Vendor::ALL)
            .all(|((read, stamp), vendor)| {
                read.vendor == vendor && stamp_could_read(*stamp, stamps.root_is_dir, &read.state)
            })
}

/// Whether one vendor's `stamp`, under a root the stamps saw as a directory or
/// not, could have read `state`. See [`read_as_stamped`] for each line.
pub(super) fn stamp_could_read(
    stamp: ManifestStamp,
    root_is_dir: bool,
    state: &census::Census,
) -> bool {
    use census::{Census, Fault};
    use std::io::ErrorKind;
    if let Census::Unreadable {
        fault: Fault::Root, ..
    } = *state
    {
        return !root_is_dir;
    }
    root_is_dir
        && match (state, stamp) {
            (Census::Absent, ManifestStamp::Missing)
            | (
                Census::Held { .. }
                | Census::Unreadable {
                    fault:
                        Fault::Refused
                        | Fault::Io(ErrorKind::PermissionDenied | ErrorKind::IsADirectory),
                    ..
                },
                ManifestStamp::At { .. } | ManifestStamp::Partitioned(_),
            ) => true,
            (
                Census::Unreadable {
                    fault: Fault::Io(read),
                    ..
                },
                ManifestStamp::Faulted(stamped),
            ) => *read == stamped,
            _ => false,
        }
}

/// Why a row [`refuse_contradicted_absences`] changed is unreadable, in the
/// words its note and every refusal that quotes the note carry.
pub(super) const CONTRADICTED_ABSENCE: &str = "this read found no manifest here, and the stamps \
     taken of the store just before or just after it contradict that, so it cannot \
     say what this feed holds and is not answered as a feed that holds nothing. The \
     next request reads again";

/// Each row of `censuses` read absent that its own stamp could not have read,
/// served unreadable instead, as [`CONTRADICTED_ABSENCE`] says.
///
/// # Why, since D-0695's eighth repair
///
/// [`read_as_stamped`] keeps such a read out of the cache, and the request that
/// made it was still handed it as read. A store root gone just after
/// `read_all`'s own check reads every feed `NotFound`, which is absent, while
/// Dhan's stamp still names its manifest. So that request's `/calendar.json`
/// answered `200` with no session on both branches, and its `/gaps.json` peer
/// vote agreed nothing and named no feed unreadable: an empty store, over a
/// store that holds NIFTY, the fallback `CLAUDE.md` §4 bans. The next request,
/// over the same missing root, answered 503. A review measured both.
///
/// Absent is the one state that says a feed holds nothing, so it is the one a
/// contradicted row may not say. Served unreadable, the row is refused where
/// every unreadable census is: `/calendar.json` answers 503 on both branches,
/// the peer vote names the feed, and `/bars` refuses it when it is the asked
/// feed's. A contradicted row read held, as a first manifest installed inside
/// the read is under a stamp that found none, keeps the bytes it read, and a
/// contradicted row read unreadable keeps its own words. Neither is changed,
/// and neither is kept: the cache keeps no read with a contradicted row.
///
/// A read no stamp contradicts is unchanged, so [`read_as_stamped`] keeps
/// what it kept before, and a changed row is never kept, because
/// [`stamp_could_read`] vouches for no [`census::Fault::Contradicted`].
///
/// # What it costs, since D-0695's tenth repair
///
/// A row it leaves unchanged costs one index into the stamps and one
/// [`stamp_could_read`], and no syscall. A row it changes also costs the line
/// below, through `telemetry::emit_if!`: the level check alone when no sink
/// is installed, or when the floor the sink applies to `api.census` is above
/// `Warn`. Otherwise it costs the event the macro builds, whose `path` field
/// formats the manifest's path into a new `String`, and what
/// `telemetry::Sink::emit` costs, including the sink's lock, a read of the
/// clock inside it, the line formatted into the sink's buffer, and a
/// `write_all` of the line to its log file, after a rotation of its files
/// when the line would carry the current one past its bound. When that write
/// fails, a second write ends the fragment, and a notice goes to stderr once
/// per sink. So a call writes at most one line per feed. Not timed.
///
/// # Each row it changes is logged, since D-0695's ninth repair
///
/// `census::read_vendor` logs every row it reads, a row read absent at
/// `Debug`, which the default floor does not write, and under the state this
/// request is then not served. This changed the row and logged nothing, so a
/// reader of `/logs` after the refusal found no trace of it. Each row changed
/// here writes one `api.census` line at `Warn`, "absence contradicted by its
/// stamp", naming the feed, the manifest's path and [`CONTRADICTED_ABSENCE`],
/// as an unreadable row `read_vendor` reads is written at `Warn`. That is at
/// most one line per feed per call, and only for a row this changes.
///
/// # And again with stamps taken after the read, since D-0695's ninth repair
///
/// The stamps taken before the read can tear. [`manifest_stamps`] asks the
/// root only when no manifest answered with a time, so a root away for the
/// five manifest `stat` calls and back for its own check stamps five missing
/// manifests under a root that is a directory: an empty store's key, over a
/// store that holds a manifest. Away again once `read_all`'s own check has
/// passed, every row reads absent, and those stamps could have read each one,
/// so this changed nothing: the request was answered an empty store, and the
/// read was kept under the torn key. A review met it with a root renamed aside
/// and back in a tight loop.
///
/// So [`census_now_stamping`] stamps again after the read when any row still
/// reads absent, and hands those stamps here too. An absence is served only
/// where both stampings found no manifest, below a root both saw as a
/// directory. With the root back, the stamps taken again find the manifest
/// the read missed, and with it away they see no directory, so the row is
/// served unreadable and nothing is kept. It asks each row its own stamp, so a manifest rewritten
/// for another feed in between changes no absence, while a first manifest
/// installed for this feed in between is served unreadable to that request,
/// which cannot tell it from a root that came back.
///
/// What is left: stamps taken again that tear the same way vouch for the
/// absence too. A root that is away for both stampings' manifest `stat` calls
/// and for the read, and back for both root checks and for `read_all`'s own,
/// is still answered an empty store, and that read is kept under the torn key
/// and served, without reading, to a later request whose stamps tear the same
/// way, until a request whose stamps do not. `docs/06-limits.md` states it.
pub(super) fn refuse_contradicted_absences(
    stamps: &CensusStamps,
    censuses: &mut [census::VendorCensus],
) {
    for row in censuses {
        let vouched = stamps
            .manifests
            .get(row.vendor as usize)
            .is_some_and(|stamp| stamp_could_read(*stamp, stamps.root_is_dir, &row.state));
        if matches!(row.state, census::Census::Absent) && !vouched {
            // LOGGED HERE, because the read's own line for this row said
            // "absent", at `Debug`: below the default floor, and not the state
            // this request is served. See this function's doc. D-0695.
            let _dropped_when_filtered = telemetry::emit_if!(
                telemetry::Level::Warn,
                "api.census",
                "absence contradicted by its stamp",
                "vendor" => telemetry::Value::Str(row.vendor.as_str()),
                "path" => telemetry::Value::Str(&row.path.display().to_string()),
                "why" => telemetry::Value::Str(CONTRADICTED_ABSENCE),
            );
            row.state = census::Census::Unreadable {
                reason: CONTRADICTED_ABSENCE.to_owned(),
                fault: census::Fault::Contradicted,
            };
        }
    }
}

/// What [`census_now`]'s cache is keyed on: what each vendor manifest's `stat`
/// said, and whether the store root was a directory. See [`manifest_stamps`].
///
/// An equality key, not an ordering: a stamp that moves backwards -- an older
/// image restored with its older time -- is a different key and is re-read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CensusStamps {
    /// One per `Vendor::ALL` entry.
    pub(super) manifests: Vec<ManifestStamp>,
    /// Whether the root is a directory; `true` without asking when any
    /// manifest answered with a modified time.
    pub(super) root_is_dir: bool,
}

impl CensusStamps {
    /// The modified time the `stat` of `row`'s own manifest found, when it
    /// found one and could have read `row`: what `calendar_of::cached` keeps a
    /// calendar derived from `row` under. See [`census_now_stamped`].
    ///
    /// One index, not a search: [`manifest_stamps`] builds one stamp per
    /// `Vendor::ALL` entry in that order, and `Vendor::ALL` is in declaration
    /// order, appended and never inserted, which `merge::Entry::ids` indexes by
    /// the same `vendor as usize`. A missing manifest or one whose `stat` failed
    /// has no time, and a calendar derived under it is not kept, as it was not
    /// when `cached` asked the manifest itself.
    ///
    /// # Only for a row the stamp could have read
    ///
    /// [`census_now_stamped`] hands its stamps to every request, including one
    /// whose census [`read_as_stamped`] refused to keep. A calendar derived
    /// from such a read was kept under a time the fault had not moved and
    /// served to every request after it. A review measured it with a store root
    /// gone after `read_all`'s own check: Dhan read "absent" under a stamp of
    /// its manifest, the symbol branch of `/calendar.json` derived NIFTY over no
    /// months and kept that empty calendar, and once the root was back both
    /// branches answered no session, the peer vote lost Dhan and the ingest
    /// path observed nothing, until Dhan's manifest was next written. So a time
    /// is handed only where [`stamp_could_read`] says this row is one the stamp
    /// could have read, the rule the census cache keeps a whole read by. It is
    /// asked per row, so a fault on another feed's manifest does not turn this
    /// feed's calendar cache off, which
    /// `a_fault_on_another_feeds_manifest_leaves_this_feeds_calendar_kept`
    /// pins for every caller. D-0695.
    ///
    /// # No caller today reaches the refusal
    ///
    /// Since D-0695's seventh repair the symbol branch keeps nothing for a name
    /// its census does not hold, which covers the case above as well. Since the
    /// eighth, that case's row is served unreadable, and `/calendar.json`
    /// refuses it before either branch derives. The other callers derive only
    /// for a row's held series, and a held row under a stamp that found a time
    /// always passes. So no calendar any caller keeps
    /// today is decided by this refusal. It stays at the one function every key
    /// comes from, so a caller added later that derives from a row its stamp
    /// contradicts is covered, and its unit test pins it.
    pub(crate) fn modified(&self, row: &census::VendorCensus) -> Option<std::time::SystemTime> {
        match self.manifests.get(row.vendor as usize).copied() {
            Some(stamp @ ManifestStamp::At { modified, .. })
                if stamp_could_read(stamp, self.root_is_dir, &row.state) =>
            {
                Some(modified)
            }
            Some(stamp @ ManifestStamp::Partitioned(partitions))
                if stamp_could_read(stamp, self.root_is_dir, &row.state) =>
            {
                Some(partitions.modified())
            }
            _ => None,
        }
    }
}

/// One vendor manifest's part of [`CensusStamps`]: what its `stat` said.
///
/// Three outcomes because [`census::read_vendor`] draws the same three lines
/// from the same call: a file it goes on to read, `NotFound` it calls absent,
/// and any other error it calls unreadable. Two outcomes sharing one key are two
/// census states the cache cannot tell apart. D-0695.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "a stamp is `Copy` and compared by value on every request, five per request; \
              boxing the partition stamp would allocate per `stat` to save ~150 bytes"
)]
pub(super) enum ManifestStamp {
    /// It exists: last modified then, and its status last changed then.
    At {
        /// The modified time, which a write moves.
        modified: std::time::SystemTime,
        /// The status-change time, which a write, a `chmod` and a `chown` all
        /// move. See [`manifest_stamps`].
        changed: StatusChanged,
    },
    /// `NotFound`: no manifest, or no directory or root on the way to one.
    Missing,
    /// Any other error, by kind: `NotADirectory` where a directory on the way
    /// is a file, `PermissionDenied` where one may not be searched.
    Faulted(std::io::ErrorKind),
    /// A migrated vendor's partitions (D-3316): see [`vendor_stamp`].
    Partitioned(PartitionStamp),
}

impl ManifestStamp {
    /// Whether the `stat` found something there, which proves the store root
    /// answered: a file's times, or a migrated vendor's directory.
    const fn answered(&self) -> bool {
        matches!(self, Self::At { .. } | Self::Partitioned(_))
    }
}

/// A manifest's status-change time, `st_ctime` in seconds and nanoseconds.
#[cfg(unix)]
pub(super) type StatusChanged = (i64, i64);

/// No status-change time: outside Unix, `std` exposes none, and the key is the
/// modified time alone, as it was before D-0695.
#[cfg(not(unix))]
pub(super) type StatusChanged = ();

/// What `meta`'s one `stat` said about when its status last changed.
#[cfg(unix)]
fn status_changed(meta: &std::fs::Metadata) -> StatusChanged {
    use std::os::unix::fs::MetadataExt as _;
    (meta.ctime(), meta.ctime_nsec())
}

/// Nothing: see [`StatusChanged`].
#[cfg(not(unix))]
fn status_changed(_meta: &std::fs::Metadata) -> StatusChanged {}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use crate::server::tests::site;

    /// The census is read once per manifest change, not once per request.
    ///
    /// # What this bounds
    ///
    /// `census_now` is on FIVE per-request paths — `/instruments.json`,
    /// `/verify.json`, `/store.json`, `/audit.json`, and `/calendar.json` by its
    /// own `read_all`. Uncached it does `std::fs::read` of every vendor's whole
    /// manifest and then sorts and dedups every entry, on a page the console
    /// polls every five seconds. `census.rs` names the shape against itself:
    /// "O(entries log entries), ON FOUR PER-REQUEST PATHS".
    ///
    /// # It asserts INVALIDATION as well as caching, and that is the harder half
    ///
    /// A cache that never rebuilds passes a hit test perfectly and serves a
    /// pre-pull answer forever. The stamp is each manifest's modified time, so
    /// rewriting one must be visible on the next call — and that is the property
    /// that makes the cache safe rather than merely fast.
    #[test]
    fn the_census_is_cached_on_the_manifests_and_rebuilt_when_one_changes() {
        let dir = crate::scratch::path("census-cache-assets");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let site = site("census-cache", &dir);

        // Nothing cached yet: the slot starts empty, so a hit here would be a
        // hit on a cache that was never filled.
        assert!(
            site.census
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
                .and_then(Cached::kept_stamps)
                .is_none(),
            "a fresh Site holds no census"
        );

        let (first_censuses, first_entries) = census_now(&site);
        let stamped_after_first = site
            .census
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .and_then(Cached::kept_stamps);
        assert!(
            stamped_after_first.is_some(),
            "the first call fills the cache"
        );

        let (second_censuses, second_entries) = census_now(&site);
        assert_eq!(
            first_censuses.len(),
            second_censuses.len(),
            "a cached read answers the same census"
        );
        assert_eq!(
            first_entries, second_entries,
            "and the same entries, which is what every page keys off"
        );

        // A MANIFEST CHANGES AND THE NEXT CALL REBUILDS. Written with a real
        // vendor name so the stamp vector's own slot moves -- a file the stamp
        // function does not look at would prove nothing.
        let manifest = site.store_root.join("manifest").join("zerodha.man");
        std::fs::write(&manifest, b"not a manifest, but it has an mtime").expect("writable");
        let _ = census_now(&site);
        let stamped_after_write = site
            .census
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .and_then(Cached::kept_stamps);
        assert_ne!(
            stamped_after_first, stamped_after_write,
            "writing a manifest must move the key the cache is held on, or a \
             pull would be invisible to every cached page for the life of the \
             process"
        );
    }

    /// One instrument-month key and a held census that carries it at 375 rows.
    fn held_nifty() -> (pull::manifest::EntryKey, pull::manifest::Manifest) {
        let series = census::Series {
            contract: None,
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Index,
            symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
            timeframe: store::path::Timeframe::MINUTE_1,
        };
        let key = series.at(store::path::YearMonth::new(2026, 9).expect("a month"));
        let mut manifest =
            pull::manifest::Manifest::open(brutex_core::vendor::Vendor::Gdfl, &[], &[])
                .expect("a genesis census");
        manifest
            .record(pull::manifest::Entry {
                key,
                rows: 375,
                first_ts_micros: 1_788_926_700_000_000,
                last_ts_micros: 1_788_949_140_000_000,
            })
            .expect("records");
        (key, manifest)
    }

    /// **`rows_of` KEEPS "NOT HELD" APART FROM "NO CENSUS TO ASK"**, which
    /// `rows_for`'s one `None` cannot. WP-01's second interface.
    ///
    /// Each of the four states is asked the same key, and `rows_for` is asked
    /// beside it, so the test shows both what the new probe says and the
    /// conflation it exists to remove: three of the four answer `None` there.
    #[test]
    fn rows_of_keeps_a_month_not_held_apart_from_no_census_to_ask() {
        let (key, manifest) = held_nifty();
        let mut other = key;
        other.month = store::path::YearMonth::new(2026, 8).expect("a month");
        let row = |state| census::VendorCensus {
            vendor: brutex_core::vendor::Vendor::Gdfl,
            path: std::path::PathBuf::from("/nonexistent/census-cache/gdfl.man"),
            state,
        };
        let held = row(census::Census::Held {
            manifest: census::Manifests::single(manifest),
        });
        assert_eq!(held.rows_of(&key), RowsOf::Held(375));
        assert_eq!(held.rows_for(&key), Some(375), "the probe is the same one");
        assert_eq!(held.rows_of(&other), RowsOf::NotHeld);
        assert_eq!(held.rows_for(&other), None);

        let absent = row(census::Census::Absent);
        let unreadable = row(census::Census::Unreadable {
            reason: "a header that does not decode".to_owned(),
            fault: census::Fault::Refused,
        });
        for nothing in [&absent, &unreadable] {
            assert_eq!(nothing.rows_of(&key), RowsOf::NoCensus, "{nothing:?}");
            assert_eq!(
                nothing.rows_for(&key),
                held.rows_for(&other),
                "rows_for answers no census exactly as it answers a month not held"
            );
        }
    }

    /// **`vendor_census` LENDS THE CACHED ROW OF THE VENDOR ASKED FOR**, with the
    /// stamps it was read under, and shares the cached read rather than copying
    /// it. WP-01's first interface, and since D-3327 a read of that vendor's
    /// own cell only.
    ///
    /// Over a real store root: one vendor's manifest is written to disk, so the
    /// held row, the absent rows and the stamps all come off the read every
    /// route makes.
    #[test]
    fn vendor_census_lends_the_cached_row_of_the_vendor_asked_for() {
        use brutex_core::vendor::Vendor;
        let dir = crate::scratch::path("vendor-census-assets");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let site = site("vendor-census", &dir);
        let (key, manifest) = held_nifty();
        std::fs::write(
            pull::manifest::manifest_path(&site.store_root, Vendor::Gdfl),
            manifest.image(),
        )
        .expect("a manifest on disk");

        let (stamps, _every) = census_now_stamped(&site);
        for vendor in Vendor::ALL {
            let (asked_under, now) = vendor_census(&site, vendor);
            assert_eq!(asked_under, stamps, "the stamps the census was read under");
            let (_, again) = vendor_census(&site, vendor);
            assert!(
                std::sync::Arc::ptr_eq(now.slot(), again.slot()),
                "a hit lends the kept slot; it never reads again or copies a census"
            );
            let census = now.census().expect("one row per vendor");
            assert_eq!(census.vendor, vendor);
            let want = if vendor == Vendor::Gdfl {
                RowsOf::Held(375)
            } else {
                RowsOf::NoCensus
            };
            assert_eq!(census.rows_of(&key), want, "{vendor:?}");
        }
    }

    /// **ANOTHER VENDOR'S COMMIT LEAVES THIS VENDOR'S CELL A HIT** (DB-G79):
    /// the cell is keyed on the vendor's own stamp, so rewriting Zerodha's
    /// manifest neither re-reads GDFL's nor replaces its slot.
    #[test]
    fn another_vendors_commit_leaves_this_vendors_cell_a_hit() {
        use brutex_core::vendor::Vendor;
        let dir = crate::scratch::path("vendor-cell-assets");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let site = site("vendor-cell", &dir);
        let (_key, manifest) = held_nifty();
        let gdfl = pull::manifest::manifest_path(&site.store_root, Vendor::Gdfl);
        std::fs::write(&gdfl, manifest.image()).expect("a manifest on disk");
        let (_, first) = vendor_census(&site, Vendor::Gdfl);
        let reads = census::manifest_reads::of(&gdfl);
        std::fs::write(
            pull::manifest::manifest_path(&site.store_root, Vendor::Zerodha),
            b"another vendor's commit",
        )
        .expect("a second manifest");
        let (every, _) = census_now(&site);
        assert_eq!(
            every.len(),
            Vendor::ALL.len(),
            "the all-vendor read still answers"
        );
        let (_, after) = vendor_census(&site, Vendor::Gdfl);
        assert!(
            std::sync::Arc::ptr_eq(first.slot(), after.slot()),
            "GDFL's slot survives Zerodha's commit"
        );
        assert_eq!(
            census::manifest_reads::of(&gdfl),
            reads,
            "and GDFL's manifest is not read again"
        );
    }

    /// **A STAMP THAT MOVES WITH NOTHING APPENDED KEEPS THE SAME SLOT; AN
    /// APPEND MAKES A NEW ONE, READ BY ITS TAIL** (D-3327 item 1).
    ///
    /// Every answer derived from a census (the 21 MB tuples body, the facet
    /// index, the audit rollup) is kept against its slot. A re-read that found
    /// the census unchanged made a new slot, so a `touch` of `gdfl.man` made
    /// the next `/audit.json?feed=gdfl` recount every key and the next tuples
    /// request encode the body again (measured on a shadow store: a 1.2 s
    /// `/audit.json` while the stamp moved every 2 s). Now the unchanged read
    /// is the same slot under the new stamp, nothing is read whole, and only a
    /// real append makes a new slot, read by its tail alone.
    #[test]
    fn a_moved_stamp_with_nothing_appended_keeps_the_same_slot() {
        use brutex_core::vendor::Vendor;
        use std::os::unix::fs::FileExt as _;
        let dir = crate::scratch::path("vendor-cell-same-read-assets");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let site = site("vendor-cell-same-read", &dir);
        let (key, mut manifest) = held_nifty();
        let gdfl = pull::manifest::manifest_path(&site.store_root, Vendor::Gdfl);
        std::fs::write(&gdfl, manifest.image()).expect("a manifest on disk");
        let (stamped, first) = vendor_census(&site, Vendor::Gdfl);
        let whole = census::manifest_reads::of(&gdfl);
        let tails = census::manifest_reads::tails_of(&gdfl).len();

        // THE STAMP MOVES, NOTHING IS APPENDED.
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&gdfl)
            .expect("open");
        file.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_hours(24))
            .expect("a new modified time");
        let (moved, _) = vendor_census(&site, Vendor::Gdfl);
        let (_, again) = vendor_census(&site, Vendor::Gdfl);
        assert_ne!(moved, stamped, "the stamp moved");
        assert!(
            Arc::ptr_eq(first.slot(), again.slot()),
            "the same read keeps the same slot under the new stamp"
        );
        assert_eq!(
            census::manifest_reads::of(&gdfl),
            whole,
            "nothing read whole"
        );

        // AN APPEND: a new slot, read by its tail of one entry.
        let append = manifest
            .record(pull::manifest::Entry {
                key,
                rows: 750,
                first_ts_micros: 1_788_926_700_000_000,
                last_ts_micros: 1_789_949_140_000_000,
            })
            .expect("records");
        file.write_all_at(&append.bytes, append.offset)
            .expect("entry");
        file.write_all_at(&append.commit.bytes, append.commit.offset)
            .expect("slot");
        let (_, grown) = vendor_census(&site, Vendor::Gdfl);
        assert!(
            !Arc::ptr_eq(first.slot(), grown.slot()),
            "a new census, a new slot"
        );
        assert_eq!(
            census::manifest_reads::of(&gdfl),
            whole,
            "still nothing read whole"
        );
        assert_eq!(
            census::manifest_reads::tails_of(&gdfl).len(),
            tails + 1,
            "read by its tail"
        );
        assert_eq!(
            grown.census().and_then(|row| row.rows_for(&key)),
            Some(750),
            "the appended row is the newest"
        );
    }
}
