//! Vendor rows in, [`store::format::Bar`]s out — with every discarded row
//! counted and every refusal named.
//!
//! # The seam, and why it is where it is
//!
//! `CLAUDE.md` requires this crate to build and test against **fakes, with no
//! live vendor call**. So the transport sits behind [`BarSource`], exactly as
//! the credential store sits behind [`crate::secret::SecretSource`]:
//!
//! - [`FakeSource`] is what every test in this crate uses.
//! - An HTTP or archive implementation is the *only* place a socket or a zip
//!   appears, and it produces the same [`RawWindow`] either way.
//!
//! Everything below [`BarSource`] — the length agreement, the paisa
//! conversion, the session filter, the census — is transport-blind. If the two
//! transports ever needed different downstream code, the seam would be in the
//! wrong place.
//!
//! # The trap that silently loses data
//!
//! A parallel-arrays response is **seven arrays that can disagree in length**.
//! The obvious Rust is `zip`, and `zip` stops at the shortest. A vendor
//! returning 375 opens and 374 volumes would yield 374 perfectly valid-looking
//! bars, and the manifest would record a complete window: data loss with a
//! green checkmark.
//!
//! [`RawWindow::decode`] compares all seven lengths as **one refusal, before
//! any iterator exists**. Once you are zipping, the bug is unobservable —
//! there is no later point at which the missing row can be noticed.
//!
//! # W1: the fault that *stored* a wrong answer
//!
//! A permutation audit found that a vendor stamping its **local IST wall
//! clock** into a field this code read as UTC epoch seconds produced 45 bars at
//! wrong timestamps that passed every validation and were written to disk.
//! Every *other* timestamp fault refuses; that one produced storable data, and
//! once stored there is nothing left to detect it — the file is well formed,
//! the checksum is right, the counter is accurate.
//!
//! The fix is that the encoding is **not assumed**. It is a field on the vendor
//! descriptor ([`vendor::TimestampEncoding`]) and this module dispatches on it.
//! A vendor whose encoding has never been established cannot be added without
//! someone choosing a variant, which is the point.
//!
//! # Cost
//!
//! One pass over the rows. Per row: a fixed number of integer operations, one
//! IST conversion (an add and two divisions) and one census
//! increment. No allocation per row beyond the output vector, which is reserved
//! from the row count before the loop — `docs/07-o1-architecture.md` law 2.

use store::format::Bar;

use crate::session::{Day, DropCensus, SessionError, Window};
use crate::vendor::{Granularity, PriceScale, TimestampEncoding};

/// The largest number of rows one window may return.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary, and
/// unbounded input always arrives from outside. A one-second feed produces
/// 22,500 rows in a session and a snapshot feed a few multiples of that, so
/// this is generous for a single window and still refuses a vendor that
/// answers a one-day request with a decade.
pub const MAX_ROWS: usize = 1_000_000;

/// One row as the vendor sent it, before any interpretation.
///
/// Prices are still in the vendor's own scale and the timestamp is still in the
/// vendor's own encoding. Nothing here has been trusted yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawRow {
    /// Timestamp in whatever [`TimestampEncoding`] the descriptor declares.
    pub timestamp: i64,
    /// Open, in the descriptor's [`PriceScale`].
    pub open: i64,
    /// High.
    pub high: i64,
    /// Low.
    pub low: i64,
    /// Close.
    pub close: i64,
    /// Volume. Always a count, never scaled.
    pub volume: i64,
    /// Open interest, or [`None`] when the vendor omitted it.
    ///
    /// [`None`] and `Some(0)` are different facts and must stay different:
    /// zero open interest is a measurement, an absent field is not.
    pub open_interest: Option<i64>,
}

/// A decoded window: rows, and nothing decided about them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawWindow {
    /// The rows, in the order the vendor sent them.
    pub rows: Vec<RawRow>,
}

