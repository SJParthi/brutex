//! Immutable original native inputs and vocabulary, shared by a complete catalog.
//! Cold authentication/reconstruction is bounded and linear in admitted history.
//! Inspection never opens the mutable market store or publishes research/audits.
use super::{Context, Limits, Loaded, catalog_identity_for, display};
use crate::candidate_universe::boolean_candidate_v1::persistence::{self, Observation};
use crate::index_stop_store::{Reader as Catalog, Record};
use brutex_core::blake3::{Hasher, hash};
use indicators::Candle;
use runner::identity::Direction;
use runner::signal_candle_stop::{Prepared, Trade};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "index_stop_source_context_codec.rs"]
mod codec;
use codec::Image;

const NAMESPACE: &str = "index-stop-source-context-v1";
const CATALOG_NAMESPACE: &str = "index-stop-catalog-context-v1";
const LINK_BYTES: u64 = 136;
pub(crate) const CATALOG_LINK_BYTES: u64 = LINK_BYTES + 112;

/// Source-view fallback shared by the native producer and its read-only API.
pub const DEFAULT_OBSERVATION_BYTES: u64 = 64 * 1024 * 1024;

/// Resolve the source viewer's independent current limit, without reading data.
/// # Errors
/// Invalid, zero or unaddressable runtime values refuse without substitution.
pub fn observation_budget() -> Result<u64, String> {
    let value = std::env::var_os("BRUTEX_BOOLEAN_OBSERVATION_BYTES");
    observation_budget_value(value.as_deref())
}
fn observation_budget_value(raw: Option<&std::ffi::OsStr>) -> Result<u64, String> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_OBSERVATION_BYTES);
    };
    let invalid = || {
        "BRUTEX_BOOLEAN_OBSERVATION_BYTES must be a canonical positive addressable integer; no source-view default substituted".to_owned()
    };
    let raw = raw.to_str().ok_or_else(invalid)?;
    if raw.is_empty()
        || raw.starts_with('0')
        || raw.len() > 20
        || !raw.bytes().all(|value| value.is_ascii_digit())
    {
        return Err(invalid());
    }
    let value = raw.parse::<u64>().map_err(|_| invalid())?;
    if value > u64::try_from(isize::MAX).map_err(display)? {
        return Err(invalid());
    }
    Ok(value)
}

/// Reproducible conservative cost for one archived source and complete catalog.
/// These are checked admission counts, not measured process memory or latency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationCost {
    /// Serialized archive including its exact completion record.
    pub source_bytes: u64,
    /// Archived OHLCV records across all separately stored input roles.
    pub source_records: u64,
    /// Complete candidate catalog including its completion record.
    pub catalog_bytes: u64,
    /// Archive, catalog and fixed immutable relation bytes together.
    pub required_bytes: u64,
    /// Conservative overlapping decode/rebuild buffers for the whole reader.
    pub required_memory_bytes: u64,
}
impl ObservationCost {
    /// Compute the same checked source and catalog accounting used by the reader.
    /// # Errors
    /// Overflow in any full-source or aggregate buffer extent refuses.
    pub fn new(source_bytes: u64, source_records: u64, catalog_bytes: u64) -> Result<Self, String> {
        let required_bytes = source_bytes
            .checked_add(catalog_bytes)
            .and_then(|value| value.checked_add(CATALOG_LINK_BYTES))
            .ok_or("source observation byte count overflow")?;
        let required_memory_bytes = codec::rebuild_bytes(source_bytes, source_records)?
            .checked_add(
                catalog_bytes
                    .checked_mul(4)
                    .ok_or("catalog buffer count overflow")?,
            )
            .and_then(|value| value.checked_add(CATALOG_LINK_BYTES * 4))
            .ok_or("source observation memory count overflow")?;
        Ok(Self {
            source_bytes,
            source_records,
            catalog_bytes,
            required_bytes,
            required_memory_bytes,
        })
    }
    fn require(self, limit: u64) -> Result<(), String> {
        if self.required_bytes > limit || self.required_memory_bytes > limit {
            return Err(format!(
                "complete original source and catalog require {} serialized bytes and {} conservative reconstruction bytes; source-view admission is {limit}; no uninspectable catalog published",
                self.required_bytes, self.required_memory_bytes
            ));
        }
        Ok(())
    }
}

