//! Exact, reference-only India VIX bars for observable trade stamps.
//!
//! `NSE-INDIAVIX` is not a third swept instrument. It is opened here through a
//! path that hard-codes the reference key and the one-minute rung; it is never
//! offered to `InstrumentKey::require_sweepable`, the condition vocabulary, a
//! ranking function, or a run-identity constructor.
//!
//! # No scalar and no interpolation
//!
//! A lookup returns [`VixStamp::Absent`] or [`VixStamp::Exact`]. `Exact`
//! carries the complete seven-field [`Candle`] copied from the stored bar.
//! There is deliberately no close-only accessor, zero fallback, previous-value
//! fill, or nearest-minute search. A caller that needs a number must first
//! handle absence and then name which field of the exact bar it means.
//!
//! # Fixed civil-month index
//!
//! One opened month owns exactly [`CIVIL_MONTH_MINUTE_SLOTS`] slots. A timestamp
//! maps to `(IST day - 1) * 1_440 + IST minute-of-day`, so lookup is fixed
//! arithmetic plus one indexed read. Loading is linear in the records in that
//! one file; lookup is O(1), and the allocation is a fixed maximum per opened
//! month rather than a map whose size or probe history depends on the data.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::path::Path;

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use indicators::Candle;
use pull::session::IstMoment;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

/// Canonical store symbol of the reference-only India VIX series.
pub const VIX_REFERENCE_SYMBOL: &str = "INDIAVIX";

/// Minute slots in the widest civil month: 31 complete IST days.
pub const CIVIL_MONTH_MINUTE_SLOTS: usize = 31 * 24 * 60;

const MICROS_PER_MINUTE: i64 = 60_000_000;
const MINUTES_PER_DAY: usize = 24 * 60;

/// One exact India VIX observation at a requested trade timestamp.
///
/// Absence is a variant rather than a sentinel. In particular, an absent VIX
/// bar can never become an all-zero bar or inherit a neighbouring minute by
/// conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VixStamp {
    /// The opened month contains no India VIX bar at that exact timestamp.
    Absent,
    /// The complete stored India VIX OHLCV/OI bar at that exact timestamp.
    Exact(Candle),
}

/// A validated one-minute `NSE-INDIAVIX` civil month.
///
/// Construction reads and validates every committed row before publishing the
/// index. A partially indexed month is never returned.
#[derive(Debug)]
pub struct VixReferenceMonth {
    vendor: Vendor,
    month: YearMonth,
    records: u64,
    slots: Box<[Option<Candle>]>,
}