/// The seven parallel arrays, as a vendor sends them.
///
/// Taken as a struct rather than seven arguments so a caller cannot transpose
/// `high` and `low` silently — four of the seven are the same type and would
/// swap without complaint.
#[derive(Debug, Clone, Default)]
pub struct ParallelArrays {
    /// Opens.
    pub open: Vec<i64>,
    /// Highs.
    pub high: Vec<i64>,
    /// Lows.
    pub low: Vec<i64>,
    /// Closes.
    pub close: Vec<i64>,
    /// Volumes.
    pub volume: Vec<i64>,
    /// Timestamps.
    pub timestamp: Vec<i64>,
    /// Open interest. Empty means the vendor does not send it at all, which is
    /// different from sending zeros.
    pub open_interest: Vec<i64>,
}

/// Why a fetch produced no window.
///
/// Every variant carries the values it refused. `CLAUDE.md` §4 — degrade loudly
/// and name the reason, or refuse; never both silently.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FetchError {
    /// The seven arrays disagree in length.
    ///
    /// **This is the refusal that exists because of `zip`.** It names every
    /// length so an operator can see which field the vendor truncated, rather
    /// than being told only that something was wrong.
    LengthDisagreement {
        /// Opens.
        open: usize,
        /// Highs.
        high: usize,
        /// Lows.
        low: usize,
        /// Closes.
        close: usize,
        /// Volumes.
        volume: usize,
        /// Timestamps.
        timestamp: usize,
        /// Open interest, when present.
        open_interest: usize,
    },
    /// More rows than [`MAX_ROWS`].
    TooManyRows {
        /// How many arrived.
        rows: usize,
        /// The bound.
        cap: usize,
    },
    /// A timestamp names no instant this build can hold.
    TimestampRefused {
        /// The row's position, so it can be found in the response.
        row: usize,
        /// The value.
        raw: i64,
        /// The calendar's own refusal.
        why: SessionError,
    },
    /// A price does not fit the paisa grid after conversion.
    PriceRefused {
        /// The row's position.
        row: usize,
        /// Which field.
        field: &'static str,
        /// The value, in the vendor's scale.
        raw: i64,
    },
    /// The vendor was reached and answered that it would not serve this.
    ///
    /// Carries the status so a rate refusal (429) is distinguishable from a
    /// broken vendor (5xx) — the first is the governor's business, the second
    /// is not.
    VendorRefused {
        /// HTTP status, or the transport's equivalent.
        status: u16,
        /// Whatever the vendor said, truncated.
        detail: String,
    },
    /// The transport failed before an answer arrived.
    TransportFailed {
        /// What went wrong, in the transport's own words.
        detail: String,
    },
    /// This feed's request carries a rung field and no word for the rung asked
    /// has ever been recorded.
    ///
    /// **Nothing was sent.** The alternative is the one thing worse than a
    /// refusal here: the request that *can* be built is the request for a
    /// different rung, and its answer would be filed under the rung the
    /// operator asked for. `CLAUDE.md` §3 rule 1 — the wire word is a vendor
    /// fact, and there is nothing to derive it from.
    RungNotSpellable {
        /// The rung the operator asked for.
        rung: Granularity,
        /// The request field that would have carried it, so the descriptor row
        /// to amend is named rather than described.
        field: &'static str,
    },
    /// This feed records no word for the instrument's listing class.
    ///
    /// The exact shape of [`Self::RungNotSpellable`], one field over. Groww's
    /// documentation states `CASH` and `FNO` for `segment` and says nothing
    /// about an index; sending a guessed word would be answered with
    /// *something*, and that something would be filed as bars for an
    /// instrument nobody asked for.
    ListingNotSpellable {
        /// The class the instrument belongs to.
        listing: crate::vendor::Listing,
        /// The request field that would have carried it, so the descriptor row
        /// to amend is named rather than described.
        field: &'static str,
    },
    /// The credential handed to a source does not match what its scheme names.
    ///
    /// A wiring fault, caught where the credential arrives rather than on the
    /// wire. A two-secret scheme handed one secret would send
    /// `token :access_token` — a syntactically valid header the vendor answers
    /// **403** to, which is indistinguishable from an expired session and sends
    /// an operator to re-login instead of to the descriptor row.
    CredentialMismatch {
        /// Whether the feed's scheme names two secrets.
        names_two: bool,
        /// Whether two were supplied. Never the secrets themselves — the two
        /// booleans are the whole of what a diagnosis needs.
        given_two: bool,
    },
    /// A value resolved into a URL **path segment** cannot sit in one.
    ///
    /// # Refused, never escaped, and the reason is a real symbol
    ///
    /// A feed whose instrument is a path segment
    /// (`/instruments/historical/:instrument_token/:interval`) interpolates a
    /// value this build read out of a vendor master. Kite's own quote examples
    /// address the NIFTY index as `NSE:NIFTY 50` — **a tradingsymbol with a
    /// space in it** — and a space, a `/`, a `?` or a `#` each change which
    /// resource the URL names rather than merely looking untidy. A `/` is the
    /// worst of them: it silently adds a segment, so
    /// `/instruments/historical/A/B/minute` is a different endpoint that a
    /// vendor may well answer.
    ///
    /// Percent-encoding would be a guess about what the vendor accepts back,
    /// and this repository does not guess at a wire word — `CLAUDE.md` §3
    /// rule 1, and the same argument [`Self::RungNotSpellable`] makes about an
    /// unrecorded rung. So the request is not sent, the offending value is
    /// named, and the descriptor row or the master column is the thing to fix.
    PathSegmentUnusable {
        /// The vendor's own name for the placeholder, so the row to look at is
        /// named the way the vendor's page names it.
        placeholder: &'static str,
        /// What it resolved to. A vendor id or a wire word, never a credential:
        /// nothing on this path can reach the token.
        value: String,
    },
}