/// Exact immutable source context and completion; neither is a mutable latest key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    /// Content-addressed original source context.
    pub identity: [u8; 32],
    /// Exact receipt-last completion bytes.
    pub completion: [u8; 32],
}

/// Independent current observation admission, separate from producer settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadBounds {
    /// Aggregate saved catalog, context, link and completion bytes.
    pub bytes: u64,
    /// Catalog evidence record admission.
    pub records: u64,
    /// Sum of archived OHLCV records across all native input roles.
    pub source_records: u64,
    /// Conservative reconstruction-buffer accounting; not a process RSS guarantee.
    pub memory_bytes: u64,
    /// Maximum actual execution candles returned by one exact window.
    pub page_records: u64,
}
impl ReadBounds {
    fn validate(self) -> Result<(), Refusal> {
        if [
            self.bytes,
            self.records,
            self.source_records,
            self.memory_bytes,
            self.page_records,
        ]
        .contains(&0)
        {
            return Err(Refusal::admission(
                "source observation limits must all be positive",
            ));
        }
        Ok(())
    }
}

/// Stable refusal code and explanation; no failed source becomes a current-data fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// Machine-readable refusal category.
    pub code: &'static str,
    /// Exact bounded-operation failure explanation.
    pub message: String,
}
impl Refusal {
    fn invalid(why: impl Into<String>) -> Self {
        Self {
            code: "original_context_refused",
            message: why.into(),
        }
    }
    fn admission(why: impl Into<String>) -> Self {
        Self {
            code: "original_context_admission_refused",
            message: why.into(),
        }
    }
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Refusal {}

/// Original snapshot label; never resolved through the current HTTP vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionName {
    /// Immutable Boolean bit position.
    pub bit: u16,
    /// Name captured from the producing build's vocabulary.
    pub name: String,
}

/// Source and selected-row metadata authenticated before any public projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    /// Exact selected catalog identity, including its source-context pin.
    pub catalog_identity: [u8; 32],
    /// Exact catalog completion.
    pub catalog_completion: [u8; 32],
    /// Original zero-based setting index.
    pub setting: u64,
    /// Original prepared native source identity.
    pub source_id: [u8; 32],
    /// Exact expression, side and measured-window native identity.
    pub run_id: [u8; 32],
    /// Immutable context identity.
    pub source_context_identity: [u8; 32],
    /// Immutable context completion.
    pub source_context_completion: [u8; 32],
    /// Canonical original feed.
    pub feed: String,
    /// Canonical original instrument.
    pub instrument: String,
    /// Original signal timeframe; chart execution candles remain one minute.
    pub timeframe: String,
    /// Producer's original build identity, not the inspecting binary's stamp.
    pub original_build_commit: String,
    /// Original vocabulary version.
    pub vocabulary_version: u32,
    /// Complete saved numerical expression in canonical syntax.
    pub expression: String,
    /// Original preparation first civil day, including the causal prefix.
    pub source_first_day: i64,
    /// Original preparation last civil day.
    pub source_last_day: i64,
    /// Selected record's first measured civil day.
    pub measurement_first_day: i64,
    /// Selected record's last measured civil day.
    pub measurement_last_day: i64,
}

/// An exact contiguous window from the archived one-minute execution stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    /// Original execution ordinal of the first returned candle.
    pub first_bar: u64,
    /// Complete original execution extent.
    pub total_bars: u64,
    /// Real archived candles only; no padding or fabricated timestamps.
    pub candles: Vec<Candle>,
    /// Exact saved trade, including both pessimistic and optimistic readings.
    pub trade: Trade,
}

/// Producer-only authority retaining the immutable snapshot generations.
pub(crate) struct Committed {
    observation: Observation,
    source_id: [u8; 32],
    source_records: u64,
    observation_bytes: u64,
}
impl Committed {
    pub(crate) fn link(&self) -> Link {
        Link {
            identity: self.observation.identity(),
            completion: self.observation.completion_digest(),
        }
    }
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.observation.require_current()
    }
    pub(crate) fn require_catalog_bytes(&self, bytes: u64) -> Result<(), String> {
        ObservationCost::new(
            self.observation
                .body_bytes()
                .checked_add(112)
                .ok_or("source receipt extent overflow")?,
            self.source_records,
            bytes,
        )?
        .require(self.observation_bytes)
    }
}

