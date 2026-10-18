//! The importer's census (R3-35, a move-only split of `gdfl_import.rs`):
//! the census a run publishes each day's batch through, its room, and the
//! run stopped at a census refusal.

use std::path::Path;

use brutex_core::vendor::Vendor;

use crate::ingest::record_held;
use crate::manifest::{ENTRY_STRIDE, HEADER_LEN, Held, MAX_ENTRIES, Manifest, manifest_path};
use crate::session::Day;

use super::causes::{Cause, ImportRefusal, Stage};
use super::day::{Ctx, Mark, Staged, close_day};
use super::{ImportKind, Report, TARGET, failure_on};

/// A census's room for one day's batch, measured from the census FILE after it
/// refused the batch (D-3317; DB-G10). The census's own refusal quotes its
/// counter in memory part way through the batch; these are the file's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CensusRoom {
    /// Entries the census file has committed.
    pub n_valid: u64,
    /// The most entries this build lets one census hold
    /// ([`crate::manifest::MAX_ENTRIES`]).
    pub limit: u64,
    /// Entries the day offered.
    pub batch: u64,
    /// Entries of the batch the census does not already hold exactly as
    /// offered: at most the appends the batch needs.
    pub new: u64,
}

impl CensusRoom {
    /// Entries the census can still take.
    #[must_use]
    pub const fn free(&self) -> u64 {
        self.limit.saturating_sub(self.n_valid)
    }

    /// Whether the batch's new entries fit the free ones.
    #[must_use]
    pub const fn fits(&self) -> bool {
        self.new <= self.free()
    }
}

/// The census a run publishes each day's batch through (D-3317). The store's
/// own is [`StoreCensus`]; a test hands a double, because a census at the
/// entry ceiling is a 268 MB file and not a thing a unit test builds (the
/// reason `pull::unit`'s ceiling checks give for their own arithmetic).
pub(super) trait Census {
    /// Publishes `held`, or says why not, as [`record_held`] does: all or
    /// nothing.
    fn publish(&mut self, store_root: &Path, held: &[Held]) -> Option<String>;

    /// The census's room for `held`, measured from its file.
    fn room(&self, store_root: &Path, held: &[Held]) -> Result<CensusRoom, String>;
}

/// The store's census of the `gdfl` feed.
pub(super) struct StoreCensus;

impl Census for StoreCensus {
    fn publish(&mut self, store_root: &Path, held: &[Held]) -> Option<String> {
        record_held(store_root, Vendor::Gdfl, held)
    }

    fn room(&self, store_root: &Path, held: &[Held]) -> Result<CensusRoom, String> {
        census_room(store_root, held)
    }
}

/// The largest census file this build can have written: the entry ceiling at
/// the widest stride it writes, after the header region. Derived, not chosen.
pub(super) const CENSUS_CAP: u64 = HEADER_LEN + MAX_ENTRIES * ENTRY_STRIDE;

/// The `gdfl` census's room for `held`, from its version-3 file (D-3317;
/// DB-G10).
///
/// One capped read and one [`Manifest::open_image`] of the census, Θ(entries)
/// and on the refusal path only: the run stops after it. `new` counts the
/// batch's entries the census does not hold exactly as offered, by the same
/// comparison the census's own count makes, so it is at most the appends the
/// batch needs. An absent census is an empty one, as the census reads it.
///
/// A store that holds D-3316's version-4 census (`manifest/v4/<vendor>/`,
/// WP-10) is not measured here: its writer refuses up front in its own words,
/// naming the file's `n_valid`, the batch and the free slots, and the frozen
/// version-3 file beside it would give numbers that are no longer the
/// census's. Its room is then `Err`, naming why, and the refusal is
/// [`ImportRefusal::CensusRefused`] carrying those words.
///
/// # Errors
///
/// The path and why, for a census that cannot be read or does not load, or a
/// version-4 census.
pub(super) fn census_room(store_root: &Path, held: &[Held]) -> Result<CensusRoom, String> {
    let path = manifest_path(store_root, Vendor::Gdfl);
    let v4 = path
        .parent()
        .map(|dir| dir.join("v4").join(Vendor::Gdfl.as_str()));
    if let Some(v4) = v4.filter(|dir| dir.exists()) {
        return Err(format!(
            "{} holds the version-4 census (D-3316); its own refusal names its room",
            v4.display()
        ));
    }
    let bytes = match crate::ingest::read_regular_capped(&path, CENSUS_CAP) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(why)) => return Err(format!("{} {why}", path.display())),
        Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    let census = Manifest::open_image(Vendor::Gdfl, &bytes)
        .map_err(|why| format!("{}: {why}", path.display()))?;
    let new = held
        .iter()
        .filter(|one| census.held(&one.entry.key) != Some(**one))
        .count();
    Ok(CensusRoom {
        n_valid: census.entries(),
        limit: MAX_ENTRIES,
        batch: u64::try_from(held.len()).unwrap_or(u64::MAX),
        new: u64::try_from(new).unwrap_or(u64::MAX),
    })
}