impl VixReferenceMonth {
    /// Open and completely index one stored India VIX one-minute month.
    ///
    /// The instrument and rung are not caller arguments: this door can open
    /// only `NSE-INDIAVIX/1min`. The ordinary stored-sweep door remains guarded
    /// by `InstrumentKey::SWEPT` and is not shared with this reference door.
    ///
    /// # Errors
    ///
    /// Refuses a missing, locked, torn, malformed, or otherwise unreadable
    /// store file. It also refuses any committed row with an impossible OHLC or
    /// count, an off-minute timestamp, a timestamp outside `month`, a duplicate
    /// exact timestamp, or timestamps that do not remain strictly increasing.
    pub fn open(root: &Path, vendor: Vendor, month: YearMonth) -> Result<Self, String> {
        let key = InstrumentKey::index(Exchange::Nse, VIX_REFERENCE_SYMBOL).map_err(|why| {
            format!(
                "the fixed reference key NSE-{VIX_REFERENCE_SYMBOL} is invalid: {why}. Nothing was read"
            )
        })?;
        if key.is_sweepable() {
            return Err(format!(
                "the fixed reference key NSE-{VIX_REFERENCE_SYMBOL} entered the swept instrument set; reference loading refused"
            ));
        }

        let path = StorePath::for_key(vendor, &key, Timeframe::MINUTE_1, month, FileKind::Bars)
            .map_err(|why| {
                format!(
                    "the {vendor} NSE-{VIX_REFERENCE_SYMBOL} 1min path for {month} is invalid: {why}. Nothing was read",
                    vendor = vendor.as_str()
                )
            })?;

        // The store header writes the low 32 bits of this exact FNV-1a value.
        // Asking with any other derivation would reject the correct file as a
        // symbol mismatch.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the store's symbol id is defined as the low 32 bits of FNV-1a; pull::ingest writes the same cast and BarFile compares that exact value"
        )]
        let symbol_id = brutex_core::universe::fnv1a(VIX_REFERENCE_SYMBOL) as u32;
        let file = BarFile::open_existing(root, path, symbol_id).map_err(|why| {
            format!(
                "{vendor} NSE-{VIX_REFERENCE_SYMBOL} 1min {month} could not be opened as reference evidence: {why}. Nothing was stamped",
                vendor = vendor.as_str()
            )
        })?;

        Self::from_file(vendor, month, &file)
    }

    fn from_file(vendor: Vendor, month: YearMonth, file: &BarFile) -> Result<Self, String> {
        let records = file.records();
        let mut slots = vec![None; CIVIL_MONTH_MINUTE_SLOTS].into_boxed_slice();
        let mut previous = None;

        for record in 0..records {
            let bar = file.read_record(record).map_err(|why| {
                format!(
                    "{vendor} NSE-{VIX_REFERENCE_SYMBOL} 1min {month} record {record} of {records} could not be read: {why}. Nothing was stamped",
                    vendor = vendor.as_str()
                )
            })?;
            validate_bar(record, &bar)?;
            let slot = slot_of(month, bar.ts_micros, &format!("stored record {record}"))?;
            let cell = slots.get_mut(slot).ok_or_else(|| {
                format!(
                    "stored record {record} timestamp {} resolved to civil-month slot {slot}, outside the fixed {CIVIL_MONTH_MINUTE_SLOTS}-slot index",
                    bar.ts_micros
                )
            })?;
            if cell.is_some() {
                return Err(format!(
                    "stored record {record} duplicates exact India VIX timestamp {}; duplicate reference evidence was refused",
                    bar.ts_micros
                ));
            }
            if let Some(before) = previous
                && bar.ts_micros <= before
            {
                return Err(format!(
                    "stored record {record} India VIX timestamp {} is not strictly after {before}; reordered reference evidence was refused",
                    bar.ts_micros
                ));
            }
            *cell = Some(candle_of(bar));
            previous = Some(bar.ts_micros);
        }

        Ok(Self {
            vendor,
            month,
            records,
            slots,
        })
    }

    /// Feed whose exact stored month this index represents.
    #[must_use]
    pub const fn vendor(&self) -> Vendor {
        self.vendor
    }

    /// Civil month whose IST minute slots this index represents.
    #[must_use]
    pub const fn month(&self) -> YearMonth {
        self.month
    }

    /// Number of committed records admitted into the fixed index.
    #[must_use]
    pub const fn records(&self) -> u64 {
        self.records
    }

    /// Pin the complete original validated reference snapshot, including holes.
    /// This presentation digest is never a strategy or execution identity.
    pub(crate) fn snapshot_digest(&self) -> [u8; 32] {
        let mut digest = brutex_core::blake3::Hasher::new();
        digest.update(b"brutex-india-vix-validated-month-snapshot-v1\0");
        digest.update(self.vendor.as_str().as_bytes());
        digest.update(&self.month.year().to_le_bytes());
        digest.update(&[self.month.month()]);
        digest.update(&self.records.to_le_bytes());
        for slot in &self.slots {
            let Some(bar) = slot else {
                digest.update(&[0]);
                continue;
            };
            digest.update(&[1]);
            for value in [
                bar.ts_micros,
                bar.open,
                bar.high,
                bar.low,
                bar.close,
                bar.volume,
                bar.open_interest,
            ] {
                digest.update(&value.to_le_bytes());
            }
        }
        digest.finalize()
    }

    /// Stamp an exact timestamp from this month.
    ///
    /// # Cost
    ///
    /// Constant arithmetic and one fixed-slot read, independent of how many
    /// India VIX records the month contains.
    ///
    /// # Errors
    ///
    /// Refuses an off-minute or wrong-month timestamp. These are malformed
    /// questions, not absent observations. An occupied slot whose stored
    /// timestamp differs is also refused instead of being treated as a nearest
    /// match.
    pub fn stamp(&self, ts_micros: i64) -> Result<VixStamp, String> {
        let slot = slot_of(self.month, ts_micros, "lookup")?;
        let Some(stored) = self.slots.get(slot).copied().flatten() else {
            return Ok(VixStamp::Absent);
        };
        if stored.ts_micros != ts_micros {
            return Err(format!(
                "India VIX civil-month slot {slot} holds timestamp {}, not requested exact timestamp {ts_micros}; interpolation and nearest-minute fallback are forbidden",
                stored.ts_micros
            ));
        }
        Ok(VixStamp::Exact(stored))
    }
}