pub(crate) fn encoded_bytes(loaded: &Loaded) -> Result<u64, String> {
    codec::encoded_bytes(loaded)?
        .checked_add(112)
        .ok_or("source context byte count overflow".into())
}

pub(crate) fn minimum_observation(
    loaded: &Loaded,
    programs: u64,
) -> Result<ObservationCost, String> {
    let catalog = programs
        .checked_mul(2)
        .and_then(|count| count.checked_mul(crate::index_stop_store::RECORD_BYTES as u64))
        .and_then(|bytes| bytes.checked_add(crate::index_stop_store::HEADER_BYTES as u64 + 112))
        .ok_or("minimum source-view catalog size overflow")?;
    ObservationCost::new(encoded_bytes(loaded)?, codec::records(loaded)?, catalog)
}

pub(crate) fn require_minimum_observation(
    loaded: &Loaded,
    programs: u64,
    view_bytes: u64,
) -> Result<(), String> {
    minimum_observation(loaded, programs)?.require(view_bytes)
}

pub(crate) fn publish(
    root: &Path,
    loaded: &Loaded,
    context: &Context<'_>,
    max_bytes: u64,
    observation_bytes: u64,
) -> Result<Committed, String> {
    loaded.require_current()?;
    if !std::ptr::eq(context.origin, loaded) || loaded.source_binding != context.source_binding {
        return Err("source context belongs to a foreign admitted loader".into());
    }
    require_minimum_observation(loaded, 0, observation_bytes)?;
    let body = codec::encode(loaded, context, max_bytes)?;
    publish_body(root, loaded, context, &body, max_bytes, observation_bytes)
}

fn publish_body(
    root: &Path,
    loaded: &Loaded,
    context: &Context<'_>,
    body: &[u8],
    max_bytes: u64,
    observation_bytes: u64,
) -> Result<Committed, String> {
    let identity = context_identity(body);
    let payload = hash(body);
    let length = u64::try_from(body.len()).map_err(display)?;
    let pending = persistence::prepare_in_namespace(root, NAMESPACE, identity, body)?;
    loaded.require_current()?;
    pending.verify_body(payload, length)?;
    pending.finish(identity, payload, length)?;
    drop(pending);
    let (observation, ()) = Observation::open(root, NAMESPACE, identity, max_bytes, |raw| {
        if raw != body {
            return Err("source context changed after publication".into());
        }
        Ok(())
    })?;
    loaded.require_current()?;
    Ok(Committed {
        observation,
        source_id: context.source_id(),
        source_records: codec::records(loaded)?,
        observation_bytes,
    })
}

pub(crate) fn contextual_catalog(legacy: [u8; 32], context: Link) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-index-stop-catalog-v2\0");
    hash.update(&legacy);
    hash.update(&context.identity);
    hash.update(&context.completion);
    hash.finalize()
}
fn context_identity(body: &[u8]) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-index-stop-source-context-v1\0");
    hash.update(body);
    hash.finalize()
}
fn catalog_link_identity(catalog: [u8; 32], pin: [u8; 32]) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-index-stop-catalog-context-v1\0");
    hash.update(&catalog);
    hash.update(&pin);
    hash.finalize()
}

pub(crate) fn bind_catalog(
    root: &Path,
    catalog: &Catalog,
    context: &Committed,
) -> Result<(), String> {
    context.require_current()?;
    if catalog
        .records()
        .iter()
        .any(|row| row.source_id() != context.source_id)
    {
        return Err("catalog context source differs from a saved record".into());
    }
    let mut body = Vec::new();
    body.try_reserve_exact(usize::try_from(LINK_BYTES).map_err(display)?)
        .map_err(display)?;
    body.extend_from_slice(b"BRISCL01");
    let link = context.link();
    for word in [
        catalog.identity(),
        catalog.completion_digest(),
        link.identity,
        link.completion,
    ] {
        body.extend_from_slice(&word);
    }
    let identity = catalog_link_identity(catalog.identity(), catalog.completion_digest());
    let payload = hash(&body);
    let pending = persistence::prepare_in_namespace(root, CATALOG_NAMESPACE, identity, &body)?;
    pending.verify_body(payload, LINK_BYTES)?;
    context.require_current()?;
    catalog.require_current()?;
    pending.finish(identity, payload, LINK_BYTES).map(|_| ())
}