impl core::fmt::Display for FetchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::LengthDisagreement {
                open,
                high,
                low,
                close,
                volume,
                timestamp,
                open_interest,
            } => write!(
                f,
                "the vendor's arrays disagree in length — open {open}, high \
                 {high}, low {low}, close {close}, volume {volume}, timestamp \
                 {timestamp}, open interest {open_interest}. Refused whole \
                 rather than truncated to the shortest: a short window filed \
                 as complete is data loss nothing downstream can detect."
            ),
            Self::TooManyRows { rows, cap } => write!(
                f,
                "the vendor returned {rows} rows; this build accepts at most {cap}"
            ),
            Self::TimestampRefused { row, raw, why } => {
                write!(f, "row {row}: timestamp {raw} refused — {why}")
            }
            Self::PriceRefused { row, field, raw } => write!(
                f,
                "row {row}: {field} {raw} does not land on the paisa grid"
            ),
            Self::VendorRefused { status, ref detail } => {
                write!(f, "the vendor refused with status {status}: {detail}")
            }
            Self::TransportFailed { ref detail } => {
                write!(f, "the vendor was not reached: {detail}")
            }
            Self::RungNotSpellable { rung, field } => write!(
                f,
                "this feed names its bar length in the request field {field:?} \
                 and no word for {rung} has been recorded for it. Nothing was \
                 sent: the request that could be built is a request for a \
                 different bar length, and its answer would be filed under \
                 {rung}. UNVERIFIED — record the vendor's own spelling in that \
                 feed's granularity_tokens row and this works the same day."
            ),
            Self::ListingNotSpellable { listing, field } => write!(
                f,
                "this feed names the instrument's class in the request field \
                 {field:?} and no word for {listing:?} has been recorded for \
                 it. Nothing was sent: a guessed word is answered by the vendor \
                 with something, and that something would be filed as bars. \
                 UNVERIFIED — record the vendor's own word in that feed's \
                 listings row and this works the same day."
            ),
            Self::CredentialMismatch {
                names_two,
                given_two,
            } => write!(
                f,
                "this feed's auth scheme names {} secret(s) and {} supplied. \
                 Nothing was sent and no client was built: a two-secret header \
                 missing half of itself is answered 403, which reads exactly \
                 like an expired session and sends an operator to re-login \
                 instead of to the descriptor row. Check the feed's \
                 Auth::key_field against the fields its vendor is configured \
                 with.",
                if names_two { "two" } else { "one" },
                if given_two { "two were" } else { "one was" }
            ),
            Self::PathSegmentUnusable {
                placeholder,
                ref value,
            } => write!(
                f,
                "this feed carries {placeholder:?} as a URL path segment and it \
                 resolved to {value:?}, which cannot be one. Nothing was sent: \
                 a space, a slash, a question mark or a hash changes which \
                 resource the URL names — a slash silently adds a segment, and \
                 the vendor may answer that different endpoint. It is refused \
                 rather than percent-encoded because what the vendor accepts \
                 back is a vendor fact and there is no source for it here."
            ),
        }
    }
}