fn validate_bar(record: u64, bar: &Bar) -> Result<(), String> {
    if !bar.ohlc_is_sane() {
        return Err(format!(
            "stored record {record} India VIX OHLC is corrupt at timestamp {}: open={} high={} low={} close={}",
            bar.ts_micros, bar.open, bar.high, bar.low, bar.close
        ));
    }
    if !bar.counts_are_sane() {
        return Err(format!(
            "stored record {record} India VIX counts are corrupt at timestamp {}: volume={} open_interest={}",
            bar.ts_micros, bar.volume, bar.open_interest
        ));
    }
    Ok(())
}

fn slot_of(month: YearMonth, ts_micros: i64, context: &str) -> Result<usize, String> {
    if ts_micros.rem_euclid(MICROS_PER_MINUTE) != 0 {
        return Err(format!(
            "{context} India VIX timestamp {ts_micros} is off the exact one-minute grid"
        ));
    }
    let seconds = ts_micros.div_euclid(1_000_000);
    let moment = IstMoment::from_epoch_secs(seconds).map_err(|why| {
        format!("{context} India VIX timestamp {ts_micros} cannot name an IST minute: {why}")
    })?;
    let day = moment.day();
    let found = day.year_month().map_err(|why| {
        format!("{context} India VIX timestamp {ts_micros} has no storable IST month: {why}")
    })?;
    if found != month {
        return Err(format!(
            "{context} India VIX timestamp {ts_micros} belongs to IST month {found}, not requested month {month}"
        ));
    }

    let day_zero = usize::from(day.day().saturating_sub(1));
    let minute = usize::try_from(moment.minute_of_day()).map_err(|_| {
        format!(
            "{context} India VIX timestamp {ts_micros} has an unaddressable IST minute-of-day {}",
            moment.minute_of_day()
        )
    })?;
    day_zero
        .checked_mul(MINUTES_PER_DAY)
        .and_then(|base| base.checked_add(minute))
        .filter(|&slot| slot < CIVIL_MONTH_MINUTE_SLOTS)
        .ok_or_else(|| {
            format!(
                "{context} India VIX timestamp {ts_micros} cannot fit the fixed {CIVIL_MONTH_MINUTE_SLOTS}-slot civil-month index"
            )
        })
}

const fn candle_of(bar: Bar) -> Candle {
    Candle {
        ts_micros: bar.ts_micros,
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.volume,
        open_interest: bar.open_interest,
    }
}

#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "fixtures use the store's exact low-32-bit symbol id, direct byte offsets to model post-write corruption, and assertions that must be able to fail"
)]
mod tests {
    use std::fs::OpenOptions;
    use std::os::unix::fs::FileExt;
    use std::path::{Path, PathBuf};

    use super::*;
    use store::file::StoreError;
    use store::layout::Layout;

    const YEAR: u16 = 2026;
    const MONTH: u8 = 8;