/// Retained exact catalog-to-source relation for inexpensive declared-search reads.
/// It authenticates the immutable relation only; full candles still require `Reader`.
pub struct ContextBinding {
    observation: Observation,
    link: Link,
}
impl ContextBinding {
    pub(crate) const fn observation(&self) -> &Observation {
        &self.observation
    }
    /// Exact saved companion pin carried by this relation.
    #[must_use]
    pub const fn link(&self) -> Link {
        self.link
    }
    /// Fixed body and completion bytes admitted by this relation reader.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.observation.body_bytes() + 112
    }
    /// Revalidate a retained relation without reopening or rebuilding the source.
    /// # Errors
    /// Changed, removed or concurrently published relation bytes refuse.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation.require_current()
    }
}

/// Authenticate the catalog's exact declared source pin without rebuilding candles.
/// # Errors
/// Missing, partial, foreign or changed relations, or insufficient bytes refuse.
pub fn open_catalog_context(
    root: &Path,
    catalog: [u8; 32],
    pin: [u8; 32],
    expected: Link,
    max_bytes: u64,
) -> Result<ContextBinding, String> {
    let identity = catalog_link_identity(catalog, pin);
    let (observation, link) = Observation::open(
        root,
        CATALOG_NAMESPACE,
        identity,
        max_bytes.min(CATALOG_LINK_BYTES),
        |body| decode_link(body, catalog, pin),
    )?;
    if link != expected {
        return Err(
            "saved catalog source companion differs from its immutable search declaration".into(),
        );
    }
    Ok(ContextBinding { observation, link })
}

/// Retained authenticated catalog, original snapshot and immutable relation.
/// Constructing this type never evaluates a strategy or writes an audit/source file.
pub struct Reader {
    catalog: Catalog,
    context: Observation,
    relation: Observation,
    image: Image,
    metadata: Metadata,
    names: Vec<ConditionName>,
    setting: usize,
    direction: Direction,
    bounds: ReadBounds,
    admitted_bytes: u64,
    projection_active: AtomicBool,
}

/// Scoped access to one already authenticated source while all three original
/// catalog, relation and context publication leases remain held.
/// This view never reacquires a same-file lease through a nested accessor.
pub struct View<'a> {
    reader: &'a Reader,
}
impl View<'_> {
    /// Exact original metadata for the selected authenticated setting.
    #[must_use]
    pub const fn metadata(&self) -> &Metadata {
        &self.reader.metadata
    }
    /// Original names used by this setting.
    #[must_use]
    pub fn condition_names(&self) -> &[ConditionName] {
        &self.reader.names
    }
    /// Original long or short direction.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.reader.direction
    }
    /// Full original execution extent, without loading additional candles.
    #[must_use]
    pub fn total_bars(&self) -> u64 {
        self.reader.image.execution().len() as u64
    }
    /// Exact bytes retained by the complete authenticated reader.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.reader.admitted_bytes
    }
    /// Current independent byte admission used for the complete reader.
    #[must_use]
    pub const fn observation_byte_limit(&self) -> u64 {
        self.reader.bounds.bytes
    }
    /// Materialize one exact archived candle window under the existing leases.
    /// # Errors
    /// Invalid trade, arithmetic, source extent or page/allocation admission.
    /// No prefix is returned and no accessor releases/reacquires a held lease.
    pub fn window(&self, trade: usize, before: u64, after: u64) -> Result<Window, Refusal> {
        self.reader.window_leased(trade, before, after)
    }
}

