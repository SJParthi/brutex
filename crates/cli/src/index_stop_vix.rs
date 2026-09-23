//! Original India VIX annotations for immutable single-stop trade observations.
//! This separate publication cannot enter strategy/source/search identities,
//! ranking or prices. Cold capture and validation use bounded sorting of trade
//! boundaries; warm pages use direct setting/trade extents. Inspection never
//! opens the market store.
use crate::candidate_universe::boolean_candidate_v1::persistence::{self, Observation};
use crate::index_stop_store::{Reader as Catalog, Trade};
use crate::vix_reference::{VixReferenceMonth, VixStamp};
use brutex_core::blake3::{Hasher, hash};
use brutex_core::vendor::Vendor;
use indicators::Candle;
use std::path::Path;
use store::path::YearMonth;

#[path = "index_stop_vix_codec.rs"]
mod codec;

pub(crate) const NAMESPACE: &str = "index-stop-vix-reference-v1";
const RECEIPT_BYTES: u64 = 112;
const MAX_REASON_BYTES: usize = 8192;
/// The exit annotation describes the printed exit minute, not a tick instant.
pub const POLICY: &str = "same-feed NSE-INDIAVIX 1min; original entry-minute and exit-minute full candles; exact absence differs from unavailable month; reference only; exit interval retained; no interpolation";

/// Independent current resource limits; none becomes a strategy parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Aggregate encoded catalog/companion/receipt bytes.
    pub bytes: u64,
    /// Maximum native evidence and companion trade records.
    pub records: u64,
    /// Conservative overlapping buffer admission, not measured process RSS.
    pub memory_bytes: u64,
    /// Maximum annotation rows in one page.
    pub page_records: u64,
}
impl Bounds {
    fn validate(self) -> Result<(), String> {
        let addressable = u64::try_from(isize::MAX).map_err(display)?;
        if self.bytes == 0
            || self.records == 0
            || self.memory_bytes == 0
            || self.page_records == 0
            || self.bytes > addressable
            || self.memory_bytes > addressable
        {
            return Err("VIX reference bounds must be positive and addressable".into());
        }
        Ok(())
    }
}

/// One saved exact-minute annotation. Unavailability is explained by its month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stamp {
    /// All seven original stored OHLCV/OI fields.
    Exact(Candle),
    /// A fully validated opened month had no bar at this exact minute.
    Absent,
    /// The canonical month loader refused; see the saved month diagnostic.
    Unavailable,
}

/// Original reference-month provenance, never a claim about today's store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Month {
    /// Original IST civil year.
    pub year: u16,
    /// Original IST civil month.
    pub month: u8,
    /// Full committed row count when canonical validation succeeded.
    pub records: Option<u64>,
    /// Hash of the original validated full minute-slot snapshot, if available.
    pub snapshot_digest: Option<[u8; 32]>,
    /// Literal bounded canonical-loader refusal; never confused with a hole.
    pub unavailable_reason: Option<String>,
}
impl Month {
    /// Stable unavailable category; the original diagnostic supplies detail.
    #[must_use]
    pub fn unavailable_code(&self) -> Option<&'static str> {
        self.unavailable_reason
            .as_ref()
            .map(|_| "reference_month_refused")
    }
}

/// Exact saved trade binding and its two reference stamps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Original setting-local trade ordinal.
    pub trade_index: u64,
    /// Exact native strategy/run identity, unchanged by VIX.
    pub run_id: [u8; 32],
    /// Hash of the unchanged complete native V1 trade bytes.
    pub trade_digest: [u8; 32],
    /// Exact entry-minute open timestamp.
    pub entry_micros: i64,
    /// Printed exit minute's open, 15:09 for a forced 15:10 close.
    pub exit_bar_micros: i64,
    /// Native earliest exit instant; no sub-minute fill is invented.
    pub exit_from_micros: i64,
    /// Native exclusive stop interval end, or the exact forced close.
    pub exit_until_micros: i64,
    /// Original month provenance index, accessible through `View::month`.
    pub month_index: u64,
    /// Original entry-minute reference candle/absence/refusal.
    pub entry: Stamp,
    /// Original exit-minute reference candle/absence/refusal.
    pub exit: Stamp,
}