/// The code of the census refusal a `room` makes (D-3317 items 6 and 7): the
/// census's own `too_many_entries` when the room is measured and the batch
/// does not fit it, `census_refused` otherwise. [`census_refusal`] chooses its
/// variant by the same test, so the journal's `reason=` and the refusal's
/// [`ImportRefusal::code`] agree.
pub(super) const fn census_code(room: &Result<CensusRoom, String>) -> &'static str {
    match room {
        Ok(room) if !room.fits() => "too_many_entries",
        _ => "census_refused",
    }
}

/// The census refusal of the day, from the census's words and its measured
/// room: full when the room is measured and the batch does not fit, refused
/// otherwise (D-3317 item 7).
pub(super) fn census_refusal(
    (kind, day): (ImportKind, Day),
    why: String,
    room: Result<CensusRoom, String>,
    done: Report,
) -> ImportRefusal {
    match room {
        Ok(room) if !room.fits() => ImportRefusal::CensusFull {
            kind,
            day,
            room,
            why,
            done: Box::new(done),
        },
        room => ImportRefusal::CensusRefused {
            kind,
            day,
            room,
            why,
            done: Box::new(done),
        },
    }
}

/// Stops the run at a census refusal (D-3317 item 7; DB-G01, DB-G10). The
/// day's bars are on disk, so it is journalled `incomplete` with
/// `stage=census` and its files and seconds stay in the report; it is not
/// counted in `days_imported`; ONE `error` event names the census file's
/// room, the batch and that nothing was published; and the refusal carries
/// the report of what the run did before it stopped.
pub(super) fn stop_at_census(
    ctx: &Ctx<'_>,
    (key, day): (&str, Day),
    (before, mut report, max_back_s): (Mark, Report, u32),
    mut failures: Vec<Staged>,
    (why, room): (String, Result<CensusRoom, String>),
) -> ImportRefusal {
    let kind = ctx.run.kind;
    let cause = Cause::open(Stage::Census, census_code(&room));
    let words = census_words(&why, &room);
    // The one stop event is emitted before the failure is filed, so the
    // failure has its event beside it (gate 19). `close_day` only reads the
    // report, so the event's fields are what they were; its one event of its
    // own, a journal line it could not append, now follows this one.
    let day_text = day.to_string();
    let event = stopped_event(
        (kind, day_text.as_str(), cause),
        (words.as_str(), report.days_imported),
        room.as_ref().ok(),
    );
    let _dropped_when_filtered = telemetry::emit(&event);
    failures.push(Staged {
        cause,
        failure: failure_on(day, &format!("{day} census"), words),
    });
    let _incomplete = close_day(
        ctx,
        (key, day),
        (before, &report, max_back_s),
        &mut failures,
    );
    report
        .failures
        .extend(failures.into_iter().map(|one| one.failure));
    census_refusal((kind, day), why, room, report)
}

/// "census refused; run stopped" (D-3317 item 7; DB-G10), at `error`: the
/// day, the cause, the census's room when it was measured, that nothing was
/// published and how many days the run imported first, in at most
/// [`telemetry::MAX_FIELDS`] fields, so none of the room's numbers is the one
/// the sink counts instead of keeping.
pub(super) fn stopped_event<'a>(
    (kind, day, cause): (ImportKind, &'a str, Cause),
    (why, days_imported): (&'a str, usize),
    room: Option<&CensusRoom>,
) -> telemetry::Event<'a> {
    let event = telemetry::Event::error(TARGET, "census refused; run stopped")
        .with("kind", kind.as_str())
        .with("day", day)
        .with("stage", cause.stage.as_str())
        .with("code", cause.code)
        .with("why", why)
        .with("days_imported", days_imported)
        .with("published", 0_u64);
    match room {
        Some(room) => event
            .with("n_valid", room.n_valid)
            .with("limit", room.limit)
            .with("free", room.free())
            .with("batch", room.batch)
            .with("new", room.new),
        None => event,
    }
}

/// The day's census failure in words that name the file's numbers (D-3317;
/// DB-G10): that nothing of the batch was published, the room and the batch
/// FIRST, the census's own words last, so a log value cut at its bound keeps
/// what matters.
fn census_words(why: &str, room: &Result<CensusRoom, String>) -> String {
    match room {
        Ok(room) if !room.fits() => format!(
            "the census is full, so nothing of the batch was published: n_valid={} limit={} \
             free={} batch={} new={} (the census's own words: {why})",
            room.n_valid,
            room.limit,
            room.free(),
            room.batch,
            room.new,
        ),
        Ok(room) => format!(
            "the census refused a batch that fits, and nothing of the batch was published \
             (n_valid={} limit={} free={} batch={} new={}): {why}",
            room.n_valid,
            room.limit,
            room.free(),
            room.batch,
            room.new,
        ),
        Err(unread) => format!(
            "the census refused, and nothing of the batch was published: {why}; its room could \
             not be measured: {unread}"
        ),
    }
}