// File locks on the same retained descriptor are not independent recursive
// leases. Reject nested/concurrent entry on one cached Reader so an accidental
// captured Reader accessor cannot release a surrounding View's publication lock.
struct Projection<'a> {
    active: &'a AtomicBool,
}
impl<'a> Projection<'a> {
    fn acquire(active: &'a AtomicBool) -> Result<Self, Refusal> {
        active.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| Refusal::invalid("original source projection is already active; use its guarded view without nesting reader access"))?;
        Ok(Self { active })
    }
}
impl Drop for Projection<'_> {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

impl Reader {
    /// Authenticate exact saved context and rebuild complete native identities.
    /// # Errors
    /// Missing legacy context, mismatched pins, malformed or changed bytes,
    /// unsupported identity/vocabulary versions, or independent admission refusal.
    pub fn open(
        root: &Path,
        identity: [u8; 32],
        pin: [u8; 32],
        setting: usize,
        bounds: ReadBounds,
    ) -> Result<Self, Refusal> {
        bounds.validate()?;
        // Catalog decoding can retain multiple observation vectors. Admit that
        // buffer allowance before the existing cold reader allocates its body.
        let catalog_limit = bounds.bytes.min(bounds.memory_bytes / 4);
        let catalog = Catalog::open(root, identity, catalog_limit, bounds.records)
            .map_err(Refusal::invalid)?;
        catalog.record(pin, setting).map_err(Refusal::invalid)?;
        let remaining = bounds
            .bytes
            .checked_sub(catalog.admitted_bytes())
            .ok_or_else(|| Refusal::admission("catalog exhausts source observation bytes"))?;
        let relation_id = catalog_link_identity(identity, pin);
        let relation_path = root
            .join(CATALOG_NAMESPACE)
            .join(crate::identity_hex(&relation_id));
        if let Err(why) = std::fs::symlink_metadata(&relation_path) {
            return Err(if why.kind() == std::io::ErrorKind::NotFound {
                Refusal { code: "original_context_unavailable", message: "This saved catalog has no immutable original source companion. Existing trades remain available; no current source or vocabulary is substituted.".into() }
            } else {
                Refusal::invalid(format!("source relation is unreadable: {why}"))
            });
        }
        let (relation, link) = Observation::open(
            root,
            CATALOG_NAMESPACE,
            relation_id,
            remaining.min(LINK_BYTES + 112),
            |body| decode_link(body, identity, pin),
        )
        .map_err(Refusal::invalid)?;
        let remaining = remaining
            .checked_sub(relation.body_bytes() + 112)
            .ok_or_else(|| Refusal::admission("source relation exhausts observation bytes"))?;
        let memory_bytes = bounds
            .memory_bytes
            .checked_sub(
                catalog
                    .admitted_bytes()
                    .checked_mul(4)
                    .and_then(|bytes| bytes.checked_add(CATALOG_LINK_BYTES * 4))
                    .ok_or_else(|| Refusal::admission("catalog reconstruction byte overflow"))?,
            )
            .ok_or_else(|| Refusal::admission("catalog exhausts source reconstruction memory"))?;
        let source_bounds = ReadBounds {
            memory_bytes,
            ..bounds
        };
        let (context, image) = Observation::open(
            root,
            NAMESPACE,
            link.identity,
            remaining.min(memory_bytes / 3),
            |body| {
                if context_identity(body) != link.identity {
                    return Err("source snapshot differs from its content identity".into());
                }
                codec::decode(body, source_bounds)
            },
        )
        .map_err(Refusal::invalid)?;
        if context.completion_digest() != link.completion {
            return Err(Refusal::invalid("original source completion pin differs"));
        }
        verify_catalog(&image, &catalog, link).map_err(Refusal::invalid)?;
        let (metadata, names, direction) = selection(&image, &catalog, pin, setting, link)?;
        let admitted_bytes = catalog
            .admitted_bytes()
            .checked_add(relation.body_bytes() + 112)
            .and_then(|n| n.checked_add(context.body_bytes() + 112))
            .filter(|n| *n <= bounds.bytes)
            .ok_or_else(|| {
                Refusal::admission("aggregate source observation exceeds byte admission")
            })?;
        let reader = Self {
            catalog,
            context,
            relation,
            image,
            metadata,
            names,
            setting,
            direction,
            bounds,
            admitted_bytes,
            projection_active: AtomicBool::new(false),
        };
        reader.require_current()?;
        Ok(reader)
    }
    /// Exact original metadata, authenticated together with the selected record.
    #[must_use]
    pub const fn metadata(&self) -> &Metadata {
        &self.metadata
    }
    /// Original snapshot names used by this setting, not today's vocabulary.
    #[must_use]
    pub fn condition_names(&self) -> &[ConditionName] {
        &self.names
    }
    /// Select another already authenticated catalog row without decoding sources
    /// or rebuilding the original indicator column and native identities.
    /// # Errors
    /// An invalid setting, source generation change or allocation refusal leaves
    /// the previous selection intact and supplies no substitute row.
    pub fn select_setting(&mut self, setting: usize) -> Result<(), Refusal> {
        self.require_current()?;
        let link = Link {
            identity: self.context.identity(),
            completion: self.context.completion_digest(),
        };
        let (metadata, names, direction) = selection(
            &self.image,
            &self.catalog,
            self.metadata.catalog_completion,
            setting,
            link,
        )?;
        self.require_current()?;
        self.metadata = metadata;
        self.names = names;
        self.direction = direction;
        self.setting = setting;
        Ok(())
    }
    /// Original long/short setting direction.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }
    /// Complete original one-minute execution record count.
    #[must_use]
    pub fn total_bars(&self) -> u64 {
        self.image.execution().len() as u64
    }
    /// Actual aggregate admitted bytes, including completion records.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.admitted_bytes
    }
    /// Independent observation byte limit used for this retained reader.
    #[must_use]
    pub const fn observation_byte_limit(&self) -> u64 {
        self.bounds.bytes
    }
    /// Cheap held-generation revalidation for a cached, already authenticated reader.
    /// # Errors
    /// Any original artifact generation changed or publication is currently busy.
    pub fn require_current(&self) -> Result<(), Refusal> {
        self.with_current(|_| Ok(()))
    }
    /// Hold all three distinct publication leases across the complete response
    /// projection, including every returned candle and metadata field.
    /// Use the scoped view inside the callback: nested Reader entry is refused.
    /// # Errors
    /// Busy publication, changed generations, nested/concurrent reader entry or
    /// a projection error refuses the whole response and releases every lease.
    pub fn with_current<T>(
        &self,
        project: impl FnOnce(&View<'_>) -> Result<T, String>,
    ) -> Result<T, Refusal> {
        let _projection = Projection::acquire(&self.projection_active)?;
        self.catalog
            .with_current(|| {
                self.relation.with_current(|| {
                    self.context
                        .with_current(|| project(&View { reader: self }))
                })
            })
            .map_err(Refusal::invalid)
    }
    /// Exact requested entry-through-exit window plus original before/after records.
    /// # Errors
    /// Invalid trade, arithmetic, source extent, independent page bound or changed evidence.
    pub fn window(&self, trade: usize, before: u64, after: u64) -> Result<Window, Refusal> {
        self.with_current(|view| Ok(view.window(trade, before, after)))?
    }

    // Only View can reach this helper. Its caller already holds all three
    // publication leases; direct record access cannot unlock any of them.
    fn window_leased(&self, trade: usize, before: u64, after: u64) -> Result<Window, Refusal> {
        let record =
            self.catalog.records().get(self.setting).ok_or_else(|| {
                Refusal::invalid("saved setting is outside its authenticated extent")
            })?;
        let selected = *record.trades().get(trade).ok_or_else(|| {
            Refusal::invalid("saved trade index is outside its exact catalog record")
        })?;
        let first = selected.entry_bar.checked_sub(before).ok_or_else(|| {
            Refusal::invalid("requested candles precede the original execution source")
        })?;
        let end = selected
            .exit_bar
            .checked_add(after)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| Refusal::invalid("requested execution window overflows"))?;
        let count = end
            .checked_sub(first)
            .filter(|n| *n > 0 && *n <= self.bounds.page_records)
            .ok_or_else(|| {
                Refusal::admission("complete requested candle window exceeds page admission")
            })?;
        let source = self.image.execution();
        let range = usize::try_from(first).map_err(|why| Refusal::invalid(why.to_string()))?
            ..usize::try_from(end).map_err(|why| Refusal::invalid(why.to_string()))?;
        let slice = source.get(range).ok_or_else(|| {
            Refusal::invalid(
                "requested candles exceed the original execution source; no clipping or padding",
            )
        })?;
        let mut candles = Vec::new();
        candles
            .try_reserve_exact(
                usize::try_from(count).map_err(|why| Refusal::admission(why.to_string()))?,
            )
            .map_err(|why| Refusal::admission(why.to_string()))?;
        candles.extend_from_slice(slice);
        Ok(Window {
            first_bar: first,
            total_bars: self.total_bars(),
            candles,
            trade: selected,
        })
    }
}