/// Independent saved reference publication identity and original feed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    /// Exact native catalog identity.
    pub catalog_identity: [u8; 32],
    /// Exact native completion receipt, required on every page.
    pub catalog_completion: [u8; 32],
    /// Independent immutable annotation body address.
    pub lookup_identity: [u8; 32],
    /// Original VIX policy/stamp publication hash; never a strategy identity.
    pub publication_id: [u8; 32],
    /// Exact saved companion completion receipt.
    pub completion_digest: [u8; 32],
    /// Original canonical stored feed, supplied by the admitted native producer.
    pub feed: String,
    /// Total settings, including those with zero trades.
    pub settings: u64,
    /// Total annotated native trades, including explicit unavailable stamps.
    pub trades: u64,
    /// Count of exact individual stamps across both trade boundaries.
    pub exact_stamps: u64,
    /// Count of exact-minute holes in valid opened months.
    pub absent_stamps: u64,
    /// Count of boundaries whose original reference month was unavailable.
    pub unavailable_stamps: u64,
}

struct Setting {
    run_id: [u8; 32],
    source_id: [u8; 32],
    first: u64,
    count: u64,
}
struct Image {
    meta: Metadata,
    settings: Vec<Setting>,
    rows: Vec<Row>,
    months: Vec<Month>,
}

/// Cached immutable native catalog plus its independently sealed VIX snapshot.
pub struct Reader {
    catalog: Catalog,
    observation: Observation,
    image: Image,
    bounds: Bounds,
    admitted_bytes: u64,
}

/// Only publication ownership is retained by native execution authority.
/// This contains no decoded VIX/candidate rows and no numerical inputs.
pub(crate) struct Publication {
    observation: Observation,
    catalog_identity: [u8; 32],
    catalog_completion: [u8; 32],
    scratch_bytes: u64,
}
impl Publication {
    pub(crate) fn with_current<T>(
        &self,
        catalog: &Catalog,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        if catalog.identity() != self.catalog_identity
            || catalog.completion_digest() != self.catalog_completion
        {
            return Err("retained VIX publication names a different native catalog".into());
        }
        Observation::with_current_many(
            &[catalog.observation(), &self.observation],
            self.scratch_bytes,
            project,
        )
    }
}
/// Projection guarded by both distinct publication owners for its lifetime.
pub struct View<'a> {
    reader: &'a Reader,
}
/// A bounded borrowed page; zero-trade settings remain valid empty pages.
pub struct Page<'a> {
    /// Exact selected setting.
    pub setting: u64,
    /// First requested setting-local trade ordinal.
    pub offset: u64,
    /// Total native trades in the selected setting.
    pub total: u64,
    /// Next exact ordinal, or None at the saved end.
    pub next_offset: Option<u64>,
    /// Original annotations in native trade order.
    pub rows: &'a [Row],
}

