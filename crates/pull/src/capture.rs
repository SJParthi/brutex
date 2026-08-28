//! Real vendor answers, written to disk before anything parses them.
//!
//! **Two budgets, two questions.** [`record`] keeps the first few answers a
//! feed gives, so a fixture can stop being hand-written. [`record_unreadable`]
//! keeps the answers that DEFEATED the decoder, which is the question an
//! operator asks after a backfill dies — and until it existed, the answer was
//! *"the body is gone"*. Both are bounded by construction and neither can fail
//! a pull; see [`record_unreadable`] for why the second one needed its own
//! ceiling rather than sharing the first's.
//!
//! # Why this exists
//!
//! Every test covering the expired-derivative path — 129 of them across
//! `fno`, `chain`, `fnowork`, `rolling`, `pricing` and `tenor` — is built on a
//! **hand-written** fixture. Each one encodes what the vendor's *documentation*
//! says a response looks like. Searched on 2026-08-22: the only fixture in this
//! repository marked as a real capture is `TrueData`'s archive layout,
//! *"verbatim from the operator's 2022 archive"*, and that is a CSV on disk
//! rather than an API answer.
//!
//! So the suite proves the code is self-consistent **with the documents**, and
//! nothing has ever proved it consistent with the vendor. Those are different
//! claims and this module exists because only one of them was ever tested.
//!
//! The expired-derivative route cannot be driven offline the way the spot route
//! can: [`crate::fno`]'s discovery is a vendor call by construction — the two
//! brokers publish different endpoints and different JSON field names — and no
//! request type on that path carries a folder. So the first real answer can
//! only arrive during a real pull, and if it is parsed and dropped, the next
//! test still has nothing but the documentation to stand on.
//!
//! **This records it before it is parsed**, so a request the operator spends
//! once becomes a fixture for ever.
//!
//! # What it never records
//!
//! **No header, ever.** `CLAUDE.md` §8 puts the credential in a header —
//! `HttpSource::header_value` builds `Raw`, `Bearer` or `PrefixedPair` and
//! hands it to the request builder — and there is no secret-valued query
//! parameter in any descriptor. Capturing the URL is therefore safe and
//! capturing headers never is, so this module is given the URL and the body and
//! is not given the request.
//!
//! # Cost
//!
//! **One atomic per answer, and I/O at most [`PER_SLOT`] times per slot.**
//! The slot is `feed as usize * 2 + method as usize` — an array index, no hash
//! and no search, because [`Feed`] carries explicit discriminants `0..4` and
//! `FEED_COUNT` pins the width. Once a slot is full, [`record`] is one
//! `fetch_update` that fails and returns; it never opens a file again and never
//! allocates.
//!
//! The total is bounded by construction at `FEED_COUNT * 2 * PER_SLOT` files
//! for the life of the process, so a twelve-hour backfill writes exactly as
//! much as a single request does once the budget is spent. That bound is the
//! whole design: a capture that grew with the pull would be a disk-filling
//! fallback, which is the shape `CLAUDE.md` §4 bans.
//!
//! # A failed capture never fails a pull
//!
//! It is a diagnostic beside the data path, not on it. A write that fails is
//! **counted and emitted**, never swallowed and never propagated: refusing a
//! vendor answer that arrived intact because a debugging aid could not be
//! written would be the fallback hiding a failure, pointing the other way.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::vendor::{FEED_COUNT, Feed};

/// How many answers are kept per (feed, method).
///
/// **Four, because one is not enough and unbounded is a disk.** Groww's F&O
/// path alone issues three distinct GETs — expiries, contracts, then the bars
/// for a named contract — so a budget of one would capture the expiry list and
/// never the shape that actually carries prices. Four leaves room for the
/// fourth call a descriptor may add without anyone revisiting this number.
pub const PER_SLOT: u32 = 4;

/// One slot per feed per method.
const SLOTS: usize = FEED_COUNT * 2;

/// How many answers each slot has already kept.
static TAKEN: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];

/// How many captures could not be written, this process.
///
/// Read by [`refused`]. A capture is a diagnostic and its failure must not
/// reach the caller, but an unrecorded failure would leave an operator
/// believing a fixture exists when none does — so it is counted here and
/// emitted at the site.
static REFUSED: AtomicU64 = AtomicU64::new(0);