fn selection(
    image: &Image,
    catalog: &Catalog,
    pin: [u8; 32],
    setting: usize,
    link: Link,
) -> Result<(Metadata, Vec<ConditionName>, Direction), Refusal> {
    let row = catalog.record(pin, setting).map_err(Refusal::invalid)?;
    let metadata = image
        .metadata(catalog.identity(), pin, setting, row, link)
        .map_err(Refusal::invalid)?;
    let mut names = Vec::new();
    names
        .try_reserve_exact(image.names.len())
        .map_err(|why| Refusal::admission(why.to_string()))?;
    let mut present = vocab::ConditionMask::ZERO;
    for name in &image.names {
        if row.program().referenced().get(u32::from(name.bit)) {
            names.push(name.clone());
            present = present.with_bit(u32::from(name.bit));
        }
    }
    if present != row.program().referenced() {
        return Err(Refusal::invalid(
            "original snapshot lacks a saved expression's condition name",
        ));
    }
    Ok((metadata, names, row.direction()))
}

fn decode_link(body: &[u8], catalog: [u8; 32], pin: [u8; 32]) -> Result<Link, String> {
    if body.len() as u64 != LINK_BYTES
        || body.get(..8) != Some(b"BRISCL01".as_slice())
        || body.get(8..40) != Some(catalog.as_slice())
        || body.get(40..72) != Some(pin.as_slice())
    {
        return Err("source relation belongs to a foreign catalog or completion".into());
    }
    let link = Link {
        identity: body
            .get(72..104)
            .ok_or("context identity")?
            .try_into()
            .map_err(display)?,
        completion: body
            .get(104..136)
            .ok_or("context completion")?
            .try_into()
            .map_err(display)?,
    };
    if link.identity == [0; 32] || link.completion == [0; 32] {
        return Err("original source relation contains an empty pin".into());
    }
    Ok(link)
}