impl Reader {
    pub(crate) fn into_publication(self) -> Publication {
        Publication {
            catalog_identity: self.catalog.identity(),
            catalog_completion: self.catalog.completion_digest(),
            observation: self.observation,
            scratch_bytes: self.bounds.memory_bytes,
        }
    }
    /// Authenticate an entire saved catalog/companion before exposing any page.
    /// # Errors
    /// Missing legacy context, foreign pins, corruption, changed owners or
    /// independent byte/record/memory admission refuses without any write.
    pub fn open(
        root: &Path,
        catalog_identity: [u8; 32],
        catalog_pin: [u8; 32],
        bounds: Bounds,
    ) -> Result<Self, String> {
        bounds.validate()?;
        if catalog_identity == [0; 32] || catalog_pin == [0; 32] {
            return Err(
                "VIX reference requires exact nonzero catalog identity and completion".into(),
            );
        }
        let catalog = Catalog::open(
            root,
            catalog_identity,
            bounds.bytes.min(bounds.memory_bytes / 4),
            bounds.records,
        )?;
        if catalog.completion_digest() != catalog_pin {
            return Err("VIX reference catalog completion differs".into());
        }
        let remaining = bounds
            .bytes
            .checked_sub(catalog.admitted_bytes())
            .ok_or("VIX aggregate byte admission exceeded")?;
        let lookup = lookup_identity(catalog_identity, catalog_pin);
        let (observation,mut image) = Observation::open(root,NAMESPACE,lookup,remaining,|raw| {
            let scratch = require_read_cost(catalog.admitted_bytes(),raw.len() as u64 + RECEIPT_BYTES,bounds)?;
            codec::decode(raw,catalog_identity,catalog_pin,bounds.records,scratch)
        }).map_err(|why| format!("saved VIX reference companion unavailable or invalid; no current-data substitution: {why}"))?;
        image.meta.completion_digest = observation.completion_digest();
        let admitted_bytes = catalog
            .admitted_bytes()
            .checked_add(observation.body_bytes())
            .and_then(|n| n.checked_add(RECEIPT_BYTES))
            .ok_or("VIX observation byte count overflow")?;
        let reader = Self {
            catalog,
            observation,
            image,
            bounds,
            admitted_bytes,
        };
        reader.with_current(|view| verify_catalog(&view.reader.image, &view.reader.catalog))?;
        Ok(reader)
    }
    /// Cheap generation checks under a compound lease; no history is rescanned.
    /// # Errors
    /// Changed, busy or removed saved authorities refuse.
    pub fn require_current(&self) -> Result<(), String> {
        self.with_current(|_| Ok(()))
    }
    /// Hold both distinct artifact owners throughout the complete projection.
    /// Use only View getters inside; nested reader entry is explicitly refused.
    /// # Errors
    /// Owner/generation/scratch failures or callback errors refuse.
    pub fn with_current<T>(
        &self,
        project: impl FnOnce(&View<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        Observation::with_current_many(
            &[self.catalog.observation(), &self.observation],
            self.bounds.memory_bytes,
            || project(&View { reader: self }),
        )
    }
}
impl View<'_> {
    /// Original immutable publication and aggregate availability counts.
    #[must_use]
    pub fn metadata(&self) -> &Metadata {
        &self.reader.image.meta
    }
    /// Aggregate encoded catalog and companion bytes already admitted.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.reader.admitted_bytes
    }
    /// Independent current memory/byte/page admission.
    #[must_use]
    pub const fn bounds(&self) -> Bounds {
        self.reader.bounds
    }
    /// Exact already authenticated native setting, with no nested lock.
    /// # Errors
    /// A foreign setting ordinal refuses.
    pub fn native_record(
        &self,
        setting: usize,
    ) -> Result<&crate::index_stop_store::Record, String> {
        self.reader
            .catalog
            .records()
            .get(setting)
            .ok_or_else(|| "VIX setting is outside the native catalog".into())
    }
    /// Original unmodified native trade for direct shared JSON projection.
    /// # Errors
    /// Foreign setting or trade ordinals refuse.
    pub fn original_trade(&self, setting: usize, index: u64) -> Result<&Trade, String> {
        self.native_record(setting)?
            .trades()
            .get(address(index)?)
            .ok_or_else(|| "VIX trade is outside the native setting".into())
    }
    /// Exact original provenance for a row's month index.
    /// # Errors
    /// A foreign month ordinal refuses.
    pub fn month(&self, index: u64) -> Result<&Month, String> {
        self.reader
            .image
            .months
            .get(address(index)?)
            .ok_or_else(|| "VIX month is outside the saved extent".into())
    }
    /// Direct immutable page access, without reloading or nested lease entry.
    /// # Errors
    /// Foreign setting/offset, zero limit, overflow or excessive page refuses.
    pub fn page(&self, setting: usize, offset: u64, limit: u64) -> Result<Page<'_>, String> {
        let selected = self
            .reader
            .image
            .settings
            .get(setting)
            .ok_or("VIX setting is outside the saved catalog")?;
        if limit == 0 || limit > self.reader.bounds.page_records || offset > selected.count {
            return Err("VIX page exceeds the saved extent or current page admission".into());
        }
        let end = offset
            .checked_add(limit)
            .ok_or("VIX page arithmetic overflow")?
            .min(selected.count);
        let first = selected
            .first
            .checked_add(offset)
            .ok_or("VIX page coordinate overflow")?;
        let last = selected
            .first
            .checked_add(end)
            .ok_or("VIX page coordinate overflow")?;
        let rows = self
            .reader
            .image
            .rows
            .get(address(first)?..address(last)?)
            .ok_or("VIX page extent is invalid")?;
        Ok(Page {
            setting: setting as u64,
            offset,
            total: selected.count,
            next_offset: (end < selected.count).then_some(end),
            rows,
        })
    }
}