/// Which verb carried the answer.
///
/// Two, and they are the two the F&O path uses: Dhan's
/// `POST /v2/charts/rollingoption` and Groww's discovery GETs. The enum exists
/// so the slot index is arithmetic rather than a string comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// A `GET`, as the discovery calls use.
    Get = 0,
    /// A `POST`, as the rolling-option call uses.
    Post = 1,
}

impl Method {
    /// The word for the file's own header line.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// How many captures this process could not write.
#[must_use]
pub fn refused() -> u64 {
    REFUSED.load(Ordering::Relaxed)
}

/// How many answers this slot has kept so far.
///
/// For a report and for a test. Never used to decide whether to capture — that
/// decision is the `fetch_update` inside [`record`], because a load followed by
/// a store is two operations and two threads can pass the same load.
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "the index is arithmetic on two #[repr] discriminants whose widths \
              are pinned below by const assertion, so it is in range at compile \
              time; `.get()` would need either an unwrap, which is banned, or a \
              default for an unreachable case, which is the §4 fallback"
)]
pub fn kept(feed: Feed, method: Method) -> u32 {
    TAKEN[slot(feed, method)].load(Ordering::Relaxed)
}

/// The array index for one (feed, method).
///
/// `Feed` carries explicit discriminants `0..4` and `FEED_COUNT` pins the
/// table's width, so this is a multiply and an add against a fixed-size array.
const fn slot(feed: Feed, method: Method) -> usize {
    (feed as usize) * 2 + (method as usize)
}

// THE BOUND, PROVED AT COMPILE TIME RATHER THAN ASSERTED IN PROSE. The widest
// index this can produce is the last feed against the last method; if a sixth
// feed or a third method is ever added, this stops compiling rather than
// indexing past the array. That is what earns the `expect` above.
const _: () = assert!(slot(Feed::Zerodha, Method::Post) == SLOTS - 1);
const _: () = assert!(slot(Feed::Dhan, Method::Get) == 0);

/// Keep this answer, if the slot has room.
///
/// Returns the path written, or `None` when the slot is full or the write could
/// not be made. **`None` is not an error the caller should act on**: the answer
/// itself is unaffected either way.
///
/// `body` is written verbatim, byte for byte, ahead of any parse. That is the
/// point — a body this build reshaped before recording it would prove only that
/// the reshaping is self-consistent, which is the circularity this module
/// exists to break.
pub fn record(
    root: &std::path::Path,
    feed: Feed,
    method: Method,
    url: &str,
    body: &str,
) -> Option<std::path::PathBuf> {
    // ONE ATOMIC, AND IT IS THE DECISION. `fetch_update` returns `Err` when the
    // closure says no, so a full slot costs exactly this and never touches the
    // filesystem. A `load` then a `store` would let two threads read the same
    // count and both write, which is how a bounded thing becomes unbounded.
    #[expect(
        clippy::indexing_slicing,
        reason = "same compile-time bound as `kept` -- see the const assertions \
                  beside `slot`"
    )]
    let seq = TAKEN[slot(feed, method)]
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |held| {
            (held < PER_SLOT).then_some(held + 1)
        })
        .ok()?;

    let dir = root.join("captures");
    let path = dir.join(format!("{}-{}-{seq}.txt", feed.wire(), method.word()));

    // THE HEADER IS THREE LINES AND THEN THE BODY UNTOUCHED. A separator that
    // could occur inside a JSON body would make the file ambiguous, so the
    // length is stated and the reader can take the last `bytes` bytes rather
    // than searching for a delimiter.
    let mut text = String::with_capacity(body.len() + 256);
    text.push_str("# brutex vendor capture — the body below is verbatim, pre-parse.\n");
    text.push_str("# No header was recorded: CLAUDE.md §8 puts the credential in one.\n");
    let _ = std::fmt::Write::write_fmt(
        &mut text,
        format_args!(
            "feed: {}\nmethod: {}\nurl: {url}\nbytes: {}\n---\n",
            feed.wire(),
            method.word(),
            body.len()
        ),
    );
    text.push_str(body);

    if std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&path, text.as_bytes()))
        .is_err()
    {
        // COUNTED, NOT SWALLOWED, AND NOT PROPAGATED. See the module note:
        // a vendor answer that arrived intact is not refused because a
        // debugging aid could not be written.
        REFUSED.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    Some(path)
}