    fn root(tag: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("brutex-vix-reference-{tag}-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&path);
        path
    }

    const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
        let adjusted_year = if month <= 2 { year - 1 } else { year };
        let era = if adjusted_year >= 0 {
            adjusted_year
        } else {
            adjusted_year - 399
        } / 400;
        let year_of_era = adjusted_year - era * 400;
        let shifted_month = (month + 9) % 12;
        let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    const fn ist_minute(year: i64, month: i64, day: i64, minute: i64) -> i64 {
        (days_from_civil(year, month, day) * 1_440 + minute) * MICROS_PER_MINUTE
            - indicators::IST_OFFSET_MICROS
    }

    fn bar(ts_micros: i64, offset: i64) -> Bar {
        Bar {
            ts_micros,
            open: 1_500_000 + offset,
            high: 1_500_100 + offset,
            low: 1_499_900 + offset,
            close: 1_500_050 + offset,
            volume: 7 + offset,
            open_interest: i64::MIN,
        }
    }

    fn month() -> YearMonth {
        YearMonth::new(YEAR, MONTH).expect("a real month")
    }

    fn vix_path(vendor: Vendor, month: YearMonth, key: &InstrumentKey) -> StorePath<'_> {
        StorePath::for_key(vendor, key, Timeframe::MINUTE_1, month, FileKind::Bars)
            .expect("a VIX bar path")
    }

    fn seed(tag: &str, vendor: Vendor, bars: &[Bar]) -> (PathBuf, PathBuf) {
        let root = root(tag);
        let key = InstrumentKey::index(Exchange::Nse, VIX_REFERENCE_SYMBOL)
            .expect("the fixed VIX key is valid");
        let path = vix_path(vendor, month(), &key);
        let file_path = path.to_path_buf(&root);
        let symbol_id = brutex_core::universe::fnv1a(VIX_REFERENCE_SYMBOL) as u32;
        let mut file = BarFile::open_or_create(&root, path, symbol_id).expect("VIX month opens");
        if !bars.is_empty() {
            file.append(bars).expect("fixture bars append");
        }
        drop(file);
        (root, file_path)
    }

    fn overwrite_i64(file_path: &Path, record: u64, field: u64, value: i64, n_valid: u64) {
        let file = OpenOptions::new()
            .write(true)
            .open(file_path)
            .expect("fixture file opens for corruption");
        let offset = Layout::CURRENT
            .offset_of(record)
            .expect("fixture offset")
            .checked_add(field)
            .expect("field offset");
        file.write_all_at(&value.to_le_bytes(), offset)
            .expect("fixture corruption lands");
        file.sync_all().expect("fixture corruption is visible");
        drop(file);
        // AND RESEALED, so these tests keep testing what they are named for.
        //
        // Reads now verify the block checksum, which catches a poked byte
        // before any semantic check runs. That is the point of the checksum --
        // but unresealed it would leave `reordered_unique_timestamps_are_refused`
        // and its two siblings asserting on a checksum message and never
        // reaching the timestamp, OHLC and count rules they exist to prove.
        //
        // A corruption that arrives with a MATCHING seal is also the harder
        // case and the one worth proving here: the bytes are internally
        // consistent -- exactly what a vendor sending bad data produces, since
        // it would have been sealed correctly on the way in -- and the reader
        // has to refuse them on their meaning alone.
        //
        // The checksum's own path is covered separately, by
        // `store::file::tests::a_flipped_byte_in_a_committed_block_is_refused_and_names_the_block`.
        reseal_block(file_path, n_valid);
    }

    /// Recomputes the committed block seal after a fixture edits a record.
    ///
    /// Built from the public primitives the writer itself uses --
    /// `Layout::covered_byte_range` and `block::seal` -- rather than from a
    /// reseal entry point on `BarFile`. A public "make the checksum match
    /// whatever the bytes are now" is a rubber stamp, and the one caller that
    /// wants it is a test fixture.
    fn reseal_block(file_path: &Path, n_valid: u64) {
        let (from, to) = Layout::CURRENT
            .covered_byte_range(0, n_valid)
            .expect("fixture block 0 has a covered range");
        let len = usize::try_from(to.saturating_sub(from)).expect("covered range fits usize");
        let mut covered = vec![0_u8; len];
        let bars = OpenOptions::new()
            .read(true)
            .open(file_path)
            .expect("fixture file opens to reseal");
        bars.read_exact_at(&mut covered, from)
            .expect("the committed block reads back");
        let seal = store::block::seal(Layout::CURRENT, n_valid, 0, &covered)
            .expect("the edited block seals");
        let sidecar = file_path.with_extension("crc");
        OpenOptions::new()
            .write(true)
            .open(&sidecar)
            .expect("fixture sidecar opens")
            .write_all_at(&seal.to_le_bytes(), 0)
            .expect("the fresh seal lands");
    }