pub(crate) fn publish(
    root: &Path,
    store: &Path,
    feed: Vendor,
    catalog: &Catalog,
    bounds: Bounds,
) -> Result<Reader, String> {
    bounds.validate()?;
    let identity = catalog.identity();
    let pin = catalog.completion_digest();
    let lookup = lookup_identity(identity, pin);
    let directory = root.join(NAMESPACE).join(crate::identity_hex(&lookup));
    match std::fs::symlink_metadata(&directory) {
        Ok(_) => {
            let saved = Reader::open(root, identity, pin, bounds)?;
            if saved.image.meta.feed != feed.as_str() {
                return Err("saved VIX reference feed differs from the native source".into());
            }
            return Ok(saved);
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
        Err(why) => return Err(display(why)),
    }
    catalog.with_current(|| {
        let image = capture(store, feed, catalog, bounds)?;
        let body = codec::encode(&image, bounds.bytes)?;
        require_read_cost(
            catalog.admitted_bytes(),
            body.len() as u64 + RECEIPT_BYTES,
            bounds,
        )?;
        let digest = hash(&body);
        let pending = persistence::prepare_in_namespace(root, NAMESPACE, lookup, &body)?;
        pending.verify_body(digest, body.len() as u64)?;
        pending.finish(lookup, digest, body.len() as u64)?;
        Ok(())
    })?;
    Reader::open(root, identity, pin, bounds)
}

#[derive(Clone, Copy)]
struct Query {
    month: YearMonth,
    row: usize,
}

fn capture(store: &Path, feed: Vendor, catalog: &Catalog, bounds: Bounds) -> Result<Image, String> {
    let trades = catalog.records().iter().try_fold(0_u64, |count, row| {
        count
            .checked_add(row.trades().len() as u64)
            .ok_or("VIX trade count overflow")
    })?;
    if trades > bounds.records {
        return Err("VIX complete trade count exceeds record admission".into());
    }
    let months = month_ceiling(catalog)?;
    let maximum = codec::maximum_bytes(catalog.records().len() as u64, trades, months)?;
    require_capture_cost(catalog.admitted_bytes(), maximum, trades, bounds)?;
    let mut image = empty_image(catalog, feed, trades);
    image
        .settings
        .try_reserve_exact(catalog.records().len())
        .map_err(display)?;
    image
        .rows
        .try_reserve_exact(address(trades)?)
        .map_err(display)?;
    image
        .months
        .try_reserve_exact(address(months)?)
        .map_err(display)?;
    let mut queries = Vec::new();
    queries
        .try_reserve_exact(address(trades)?)
        .map_err(display)?;
    for record in catalog.records() {
        image.settings.push(Setting {
            run_id: record.run_id(),
            source_id: record.source_id(),
            first: image.rows.len() as u64,
            count: record.trades().len() as u64,
        });
        for (index, trade) in record.trades().iter().enumerate() {
            let month = month_of(trade.entry_micros)?;
            if month != month_of(trade.exit_bar_micros)? {
                return Err("intraday VIX entry and exit do not name the same civil month".into());
            }
            queries.push(Query {
                month,
                row: image.rows.len(),
            });
            image.rows.push(Row {
                trade_index: index as u64,
                run_id: record.run_id(),
                trade_digest: hash(&trade.canonical_bytes()),
                entry_micros: trade.entry_micros,
                exit_bar_micros: trade.exit_bar_micros,
                exit_from_micros: trade.exit_from_micros,
                exit_until_micros: trade.exit_until_micros,
                month_index: 0,
                entry: Stamp::Unavailable,
                exit: Stamp::Unavailable,
            });
        }
    }
    queries.sort_unstable_by_key(|query| (query.month.year(), query.month.month(), query.row));
    let mut current: Option<(YearMonth, Option<VixReferenceMonth>)> = None;
    for query in queries {
        if current
            .as_ref()
            .is_none_or(|(month, _)| *month != query.month)
        {
            // Drop the previous fixed month index before admitting another.
            drop(current.take());
            let (provenance, loaded) = load_month(store, feed, query.month)?;
            image.months.push(provenance);
            current = Some((query.month, loaded));
        }
        let row = image
            .rows
            .get_mut(query.row)
            .ok_or("VIX capture row disappeared")?;
        row.month_index = image
            .months
            .len()
            .checked_sub(1)
            .ok_or("VIX capture month disappeared")? as u64;
        if let Some((_, Some(month))) = &current {
            row.entry = convert(month.stamp(row.entry_micros)?);
            row.exit = convert(month.stamp(row.exit_bar_micros)?);
        }
    }
    count_stamps(&mut image)?;
    Ok(image)
}

fn empty_image(catalog: &Catalog, feed: Vendor, trades: u64) -> Image {
    Image {
        meta: Metadata {
            catalog_identity: catalog.identity(),
            catalog_completion: catalog.completion_digest(),
            lookup_identity: lookup_identity(catalog.identity(), catalog.completion_digest()),
            publication_id: [0; 32],
            completion_digest: [0; 32],
            feed: feed.as_str().into(),
            settings: catalog.records().len() as u64,
            trades,
            exact_stamps: 0,
            absent_stamps: 0,
            unavailable_stamps: 0,
        },
        settings: Vec::new(),
        rows: Vec::new(),
        months: Vec::new(),
    }
}
fn load_month(
    store: &Path,
    feed: Vendor,
    month: YearMonth,
) -> Result<(Month, Option<VixReferenceMonth>), String> {
    match VixReferenceMonth::open(store, feed, month) {
        Ok(loaded) => Ok((
            Month {
                year: month.year(),
                month: month.month(),
                records: Some(loaded.records()),
                snapshot_digest: Some(loaded.snapshot_digest()),
                unavailable_reason: None,
            },
            Some(loaded),
        )),
        Err(reason) => {
            if reason.is_empty() || reason.len() > MAX_REASON_BYTES {
                return Err("complete VIX refusal diagnostic exceeds explicit record admission; no truncated annotation published".into());
            }
            Ok((
                Month {
                    year: month.year(),
                    month: month.month(),
                    records: None,
                    snapshot_digest: None,
                    unavailable_reason: Some(reason),
                },
                None,
            ))
        }
    }
}
const fn convert(stamp: VixStamp) -> Stamp {
    match stamp {
        VixStamp::Exact(bar) => Stamp::Exact(bar),
        VixStamp::Absent => Stamp::Absent,
    }
}
fn month_of(micros: i64) -> Result<YearMonth, String> {
    if micros.rem_euclid(60_000_000) != 0 {
        return Err("VIX annotation is not an exact minute".into());
    }
    pull::session::IstMoment::from_epoch_secs(micros.div_euclid(1_000_000))
        .map_err(display)?
        .day()
        .year_month()
        .map_err(display)
}
fn month_ceiling(catalog: &Catalog) -> Result<u64, String> {
    let mut first = None;
    let mut last = None;
    for trade in catalog
        .records()
        .iter()
        .flat_map(crate::index_stop_store::Record::trades)
    {
        let month = month_of(trade.entry_micros)?;
        let ordinal = u64::from(month.year()) * 12 + u64::from(month.month());
        first = Some(first.map_or(ordinal, |n: u64| n.min(ordinal)));
        last = Some(last.map_or(ordinal, |n: u64| n.max(ordinal)));
    }
    match (first, last) {
        (Some(a), Some(b)) => b
            .checked_sub(a)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| "VIX month extent overflow".into()),
        _ => Ok(0),
    }
}
fn require_read_cost(catalog: u64, companion: u64, bounds: Bounds) -> Result<u64, String> {
    let bytes = catalog
        .checked_add(companion)
        .ok_or("VIX aggregate byte count overflow")?;
    let memory = bytes
        .checked_mul(4)
        .and_then(|n| n.checked_add(4096))
        .ok_or("VIX aggregate memory count overflow")?;
    if bytes > bounds.bytes || memory > bounds.memory_bytes {
        return Err(format!(
            "complete VIX/catalog inspection requires {bytes} saved bytes and {memory} conservative buffer bytes; current bounds are {}/{}",
            bounds.bytes, bounds.memory_bytes
        ));
    }
    bounds
        .memory_bytes
        .checked_sub(memory)
        .ok_or_else(|| "VIX read scratch admission underflow".into())
}
fn require_capture_cost(
    catalog: u64,
    companion: u64,
    trades: u64,
    bounds: Bounds,
) -> Result<(), String> {
    require_read_cost(catalog, companion, bounds)?;
    let query_bytes = trades
        .checked_mul(std::mem::size_of::<Query>() as u64)
        .ok_or("VIX query buffer overflow")?;
    let month_bytes = (crate::vix_reference::CIVIL_MONTH_MINUTE_SLOTS as u64)
        .checked_mul(std::mem::size_of::<Option<Candle>>() as u64)
        .ok_or("VIX month buffer overflow")?;
    let capture_scratch = query_bytes
        .checked_add(if trades == 0 { 0 } else { month_bytes })
        .ok_or("VIX capture scratch count overflow")?;
    // Capture sorting and cold repeated-minute validation do not overlap.
    // Admit the larger temporary vector/workspace before publishing anything.
    let scratch = capture_scratch.max(codec::validation_bytes(trades)?);
    let memory = catalog
        .checked_add(companion)
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(scratch))
        .and_then(|n| n.checked_add(4096))
        .ok_or("VIX capture memory count overflow")?;
    if memory > bounds.memory_bytes {
        return Err(format!(
            "complete VIX capture requires {memory} conservative buffer bytes; admission is {}; no partial annotations published",
            bounds.memory_bytes
        ));
    }
    Ok(())
}