fn verify_catalog(image: &Image, catalog: &Catalog, link: Link) -> Result<(), String> {
    let column = image.column()?;
    let native = Prepared::new(
        runner::signal_candle_stop::Policy::V1,
        image.native_source(&column)?,
    )
    .map_err(display)?;
    if native.source_id() != image.source_id {
        return Err(
            "archived inputs do not reconstruct the original native source identity".into(),
        );
    }
    let first = catalog
        .records()
        .first()
        .ok_or("saved source catalog is empty")?;
    let mut programs = Vec::new();
    programs
        .try_reserve_exact(catalog.records().len() / 2)
        .map_err(display)?;
    for pair in catalog.records().chunks_exact(2) {
        let row = pair.first().ok_or("saved source pair is absent")?;
        programs.push(row.program().clone());
        for row in pair {
            if row.source_id() != image.source_id
                || row.first_day() != first.first_day()
                || row.last_day() != first.last_day()
                || row.family().instrument() != image.key
                || row.timeframe() != image.timeframe
                || native
                    .run_id_days(
                        row.program(),
                        row.direction(),
                        row.first_day(),
                        row.last_day(),
                    )
                    .map_err(display)?
                    != row.run_id()
            {
                return Err("saved setting differs from its archived source, expression, direction or measurement window".into());
            }
            image.check_trade_coordinates(row)?;
        }
    }
    let legacy = catalog_identity_for(
        image.source_binding,
        &native,
        &programs,
        (first.first_day(), first.last_day()),
        image.limits,
    )?;
    if contextual_catalog(legacy, link) != catalog.identity() {
        return Err(
            "catalog identity does not bind this original source and vocabulary companion".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "index_stop_source_context_tests.rs"]
mod tests;