impl core::error::Error for FetchError {}

impl RawWindow {
    /// Decodes seven parallel arrays into rows, or refuses the whole window.
    ///
    /// # Errors
    ///
    /// [`FetchError::LengthDisagreement`] when the arrays do not agree, and
    /// [`FetchError::TooManyRows`] past [`MAX_ROWS`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::fetch::{ParallelArrays, RawWindow, FetchError};
    /// // 3 opens, 2 volumes — `zip` would have produced two valid-looking bars.
    /// let short = ParallelArrays {
    ///     open: vec![1, 2, 3],
    ///     high: vec![1, 2, 3],
    ///     low: vec![1, 2, 3],
    ///     close: vec![1, 2, 3],
    ///     volume: vec![1, 2],
    ///     timestamp: vec![0, 60, 120],
    ///     open_interest: Vec::new(),
    /// };
    /// assert!(matches!(
    ///     RawWindow::decode(&short),
    ///     Err(FetchError::LengthDisagreement { volume: 2, open: 3, .. })
    /// ));
    /// ```
    pub fn decode(a: &ParallelArrays) -> Result<Self, FetchError> {
        // EVERY LENGTH, COMPARED BEFORE AN ITERATOR EXISTS. Open interest is
        // optional — a vendor that never sends it sends an empty vector — so it
        // agrees when it is either empty or the same length as the rest.
        let n = a.open.len();
        let oi = a.open_interest.len();
        let agree = a.high.len() == n
            && a.low.len() == n
            && a.close.len() == n
            && a.volume.len() == n
            && a.timestamp.len() == n
            && (oi == 0 || oi == n);
        if !agree {
            return Err(FetchError::LengthDisagreement {
                open: n,
                high: a.high.len(),
                low: a.low.len(),
                close: a.close.len(),
                volume: a.volume.len(),
                timestamp: a.timestamp.len(),
                open_interest: oi,
            });
        }
        if n > MAX_ROWS {
            return Err(FetchError::TooManyRows {
                rows: n,
                cap: MAX_ROWS,
            });
        }

        // Reserved from a count known before the loop — law 2, so the vector
        // never grows and the append stays worst-case constant.
        let mut rows = Vec::with_capacity(n);
        for i in 0..n {
            // `get` rather than indexing: the agreement above makes every index
            // in range, but an index that is correct *because of a check forty
            // lines up* is how a panic arrives during a later edit.
            let (Some(&open), Some(&high), Some(&low), Some(&close)) =
                (a.open.get(i), a.high.get(i), a.low.get(i), a.close.get(i))
            else {
                break;
            };
            let (Some(&volume), Some(&timestamp)) = (a.volume.get(i), a.timestamp.get(i)) else {
                break;
            };
            rows.push(RawRow {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                open_interest: a.open_interest.get(i).copied(),
            });
        }
        Ok(Self { rows })
    }
}