fn count_stamps(image: &mut Image) -> Result<(), String> {
    let mut counts = [0_u64; 3];
    for stamp in image.rows.iter().flat_map(|row| [row.entry, row.exit]) {
        let slot = match stamp {
            Stamp::Exact(_) => 0,
            Stamp::Absent => 1,
            Stamp::Unavailable => 2,
        };
        let count = counts
            .get_mut(slot)
            .ok_or("VIX stamp category disappeared")?;
        *count = count.checked_add(1).ok_or("VIX stamp count overflow")?;
    }
    let [exact, absent, unavailable] = counts;
    image.meta.exact_stamps = exact;
    image.meta.absent_stamps = absent;
    image.meta.unavailable_stamps = unavailable;
    Ok(())
}
fn verify_catalog(image: &Image, catalog: &Catalog) -> Result<(), String> {
    if image.settings.len() != catalog.records().len() {
        return Err("VIX setting count differs from exact native catalog".into());
    }
    for (saved, record) in image.settings.iter().zip(catalog.records()) {
        if saved.run_id != record.run_id()
            || saved.source_id != record.source_id()
            || saved.count != record.trades().len() as u64
        {
            return Err("VIX setting differs from exact native source/run/trade count".into());
        }
        let end = saved
            .first
            .checked_add(saved.count)
            .ok_or("VIX native range overflow")?;
        let rows = image
            .rows
            .get(address(saved.first)?..address(end)?)
            .ok_or("VIX native range is invalid")?;
        for (index, (row, trade)) in rows.iter().zip(record.trades()).enumerate() {
            if row.trade_index != index as u64
                || row.run_id != record.run_id()
                || row.trade_digest != hash(&trade.canonical_bytes())
                || row.entry_micros != trade.entry_micros
                || row.exit_bar_micros != trade.exit_bar_micros
                || row.exit_from_micros != trade.exit_from_micros
                || row.exit_until_micros != trade.exit_until_micros
            {
                return Err("VIX annotation differs from the exact original native trade".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn lookup_identity(catalog: [u8; 32], pin: [u8; 32]) -> [u8; 32] {
    let mut digest = Hasher::new();
    digest.update(b"brutex-index-stop-vix-reference-lookup-v1\0");
    digest.update(&catalog);
    digest.update(&pin);
    digest.finalize()
}
fn policy_digest() -> [u8; 32] {
    hash(POLICY.as_bytes())
}
fn address(value: u64) -> Result<usize, String> {
    usize::try_from(value).map_err(display)
}
fn display(value: impl std::fmt::Display) -> String {
    value.to_string()
}

#[cfg(test)]
#[path = "index_stop_vix_tests.rs"]
mod tests;