/// How many bodies THIS BUILD COULD NOT READ each feed has kept.
///
/// A separate budget from [`TAKEN`], and separate on purpose. That one records
/// the first few answers a feed gives so a fixture can stop being hand-written;
/// this one records the answers that DEFEATED the decoder, which is a different
/// question and the one an operator asks at 2am.
static UNREADABLE: [AtomicU32; FEED_COUNT] = [const { AtomicU32::new(0) }; FEED_COUNT];

/// How many bodies this feed has kept for being unreadable.
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "`Feed` carries explicit discriminants 0..4 and `FEED_COUNT` pins \
              this array's width, exactly as `kept` is bounded; the const \
              assertion below proves the widest index is in range"
)]
pub fn unread_kept(feed: Feed) -> u32 {
    UNREADABLE[feed as usize].load(Ordering::Relaxed)
}

// THE SAME COMPILE-TIME BOUND AS `slot`, on the narrower index. A sixth feed
// stops this compiling rather than indexing past the array.
const _: () = assert!((Feed::Zerodha as usize) == FEED_COUNT - 1);

/// Keep a body **this build could not read**, if the feed's budget has room.
///
/// # Why this is worth its own budget
///
/// The measured case, and it is the reason this function exists. Dhan sent
/// `volume: -125` for `ADANIENT`; the decoder refused it, the refusal cost that
/// instrument every intraday rung it had, and afterwards **nothing could say
/// what `-125` actually was** — a genuine value, a wrapped `int32`, or a column
/// read at the wrong offset. Those three want different responses and the
/// evidence to tell them apart had already been dropped on the floor.
///
/// The audit journal is a fixed-stride binary record: it keeps the URL and a
/// message and never the payload. So a vendor defect was diagnosable only while
/// the process that met it was still running, and only by someone watching.
///
/// This closes that. The body that defeated the decoder is written verbatim,
/// beside the sentence saying what defeated it.
///
/// # Bounded by construction, like everything else here
///
/// [`PER_SLOT`] per feed for the life of the process — `FEED_COUNT * PER_SLOT`
/// files at most, whatever a backfill does. A capture that grew with the
/// failures would be a disk-filling fallback, and a run that fails a million
/// times is exactly the run that must not also fill the disk. The first few are
/// what carries the information: the millionth copy of one vendor defect says
/// nothing the first did not.
///
/// # It never fails a pull, and here that matters more, not less
///
/// The caller is already on a failure path. Turning a diagnostic's failed write
/// into a second, different failure would replace the reason the operator needs
/// with one about the disk — `CLAUDE.md` §4 pointing the other way.
pub fn record_unreadable(
    root: &std::path::Path,
    feed: Feed,
    url: &str,
    body: &str,
    why: &str,
) -> Option<std::path::PathBuf> {
    #[expect(
        clippy::indexing_slicing,
        reason = "same compile-time bound as `unread_kept` -- see the const \
                  assertion above"
    )]
    let seq = UNREADABLE[feed as usize]
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |held| {
            (held < PER_SLOT).then_some(held + 1)
        })
        .ok()?;

    let dir = root.join("captures");
    let path = dir.join(format!("{}-unreadable-{seq}.txt", feed.wire()));

    let mut text = String::with_capacity(body.len() + 512);
    text.push_str("# brutex UNREADABLE vendor body — verbatim, exactly as it arrived.\n");
    text.push_str("# This build could not decode it. The reason is on the `why:` line.\n");
    text.push_str("# No header was recorded: CLAUDE.md §8 puts the credential in one.\n");
    let _ = std::fmt::Write::write_fmt(
        &mut text,
        format_args!(
            "feed: {}\nurl: {url}\nwhy: {why}\nbytes: {}\n---\n",
            feed.wire(),
            body.len()
        ),
    );
    text.push_str(body);

    if std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&path, text.as_bytes()))
        .is_err()
    {
        REFUSED.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    Some(path)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod tests {
    use super::*;

    /// A slot nobody else touches, so the budget assertions are not a race
    /// against whatever another test in this binary captured.
    ///
    /// `TrueData` is an archive feed: it has no HTTP transport, so nothing in
    /// production ever records against its slot and the count starts at zero
    /// for this test alone.
    const MINE: Feed = Feed::TrueData;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("brutex-capture-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    /// **THE BODY IS RECORDED BYTE FOR BYTE, AND THE BUDGET IS A CEILING.**
    ///
    /// Both halves matter and the second is the one that keeps this affordable.
    /// A capture that recorded every answer would grow with the backfill —
    /// the disk-filling fallback `CLAUDE.md` §4 bans — so the fifth answer into
    /// a full slot must write nothing at all.
    #[test]
    fn a_body_is_kept_verbatim_and_the_budget_is_a_hard_ceiling() {
        let root = scratch("verbatim");
        // A body with the two things a naive format would break on: the
        // separator this file uses, and a newline inside a value.
        let body = "{\"candles\":[[1,2]],\"note\":\"---\\nnot a delimiter\"}";

        let first = record(&root, MINE, Method::Get, "https://example.test/a", body)
            .expect("the first capture is written");
        let text = std::fs::read_to_string(&first).expect("read back");

        let (_head, kept_body) = text
            .split_once("\n---\n")
            .expect("the header ends with its own line");
        assert_eq!(
            kept_body, body,
            "the body must survive byte for byte -- a capture this build \
             reshaped would prove only that the reshaping is self-consistent"
        );
        assert!(
            text.contains("url: https://example.test/a"),
            "the URL is recorded, because a body with no endpoint is not a \
             fixture anybody can reuse: {text}"
        );
        assert!(
            !text.to_ascii_lowercase().contains("authorization"),
            "NO HEADER IS EVER RECORDED -- CLAUDE.md §8 puts the credential in \
             one, and this function is never given the request: {text}"
        );

        // FILL THE SLOT, THEN OVERFLOW IT.
        for n in 1..PER_SLOT {
            assert!(
                record(&root, MINE, Method::Get, "https://example.test/b", "{}").is_some(),
                "capture {n} is inside the budget of {PER_SLOT}"
            );
        }
        assert_eq!(kept(MINE, Method::Get), PER_SLOT, "the slot is now full");

        let over = record(&root, MINE, Method::Get, "https://example.test/c", "{}");
        assert!(
            over.is_none(),
            "past the budget nothing is written -- a capture that grew with the \
             pull is the disk-filling fallback §4 bans"
        );
        assert_eq!(
            kept(MINE, Method::Get),
            PER_SLOT,
            "and the counter does not climb past the ceiling, so it cannot wrap"
        );
    }

    /// **A BODY THIS BUILD COULD NOT READ IS KEPT VERBATIM, WITH THE REASON
    /// BESIDE IT — AND THE BUDGET IS A HARD CEILING.**
    ///
    /// The gap this closes was measured, not imagined. Dhan sent
    /// `volume: -125` for `ADANIENT`; the decoder refused it; the refusal cost
    /// that instrument every intraday rung it had; and afterwards **nothing in
    /// the system could say what `-125` actually was**. A genuine value, a
    /// wrapped `int32`, and a column read at the wrong offset are three
    /// different faults wanting three different responses, and the one artifact
    /// that separates them had already been dropped — the audit journal is a
    /// fixed-stride record that keeps a URL and a message and never a payload.
    ///
    /// Three things are asserted because all three are the point: the body is
    /// **byte-for-byte** what arrived (a reshaped one would prove only that the
    /// reshaping is self-consistent), the **reason travels with it** (a body
    /// with no verdict is a puzzle rather than evidence), and the budget
    /// **stops** (a capture that grew with the failures would fill a disk on
    /// exactly the run that fails a million times).
    #[test]
    fn an_unreadable_body_is_kept_with_its_reason_and_the_budget_stops() {
        let root = scratch("unreadable");
        let body = r#"{"open":[1],"volume":[-125],"timestamp":[1751337900]}"#;
        let why = "\"volume\" holds -125, and a volume counts shares traded";

        let path = record_unreadable(&root, Feed::Dhan, "https://example.invalid/v2", body, why)
            .expect("the first one is inside the budget");
        let text = std::fs::read_to_string(&path).expect("it was written");

        assert!(
            text.ends_with(body),
            "the body is verbatim and LAST, so a reader can take the final \
             `bytes` bytes without searching for a delimiter that could occur \
             inside JSON: {text}"
        );
        assert!(
            text.contains(why),
            "and the verdict travels with it — a body with no reason attached \
             is a puzzle, not evidence: {text}"
        );
        assert!(
            text.contains(&format!("bytes: {}", body.len())),
            "the length is stated rather than delimited: {text}"
        );
        assert!(
            text.contains("https://example.invalid/v2"),
            "and the request that produced it: {text}"
        );
        assert!(
            !text.to_ascii_lowercase().contains("authorization")
                && !text.to_ascii_lowercase().contains("access-token"),
            "NEVER a header — CLAUDE.md §8 puts the credential in one: {text}"
        );

        // THE CEILING IS HARD. `PER_SLOT` in total for this feed, whatever the
        // run does afterwards.
        for _ in 0..PER_SLOT {
            drop(record_unreadable(
                &root,
                Feed::Dhan,
                "https://example.invalid/v2",
                body,
                why,
            ));
        }
        assert_eq!(
            unread_kept(Feed::Dhan),
            PER_SLOT,
            "the budget is a ceiling, not a rate"
        );
        assert!(
            record_unreadable(&root, Feed::Dhan, "u", body, why).is_none(),
            "and past it the function writes nothing at all"
        );

        // AND IT IS A SEPARATE BUDGET FROM THE FIXTURE ONE. A run that spent
        // its unreadable budget must still be able to keep a good answer, and
        // the reverse — they answer different questions.
        assert_eq!(
            kept(Feed::Dhan, Method::Post),
            0,
            "spending the unreadable budget did not touch the fixture budget"
        );

        // ANOTHER FEED HAS ITS OWN. One vendor misbehaving must not blind the
        // build to a second one starting to.
        assert_eq!(unread_kept(Feed::Groww), 0, "budgets are per feed");
        assert!(
            record_unreadable(&root, Feed::Groww, "u", body, why).is_some(),
            "and Groww's is still open"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE TWO METHODS DO NOT SHARE A BUDGET.**
    ///
    /// Dhan's rolling-option answer is a POST and Groww's discovery answers are
    /// GETs. One shared slot would let four GETs starve the single POST that
    /// carries the shape nobody has ever seen.
    #[test]
    fn a_full_get_slot_leaves_the_post_slot_untouched() {
        let root = scratch("methods");
        for _ in 0..PER_SLOT {
            drop(record(
                &root,
                Feed::Gdfl,
                Method::Get,
                "https://x.test/g",
                "{}",
            ));
        }
        assert_eq!(kept(Feed::Gdfl, Method::Get), PER_SLOT, "GET is full");
        assert_eq!(
            kept(Feed::Gdfl, Method::Post),
            0,
            "and POST has spent nothing"
        );
        assert!(
            record(&root, Feed::Gdfl, Method::Post, "https://x.test/p", "{}").is_some(),
            "so the POST answer is still captured"
        );
    }

    /// **A WRITE THAT CANNOT LAND IS COUNTED, NEVER SWALLOWED.**
    ///
    /// The capture is a diagnostic beside the data path. Its failure must not
    /// reach the caller — refusing a vendor answer that arrived intact because
    /// a debugging aid failed would be §4 pointing the wrong way — but an
    /// unrecorded failure would leave an operator believing a fixture exists.
    #[test]
    fn a_root_that_cannot_be_written_is_counted_rather_than_hidden() {
        // A FILE WHERE THE DIRECTORY MUST GO. `create_dir_all` cannot succeed
        // against it on any platform, which is the point: this needs a real
        // failure rather than a mocked one.
        let base = scratch("unwritable");
        let blocker = base.join("captures");
        std::fs::write(&blocker, b"not a directory").expect("write the blocker");

        let before = refused();
        let got = record(&base, Feed::Zerodha, Method::Post, "https://x.test/z", "{}");
        assert!(got.is_none(), "nothing was written");
        assert_eq!(
            refused(),
            before + 1,
            "and the failure is COUNTED -- silence here would be the fixture \
             that an operator thinks exists and does not"
        );
    }
}