    #[test]
    fn exact_lookup_carries_all_seven_fields_and_a_hole_stays_typed_absent() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let second = first + 2 * MICROS_PER_MINUTE;
        let offered = [bar(first, 0), bar(second, 2)];
        let (root, _) = seed("exact", Vendor::Dhan, &offered);

        let index = VixReferenceMonth::open(&root, Vendor::Dhan, month())
            .expect("the exact VIX month is valid");
        assert_eq!(index.vendor(), Vendor::Dhan);
        assert_eq!(index.month(), month());
        assert_eq!(index.records(), 2);
        assert_eq!(
            index.stamp(first),
            Ok(VixStamp::Exact(candle_of(offered[0])))
        );
        assert_eq!(
            index.stamp(first + MICROS_PER_MINUTE),
            Ok(VixStamp::Absent),
            "a missing minute was interpolated, zero-filled, or inherited"
        );
        assert_eq!(
            index.stamp(second),
            Ok(VixStamp::Exact(candle_of(offered[1])))
        );
    }

    #[test]
    fn the_reference_door_does_not_weaken_the_stored_sweep_guard() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let (root, _) = seed("guard", Vendor::Dhan, &[bar(first, 0)]);

        VixReferenceMonth::open(&root, Vendor::Dhan, month())
            .expect("the dedicated reference door opens VIX");
        let why = crate::stored::load(
            &root,
            Vendor::Dhan,
            VIX_REFERENCE_SYMBOL,
            "1min",
            YEAR,
            MONTH,
        )
        .expect_err("the sweep door must still refuse the reference index");
        assert!(
            why.contains("not an instrument this engine sweeps"),
            "{why}"
        );
    }

    #[test]
    fn an_off_grid_stored_bar_and_an_off_grid_lookup_both_refuse() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let (root, _) = seed("off-grid", Vendor::Dhan, &[bar(first + 1, 0)]);
        let why = VixReferenceMonth::open(&root, Vendor::Dhan, month())
            .expect_err("an off-grid stored timestamp is corrupt");
        assert!(why.contains("off the exact one-minute grid"), "{why}");

        let (root, _) = seed("lookup-grid", Vendor::Dhan, &[bar(first, 0)]);
        let index = VixReferenceMonth::open(&root, Vendor::Dhan, month()).expect("valid month");
        let why = index
            .stamp(first + 1)
            .expect_err("an off-grid question is not absence");
        assert!(why.contains("off the exact one-minute grid"), "{why}");
    }

    #[test]
    fn a_wrong_month_stored_bar_and_a_wrong_month_lookup_both_refuse() {
        let august = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let september = ist_minute(2026, 9, 1, 9 * 60 + 15);
        let (root, _) = seed("wrong-month", Vendor::Dhan, &[bar(september, 0)]);
        let why = VixReferenceMonth::open(&root, Vendor::Dhan, month())
            .expect_err("a foreign-month row cannot enter the index");
        assert!(why.contains("IST month 2026-09"), "{why}");
        assert!(why.contains("requested month 2026-08"), "{why}");

        let (root, _) = seed("lookup-month", Vendor::Dhan, &[bar(august, 0)]);
        let index = VixReferenceMonth::open(&root, Vendor::Dhan, month()).expect("valid month");
        let why = index
            .stamp(september)
            .expect_err("a foreign month is not an absent slot");
        assert!(why.contains("IST month 2026-09"), "{why}");
    }

    #[test]
    fn duplicate_exact_timestamps_are_refused_even_after_the_store_was_corrupted() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let bars = [bar(first, 0), bar(first + MICROS_PER_MINUTE, 1)];
        let (root, file_path) = seed("duplicate", Vendor::Dhan, &bars);
        overwrite_i64(&file_path, 1, 0, first, 2);

        let why = VixReferenceMonth::open(&root, Vendor::Dhan, month())
            .expect_err("the duplicate must not overwrite the first slot");
        assert!(
            why.contains("duplicates exact India VIX timestamp"),
            "{why}"
        );
    }

    #[test]
    fn reordered_unique_timestamps_are_refused() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let bars = [
            bar(first, 0),
            bar(first + MICROS_PER_MINUTE, 1),
            bar(first + 2 * MICROS_PER_MINUTE, 2),
        ];
        let (root, file_path) = seed("reordered", Vendor::Dhan, &bars);
        overwrite_i64(&file_path, 1, 0, first + 3 * MICROS_PER_MINUTE, 3);

        let why = VixReferenceMonth::open(&root, Vendor::Dhan, month())
            .expect_err("reordered evidence must not publish an index");
        assert!(why.contains("is not strictly after"), "{why}");
        assert!(why.contains("reordered reference evidence"), "{why}");
    }

    #[test]
    fn post_write_ohlc_and_count_corruption_are_refused() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let (ohlc_root, ohlc_path) = seed("bad-ohlc", Vendor::Dhan, &[bar(first, 0)]);
        overwrite_i64(&ohlc_path, 0, 16, 1, 1);
        let why = VixReferenceMonth::open(&ohlc_root, Vendor::Dhan, month())
            .expect_err("an impossible high is corrupt");
        assert!(why.contains("OHLC is corrupt"), "{why}");

        let (count_root, count_path) = seed("bad-count", Vendor::Dhan, &[bar(first, 0)]);
        overwrite_i64(&count_path, 0, 40, -1, 1);
        let why = VixReferenceMonth::open(&count_root, Vendor::Dhan, month())
            .expect_err("negative volume is corrupt");
        assert!(why.contains("counts are corrupt"), "{why}");
        assert!(why.contains("volume=-1"), "{why}");
    }

    #[test]
    fn feed_and_whole_month_absence_refuse_instead_of_borrowing_another_source() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let (store_root, _) = seed("feed", Vendor::Dhan, &[bar(first, 0)]);
        let why = VixReferenceMonth::open(&store_root, Vendor::Groww, month())
            .expect_err("Groww must not borrow Dhan's reference month");
        assert!(why.contains("groww"), "{why}");
        assert!(why.contains("does not exist"), "{why}");

        let missing = root("missing");
        let why = VixReferenceMonth::open(&missing, Vendor::Dhan, month())
            .expect_err("a missing whole month is not a month of absent minute stamps");
        assert!(why.contains("dhan"), "{why}");
        assert!(why.contains("does not exist"), "{why}");
    }

    #[test]
    fn store_open_failures_remain_named_and_are_not_downgraded_to_absence() {
        let first = ist_minute(2026, 8, 3, 9 * 60 + 15);
        let (root, _) = seed("locked", Vendor::Dhan, &[bar(first, 0)]);
        let key = InstrumentKey::index(Exchange::Nse, VIX_REFERENCE_SYMBOL)
            .expect("the fixed VIX key is valid");
        let path = vix_path(Vendor::Dhan, month(), &key);
        let symbol_id = brutex_core::universe::fnv1a(VIX_REFERENCE_SYMBOL) as u32;
        let _writer = BarFile::open_or_create(&root, path, symbol_id).expect("writer owns month");
        let why = VixReferenceMonth::open(&root, Vendor::Dhan, month())
            .expect_err("a live writer must refuse the reference reader");
        assert!(why.contains("another writer holds"), "{why}");
        assert!(
            !matches!(
                BarFile::open_existing(&root, path, symbol_id),
                Err(StoreError::Missing { .. })
            ),
            "the fixture exists; lock must not be reclassified as missing"
        );
    }
}