/// What one window of bars was asked for.
///
/// Not `Copy`: it now carries the vendor's own instrument id, which is a
/// `String` because the two brokers spell it differently and neither spelling
/// is known at compile time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarRequest {
    /// The vendor's own id for the instrument being asked for.
    ///
    /// Empty on the local-archive path, which addresses a FILE rather than an
    /// instrument. On the HTTP path this is what
    /// [`crate::vendor::ParamValue::InstrumentId`] resolves to — `securityId`
    /// at Dhan, `groww_symbol` at Groww — and its absence is precisely what
    /// `DH-905 securityId is required` reports.
    pub instrument_id: String,
    /// Which class of listing this instrument is.
    ///
    /// Carried beside the id because both brokers require the class on the
    /// wire as well as the id, and the two are read from the SAME master row —
    /// separating them is how 750 equities came to be asked for inside an
    /// index segment. [`crate::vendor::ParamValue::Segment`] and
    /// [`crate::vendor::ParamValue::Kind`] resolve it through the feed's own
    /// [`crate::vendor::HttpSpec::listing_words`].
    ///
    /// Defaults to [`crate::vendor::Listing::Equity`] nowhere: there is no
    /// `Default` impl, so every construction site states it and a new caller
    /// cannot inherit a silent wrong answer.
    pub listing: crate::vendor::Listing,
    /// The inclusive window the operator asked for.
    pub window: Window,
    /// Which rung of the ladder these bars are.
    ///
    /// An argument rather than an assumption, and a [`Granularity`] rather than
    /// the [`Cadence`] it used to be. The two were being stated independently —
    /// the rung decided the store directory and the cadence decided the session
    /// filter — and they disagree silently in the direction that loses
    /// everything: a daily bar is stamped at midnight, so a daily pull left at
    /// `Cadence::Minute` counts every bar `BeforeSessionOpen` and reports a
    /// clean census of nothing. One field, and
    /// [`Granularity::cadence`](crate::vendor::Granularity::cadence) derives the
    /// other.
    pub granularity: Granularity,
}

/// Where rows come from. The only thing a transport must satisfy.
pub trait BarSource {
    /// Fetches one window.
    ///
    /// # Errors
    ///
    /// Whatever the transport refuses, as a named [`FetchError`].
    fn window(&self, request: &BarRequest) -> Result<RawWindow, FetchError>;
}

/// A source that answers from memory. What every test in this crate uses.
///
/// Exists so that no test needs a socket. A test that needed one would be
/// testing the network rather than this code, and would fail on a train.
#[derive(Debug, Clone)]
pub struct FakeSource {
    /// What to answer with.
    pub answer: Result<RawWindow, FetchError>,
}

impl Default for FakeSource {
    /// An empty window. A fake that refuses by default would make every
    /// test that forgot to configure it pass for the wrong reason.
    fn default() -> Self {
        Self {
            answer: Ok(RawWindow::default()),
        }
    }
}

impl FakeSource {
    /// A source that returns these rows.
    #[must_use]
    pub fn returning(rows: Vec<RawRow>) -> Self {
        Self {
            answer: Ok(RawWindow { rows }),
        }
    }

    /// A source that refuses.
    #[must_use]
    pub const fn refusing(why: FetchError) -> Self {
        Self { answer: Err(why) }
    }
}

impl BarSource for FakeSource {
    fn window(&self, _request: &BarRequest) -> Result<RawWindow, FetchError> {
        self.answer.clone()
    }
}

/// What one window produced: the bars that survived, and why the rest did not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Landed {
    /// Bars inside the window and the session, ready for the store.
    pub bars: Vec<Bar>,
    /// Every discarded row, by reason. The total equals rows in minus bars out.
    pub census: DropCensus,
}

/// Converts a vendor price to paisa.
///
/// A vendor quoting rupees is multiplied by 100; one already quoting paisa is
/// taken as is. **There is no rounding here and that is deliberate** —
/// `CLAUDE.md` §7 puts the single snap on the tick grid at the write boundary,
/// and a second rounding site is a second answer.
const fn to_paisa(raw: i64, scale: PriceScale) -> Option<i64> {
    match scale {
        PriceScale::Paisa => Some(raw),
        PriceScale::Rupees => raw.checked_mul(100),
    }
}

/// Turns a raw window into bars, filtering by the operator's window and the
/// session, and counting every drop by reason.
///
/// # Errors
///
/// [`FetchError::TimestampRefused`] for an instant this build cannot hold, and
/// [`FetchError::PriceRefused`] for a price that overflows on conversion. Both
/// refuse the **whole window**: a window missing an arbitrary subset of its
/// rows is not a shorter window, it is a wrong one.
///
/// # Cost
///
/// One pass. Per row: a fixed number of integer operations, one [`IstMoment`]
/// conversion and one census increment. The output vector is reserved from the
/// row count before the loop.
pub fn land(
    raw: &RawWindow,
    request: &BarRequest,
    encoding: TimestampEncoding,
    scale: PriceScale,
) -> Result<Landed, FetchError> {
    let mut bars = Vec::with_capacity(raw.rows.len());
    let mut census = DropCensus::default();

    for (i, row) in raw.rows.iter().enumerate() {
        // W1 LIVES HERE. The encoding is dispatched, never assumed. A vendor
        // stamping IST wall-clock seconds into a field read as UTC epoch
        // produced 45 bars at wrong timestamps that passed every check and
        // were written to disk — the only fault in this pipeline that STORED a
        // wrong answer instead of refusing.
        let epoch_utc = match encoding {
            // ALREADY UTC — AND THE THIRD SPELLING IS HERE FOR A REASON, NOT
            // FOR TIDINESS.
            //
            // `IsoDateTimeOffset` carries its own zone (`…T09:15:00+0530`) and
            // `crate::http::one_stamp` has already applied it, because that is
            // the only place the offset is visible. Subtracting
            // `IST_OFFSET_SECS` here as well — which is what the text arm below
            // does, correctly, for a value that states no zone — would put
            // every bar 5h30m early. 03:45 on a trading day passes every
            // validation below this line and writes to disk looking exactly
            // like a real bar: the **W1 fault** this module's header describes,
            // the one class of timestamp error that stores instead of refusing.
            //
            // It sits WITH the epoch arms rather than below them because "the
            // value already means UTC" is precisely what all three say, and one
            // arm is one place to be wrong instead of two. D-0135.
            TimestampEncoding::EpochSecondsUtc | TimestampEncoding::IsoDateTimeOffset => {
                row.timestamp
            }
            TimestampEncoding::EpochMillisUtc => row.timestamp.div_euclid(1_000),
            // The vendor's value is already IST-based, so the offset this
            // build would add has already been added by the vendor. Subtract it
            // back out before the shared conversion, rather than teaching that
            // conversion a second mode.
            // BOTH TEXT SPELLINGS, ONE BEHAVIOUR. The decoder has already
            // turned `2026-08-04 09:15:00` and `2026-08-04T09:15:00` into
            // IST-based seconds — the separator differs on the wire and the
            // meaning does not — so the offset the vendor already added comes
            // back out here. Joined rather than written twice, because two arms
            // doing the same thing is two places for one of them to drift.
            TimestampEncoding::IstDateTimeText | TimestampEncoding::IsoDateTimeText => {
                row.timestamp - crate::session::IST_OFFSET_SECS
            }
        };

        // `verdict` does the conversion itself and refuses a timestamp that is
        // not one. A drop is a bar this engine DECLINED; a timestamp it could
        // not read is the vendor or the decoder being wrong, and those are
        // different answers.
        let verdict = request
            .window
            .verdict(epoch_utc, request.granularity.cadence())
            .map_err(|why| FetchError::TimestampRefused {
                row: i,
                raw: row.timestamp,
                why,
            })?;
        if let Some(reason) = verdict {
            census.count(reason);
            continue;
        }

        let price = |raw: i64, field: &'static str| {
            to_paisa(raw, scale).ok_or(FetchError::PriceRefused { row: i, field, raw })
        };
        let bar = Bar {
            ts_micros: epoch_utc
                .checked_mul(1_000_000)
                .ok_or(FetchError::TimestampRefused {
                    row: i,
                    raw: row.timestamp,
                    why: SessionError::TimestampOutOfRange { secs: epoch_utc },
                })?,
            open: price(row.open, "open")?,
            high: price(row.high, "high")?,
            low: price(row.low, "low")?,
            close: price(row.close, "close")?,
            volume: row.volume,
            // ABSENT IS NOT ZERO. `i64::MIN` is the null sentinel; a vendor
            // sending a literal 0 means zero open interest, which is a
            // measurement, and it must survive as one.
            open_interest: row.open_interest.unwrap_or(i64::MIN),
        };
        bars.push(bar);
    }

    // THE ROW LEDGER FOR THIS WINDOW, AS AN AGGREGATE.
    //
    // Per row would be millions on a one-minute backfill and would roll the
    // run out of a 64 MiB window; the loop above therefore logs NOTHING and
    // pays no call per row. `DropCensus` counted every rejection in a plain
    // integer, and this is that count, once, where the window closes.
    //
    // The four reasons are named separately rather than summed, because
    // "outside the window" and "outside the session" send an operator to two
    // different places — the caller's chunking and the vendor's clock.
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::debug("pull.land", "window decoded")
            .with("rows_in", telemetry::Value::Uint(raw.rows.len() as u64))
            .with("bars_out", telemetry::Value::Uint(bars.len() as u64))
            .with(
                "before_window",
                telemetry::Value::Uint(u64::from(
                    census.of(crate::session::DropReason::BeforeWindow),
                )),
            )
            .with(
                "after_window",
                telemetry::Value::Uint(u64::from(
                    census.of(crate::session::DropReason::AfterWindow),
                )),
            )
            .with(
                "before_open",
                telemetry::Value::Uint(u64::from(
                    census.of(crate::session::DropReason::BeforeSessionOpen),
                )),
            )
            .with(
                "at_or_after_close",
                telemetry::Value::Uint(u64::from(
                    census.of(crate::session::DropReason::AtOrAfterSessionClose),
                )),
            ),
    );
    Ok(Landed { bars, census })
}

/// Fetches one window and lands it, in one call.
///
/// # Errors
///
/// Whatever the source or [`land`] refuses.
pub fn fetch_and_land<S: BarSource>(
    source: &S,
    request: &BarRequest,
    encoding: TimestampEncoding,
    scale: PriceScale,
) -> Result<Landed, FetchError> {
    let raw = source.window(request)?;
    land(&raw, request, encoding, scale)
}

/// The wire value for a window's end, honouring the vendor's inclusivity.
///
/// **One conversion site.** Dhan's `toDate` is exclusive, so the wire value is
/// the day *after* the operator's last day; a vendor whose end is inclusive
/// takes it unchanged. Two sites would be two answers, and the off-by-one would
/// return the first time someone edited one of them.
///
/// # Errors
///
/// [`SessionError::NoNextDay`] past 9999-12-31, for an exclusive vendor.
pub fn wire_end(last: Day, end: crate::vendor::RangeEnd) -> Result<Day, SessionError> {
    match end {
        crate::vendor::RangeEnd::Inclusive => Ok(last),
        crate::vendor::RangeEnd::Exclusive => last.succ(),
    }
}
