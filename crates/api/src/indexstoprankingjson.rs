//! Cumulative per-timeframe comparisons over an exact acknowledged search
//! prefix. Full cold authentication and global ordering precede filtering and
//! paging. A pending next batch never becomes an apparent completed result.
use axum::http::{StatusCode, Uri};
use cli::index_stop_search::reader::{Budget, Reader};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const MODEL: &str = "index-stop-ranking";
type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    All,
    Qualified,
}
impl Filter {
    const fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Qualified => "qualified",
        }
    }
}
struct Asked {
    identity: [u8; 32],
    rung: usize,
    pin: Option<(u64, [u8; 32])>,
    filter: Filter,
    offset: usize,
    limit: usize,
}
impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or("ranking query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity"
                        | "timeframe"
                        | "sequence"
                        | "completion"
                        | "filter"
                        | "offset"
                        | "limit"
                )
                || !seen.insert(key)
            {
                return Err("empty, repeated or unknown ranking query field".into());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("exact search identity required")?;
        let timeframe = crate::server::param(query, "timeframe");
        let rung = cli::EVERY_RUNG
            .iter()
            .position(|label| *label == timeframe)
            .ok_or("ranking requires one exact intraday timeframe")?;
        let sequence = crate::candidatejson::integer(query, "sequence")?;
        let completion = crate::candidatejson::hex_param(query, "completion")?;
        let pin =
            match (sequence, completion) {
                (None, None) => None,
                (Some(sequence), Some(completion)) if sequence > 0 => Some((sequence, completion)),
                _ => return Err(
                    "ranking checkpoint requires a positive sequence and completion pin together"
                        .into(),
                ),
            };
        let filter = match crate::server::param(query, "filter").as_str() {
            "" | "qualified" => Filter::Qualified,
            "all" => Filter::All,
            _ => return Err("ranking filter must be qualified or all".into()),
        };
        let offset = usize::try_from(crate::candidatejson::integer(query, "offset")?.unwrap_or(0))
            .map_err(display)?;
        let limit = usize::try_from(crate::candidatejson::integer(query, "limit")?.unwrap_or(16))
            .map_err(display)?;
        if identity == [0; 32]
            || completion == Some([0; 32])
            || limit == 0
            || limit > 256
            || offset > 0 && pin.is_none()
        {
            return Err(
                "ranking pages require 1–256 rows and exact checkpoint pins for continuation"
                    .into(),
            );
        }
        Ok(Self {
            identity,
            rung,
            pin,
            filter,
            offset,
            limit,
        })
    }
}
fn response(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}
fn refused(status: StatusCode, why: &str) -> Response {
    response(
        status,
        &json!({"schema_version":1,"status":"refused","model":MODEL,"rows":[],"refusal":why}),
    )
}
/// Read one bounded cumulative page. This endpoint has no write or run action.
pub async fn index_stop_ranking_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(value) => value,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| render(&root, &asked)) {
        Ok(body) => {
            let reply = response(StatusCode::OK, &body);
            if reply.2.len() > crate::detail::MAX_RESPONSE_BYTES {
                refused(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "cumulative response exceeds byte admission; no prefix returned",
                )
            } else {
                reply
            }
        }
        Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    })
    .await
    {
        Ok(reply) => reply,
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "saved-result read capacity is full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rank {
    score: i64,
    family: usize,
    batch: u64,
    setting: usize,
    family_position: usize,
    qualified: bool,
}
struct Cached {
    root: PathBuf,
    identity: [u8; 32],
    rung: usize,
    budget: Budget,
    reader: Reader,
    ranked: Vec<Rank>,
    qualified: Vec<usize>,
    summary: Value,
}
fn render(root: &Path, asked: &Asked) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let observation = crate::detail::BooleanObservationBudget::load()?;
    let work = crate::booleansearchjson::budget::ReplayBudget::load()?.nodes();
    let budget = Budget {
        bytes: observation.bytes(),
        records: observation.bytes() / 64,
        grammar_nodes: work,
        bootstrap_work: work,
        split_work: work,
    };
    // Taken out for this request and put back afterwards; never held
    // across the cold open (locks-2, D-1912).
    let mut held = crate::detail::Checkout::take(CACHE.get_or_init(|| Mutex::new(None)));
    // NOT CONSTANT-TIME WHEN UNPINNED, WARM OR COLD. The pin must be known
    // before the cache can be compared, so every unpinned request walks the
    // search's checkpoint reservation directories and reads the newest
    // acknowledged payload, even when the cached reader already holds that
    // checkpoint. A pinned request skips the walk. `docs/06-limits.md`, D-0904.
    let pin = match asked.pin {
        Some(value) => value,
        None => Reader::latest_checkpoint(root, asked.identity, budget.bytes)?,
    };
    if !held.as_ref().is_some_and(|old| {
        old.root == root
            && old.identity == asked.identity
            && old.rung == asked.rung
            && old.budget == budget
            && old.reader.checkpoint() == pin
    }) {
        // The one retained native cache is replaced within its declared bound;
        // clients retain their last authenticated rendered scope on failure.
        *held = None;
        let reader=Reader::open(root,asked.identity,asked.rung,Some(pin),budget).map_err(|why|observation.context(&format!("Cumulative saved comparison: {why}. Numerical admission counts the sum of complete required child bootstrap/split work from exact saved geometry; no partial leaderboard was substituted.")))?;
        let (ranked, qualified, summary) = rank(&reader, budget.bytes)?;
        *held = Some(Cached {
            root: root.to_owned(),
            identity: asked.identity,
            rung: asked.rung,
            budget,
            reader,
            ranked,
            qualified,
            summary,
        });
    }
    let cached = held.as_ref().ok_or("cumulative cache admission missing")?;
    let result = cached.reader.with_current(|| project(cached, asked));
    if result.is_err() {
        *held = None;
    }
    result
}
fn sort_ranks(rows: &mut [Rank]) {
    rows.sort_unstable_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.batch.cmp(&b.batch))
            .then_with(|| a.setting.cmp(&b.setting))
    });
}
fn rank(reader: &Reader, budget: u64) -> Result<(Vec<Rank>, Vec<usize>, Value), String> {
    reader.with_current(|| rank_current(reader, budget))
}
// The caller holds every saved family and relation through the global sort.
fn rank_current(reader: &Reader, budget: u64) -> Result<(Vec<Rank>, Vec<usize>, Value), String> {
    let count = reader
        .families()
        .iter()
        .try_fold(0_usize, |count, family| {
            count
                .checked_add(family.reader.rows().len())
                .ok_or("cumulative rank count overflow")
        })?;
    let extra = u64::try_from(count)
        .map_err(display)?
        .checked_mul(
            u64::try_from(std::mem::size_of::<Rank>() + 2 * std::mem::size_of::<usize>())
                .map_err(display)?,
        )
        .ok_or("cumulative rank byte count overflow")?;
    if reader
        .admitted_bytes()
        .checked_add(reader.projection_scratch_bytes())
        .and_then(|bytes| bytes.checked_add(extra))
        .is_none_or(|bytes| bytes > budget)
    {
        return Err("cumulative evidence and complete rank indexes exceed read admission".into());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(display)?;
    let mut counts = [0_u64; 5];
    for (family_index, family) in reader.families().iter().enumerate() {
        let first = rows.len();
        for (index, row) in family.reader.rows().iter().enumerate() {
            let later = family
                .reader
                .later()
                .records()
                .get(index)
                .ok_or("cumulative later coordinate absent")?;
            let daily = family
                .reader
                .consistencies()
                .get(index)
                .ok_or("cumulative daily coordinate absent")?;
            let state = row.projection.verdict().status();
            let qualified = daily.combined_qualifies(state);
            let slot = match state {
                cli::boolean_evidence::AdmissionStatusV1::Admitted => 0,
                cli::boolean_evidence::AdmissionStatusV1::Rejected => 1,
                cli::boolean_evidence::AdmissionStatusV1::Refused => 2,
                cli::boolean_evidence::AdmissionStatusV1::Unmeasured => 3,
            };
            let count = counts
                .get_mut(slot)
                .ok_or("cumulative verdict slot unavailable")?;
            *count = count
                .checked_add(1)
                .ok_or("cumulative verdict count overflow")?;
            counts[4] = counts[4]
                .checked_add(u64::from(qualified))
                .ok_or("cumulative combined count overflow")?;
            rows.push(Rank {
                score: later.metrics().pessimistic_paisa,
                family: family_index,
                batch: family.batch,
                setting: index,
                family_position: 0,
                qualified,
            });
        }
        // Each exact detail link retains its rank in its complete original
        // family; it does not reuse the cumulative rank as a row address.
        let family_rows = rows
            .get_mut(first..)
            .ok_or("cumulative family rank extent missing")?;
        sort_ranks(family_rows);
        for (rank, row) in family_rows.iter_mut().enumerate() {
            row.family_position = rank + 1;
        }
    }
    sort_ranks(&mut rows);
    let mut qualified = Vec::new();
    qualified
        .try_reserve_exact(usize::try_from(counts[4]).map_err(display)?)
        .map_err(display)?;
    for (index, row) in rows.iter().enumerate() {
        if row.qualified {
            qualified.push(index);
        }
    }
    Ok((
        rows,
        qualified,
        json!({"institutional_admitted":counts[0].to_string(),"rejected":counts[1].to_string(),"refused":counts[2].to_string(),"unmeasured":counts[3].to_string(),"combined_qualified":counts[4].to_string()}),
    ))
}
fn project(cached: &Cached, asked: &Asked) -> Result<Value, String> {
    let reader = &cached.reader;
    let total = match asked.filter {
        Filter::All => cached.ranked.len(),
        Filter::Qualified => cached.qualified.len(),
    };
    if asked.offset > total {
        return Err("cumulative ranking page lies outside its pinned filtered extent".into());
    }
    let end = asked.offset.saturating_add(asked.limit).min(total);
    let mut rows = Vec::new();
    rows.try_reserve_exact(end - asked.offset)
        .map_err(display)?;
    for index in asked.offset..end {
        let global = match asked.filter {
            Filter::All => index,
            Filter::Qualified => *cached
                .qualified
                .get(index)
                .ok_or("qualified rank index missing")?,
        };
        let ranked = cached
            .ranked
            .get(global)
            .ok_or("cumulative rank index missing")?;
        let family = reader
            .families()
            .get(ranked.family)
            .ok_or("cumulative family missing")?;
        // The complete cumulative scope is already leased, including families
        // outside this page that determine the rank and filtered denominator.
        rows.push(row(&family.reader, *ranked, index + 1, global + 1)?);
    }
    let (sequence, pin) = reader.checkpoint();
    let (grammar, bootstrap, splits) = reader.charged_work();
    let declared = reader.declaration();
    let timeframe = cli::EVERY_RUNG
        .get(cached.rung)
        .ok_or("cumulative timeframe unavailable")?;
    Ok(
        json!({"schema_version":1,"status":"saved","model":MODEL,"identity":hex(cached.identity),"timeframe":timeframe,"rung":cached.rung.to_string(),
        "checkpoint":{"sequence":sequence.to_string(),"completion":hex(pin)},
        "scope":{"completed_batches":reader.completed_batches().to_string(),"completed_programs":reader.completed_programs().to_string(),"completed_work":reader.completed_work().to_string(),
        "included_families":reader.families().len().to_string(),"first_batch":reader.families().first().map(|family|family.batch.to_string()),"last_batch":reader.families().last().map(|family|family.batch.to_string()),
        "pending_next_batch":reader.pending(),"exhausted":reader.exhausted(),"interrupted_checkpoints":reader.interrupted().to_string(),"writer_observed_at_admission":reader.writer_observed(),
        "instrument":declared.index(),"feed":declared.feed(),"training_first_day":declared.training_days().0.to_string(),"training_last_day":declared.training_days().1.to_string(),"later_first_day":declared.later_days().0.to_string(),"later_last_day":declared.later_days().1.to_string()},
        "ranking":"later_pessimistic_descending_then_batch_then_setting","filter":asked.filter.label(),"total":total.to_string(),"all_total":cached.ranked.len().to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"summary":cached.summary,
        "read_work":{"retained_serialized_bytes":reader.admitted_bytes().to_string(),"charged_grammar_nodes":grammar.to_string(),"charged_required_bootstrap_work":bootstrap.to_string(),"charged_required_split_work":splits.to_string(),
        "cold":"complete_acknowledged_prefix_then_global_sort","warm":"ancestor_generation_checks_then_bounded_indexed_page"},"rows":rows,"refusal":null}),
    )
}
fn row(
    reader: &cli::index_stop_qualification::Reader,
    ranked: Rank,
    rank: usize,
    global_rank: usize,
) -> Result<Value, String> {
    let index = ranked.setting;
    let mut value = crate::indexstopqualificationjson::setting(reader, index, rank)?;
    let fields = value
        .as_object_mut()
        .ok_or("native setting serializer returned a nonobject")?;
    let extra = json!({"global_rank":global_rank.to_string(),"batch":ranked.batch.to_string(),"qualification_rank":ranked.family_position.to_string(),
        "qualification":{"identity":hex(reader.identity()),"completion":hex(reader.completion_digest())},"qualification_total":reader.rows().len().to_string(),
        "original_source":{"identity":hex(reader.training().identity()),"completion":hex(reader.training().completion_digest())},"later_source":{"identity":hex(reader.later().identity()),"completion":hex(reader.later().completion_digest())}});
    fields.extend(
        extra
            .as_object()
            .ok_or("native ranking links returned a nonobject")?
            .clone(),
    );
    Ok(value)
}
fn hex(value: [u8; 32]) -> String {
    crate::server::hex32(value)
}
fn display(value: impl std::fmt::Display) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::indexing_slicing,
        reason = "small fixed ranking fixtures assert known sorted positions"
    )]
    use super::*;
    const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    #[test]
    fn strict_scope_and_paired_checkpoint_are_required_before_any_store_read() {
        let base = format!("identity={ID}&timeframe=1min");
        assert!(Asked::parse(&base).is_ok());
        for extra in [
            "&offset=1",
            "&sequence=1",
            "&completion=bad",
            "&filter=winner",
            "&timeframe=2min",
            "&limit=0",
            "&limit=257",
            "&limit=01",
            "&offset=-1",
            "&unknown=1",
        ] {
            assert!(Asked::parse(&format!("{base}{extra}")).is_err(), "{extra}");
        }
        assert!(
            Asked::parse(&format!(
                "{base}&sequence=1&completion={ID}&offset=256&limit=256&filter=all"
            ))
            .is_ok()
        );
        assert!(Asked::parse(&format!("identity={ID}&timeframe=1day")).is_err());
        assert!(Asked::parse(&format!("{base}&sequence=0&completion={ID}")).is_err());
    }
    #[test]
    fn cross_batch_ranking_precedes_pages_and_qualified_filter_retains_total_order() {
        let mut rows = vec![
            Rank {
                score: 5,
                family: 0,
                batch: 0,
                setting: 0,
                family_position: 1,
                qualified: true,
            },
            Rank {
                score: 500,
                family: 1,
                batch: 1,
                setting: 0,
                family_position: 1,
                qualified: false,
            },
            Rank {
                score: 5,
                family: 1,
                batch: 1,
                setting: 1,
                family_position: 2,
                qualified: true,
            },
            Rank {
                score: i64::MIN,
                family: 0,
                batch: 0,
                setting: 1,
                family_position: 2,
                qualified: false,
            },
            Rank {
                score: i64::MAX,
                family: 2,
                batch: 2,
                setting: 0,
                family_position: 1,
                qualified: true,
            },
        ];
        sort_ranks(&mut rows);
        assert_eq!(
            rows.iter()
                .map(|row| (row.batch, row.setting))
                .collect::<Vec<_>>(),
            vec![(2, 0), (1, 0), (0, 0), (1, 1), (0, 1)]
        );
        let filtered = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.qualified)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(filtered, vec![0, 2, 3]);
        assert_eq!(rows[filtered[1]].batch, 0);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[1].family_position, 1);
    }

    /// A WARM UNPINNED RANKING REQUEST STILL WALKS THE CHECKPOINTS. D-0904.
    ///
    /// No fixture in this crate builds a saved index-stop search, so this
    /// reads the path off its source: the unpinned arm of `render` precedes the
    /// cache comparison, `latest_checkpoint` opens a snapshot and reads one
    /// payload, and opening a snapshot walks the reservation directory with a
    /// stat per reservation. `docs/06-limits.md` must say so.
    #[test]
    #[allow(
        clippy::expect_used,
        clippy::panic,
        reason = "a source marker that is missing must fail the test loudly"
    )]
    fn an_unpinned_ranking_request_walks_the_checkpoint_directory_before_its_cache() {
        fn after<'a>(text: &'a str, marker: &str) -> &'a str {
            text.split_once(marker)
                .unwrap_or_else(|| panic!("source names {marker:?}"))
                .1
        }
        let render = after(
            include_str!("indexstoprankingjson.rs"),
            "fn render(root: &Path, asked: &Asked) -> Result<Value, String> {\n",
        );
        let discover = render
            .find("None => Reader::latest_checkpoint(root, asked.identity, budget.bytes)?,")
            .expect("the unpinned arm discovers the checkpoint");
        let pinned = render
            .find("Some(value) => value,")
            .expect("a pinned request uses its pin");
        let cache = render
            .find("if !held.as_ref().is_some_and(|old| {")
            .expect("the cache comparison");
        assert!(discover < cache && pinned < cache);

        let (doc, latest) = include_str!("../../cli/src/index_stop_search_reader.rs")
            .split_once("    pub fn latest_checkpoint(")
            .expect("source names latest_checkpoint");
        assert!(
            doc.ends_with(
                "/// this one payload read are bounded cold work, not a constant-time index.\n    \
                 /// # Errors\n    \
                 /// Refuses missing, malformed, unacknowledged or over-budget checkpoints.\n"
            ),
            "the cli doc this section corrects is the one on latest_checkpoint"
        );
        let latest = latest.split_once("\n    }\n").expect("body ends").0;
        for call in [
            "Snapshot::open(root, NAMESPACE, identity)?",
            "snapshot.read(sequence, max_bytes)?",
        ] {
            assert!(latest.contains(call), "latest_checkpoint calls {call}");
        }

        let checkpoint = include_str!("../../cli/src/search_checkpoint.rs");
        let open = after(checkpoint, "    fn open_through(");
        assert!(
            open.split_once("\n    }\n")
                .expect("open_through ends")
                .0
                .contains("discover_through(&directory, through)?")
        );
        let walk = after(checkpoint, "fn discover_through(");
        let walk = walk.split_once("\n}\n").expect("discover_through ends").0;
        for step in [
            "fs::read_dir(directory)",
            "if index == directory_limit() {",
            "fs::symlink_metadata(entry.path().join(\"complete\"))",
        ] {
            assert!(walk.contains(step), "discover_through does {step}");
        }

        let expression = include_str!("expressionsearchjson.rs");
        assert!(
            expression
                .contains("Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)?")
        );
        assert!(
            include_str!("../../cli/src/expression_search_reader.rs")
                .contains("Snapshot::open(root, \"expression-search-v1\", identity)?")
        );
        let boolean = after(
            include_str!("booleansearchjson.rs"),
            "fn render(root: &Path, asked: &Asked, bytes: u64, nodes: u64) -> Result<Value, String> {\n",
        );
        assert!(
            boolean
                .trim_start()
                .starts_with("let reader=QualifiedSearch::open(root,asked.identity,bytes,nodes)")
        );

        let limits = include_str!("../../../docs/06-limits.md")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let section = after(
            &limits,
            "### Unpinned saved-ranking requests discover their checkpoint every time (D-0904)",
        );
        for claim in [
            "calls `Reader::latest_checkpoint` before the cache comparison",
            "every unpinned request, warm or cold",
            "A pinned request skips it.",
            "O(reservations) syscalls, not constant",
            "\"bounded cold work\"",
        ] {
            assert!(section.contains(claim), "06-limits states {claim:?}");
        }
    }

    const UNPINNED_CHILD: &str = "BRUTEX_TEST_RANKING_UNPINNED_ROOT";

    /// AN UNPINNED REQUEST RESOLVES ITS PIN BEFORE ANY READER IS OPENED. D-0904.
    ///
    /// On a root that holds no saved search, the unpinned request is refused
    /// by `latest_checkpoint`'s own words, unwrapped, which only its discovery
    /// call returns; the pinned request never makes that call and is refused
    /// by `Reader::open` under the comparison's context instead. Run in a child
    /// because `render` needs `BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES` and a test
    /// cannot set its own environment; see `crate::isolated`.
    #[test]
    fn an_unpinned_request_discovers_its_checkpoint_and_a_pinned_one_does_not() {
        if let Some(root) = std::env::var_os(UNPINNED_CHILD) {
            let root = Path::new(&root);
            let base = format!("identity={ID}&timeframe=1min");
            let unpinned = Asked::parse(&base).map(|asked| render(root, &asked));
            assert_eq!(
                unpinned,
                Ok(Err("no saved checkpoint exists for this search".to_owned())),
                "the unpinned arm refuses in latest_checkpoint's words"
            );
            let pinned = Asked::parse(&format!("{base}&sequence=1&completion={ID}"))
                .map(|asked| render(root, &asked));
            assert!(
                matches!(
                    &pinned,
                    Ok(Err(why)) if why.starts_with("Cumulative saved comparison: no saved checkpoint exists")
                ),
                "the pinned arm is refused by Reader::open: {pinned:?}"
            );
            // The proof the child took this branch is a file, not a print:
            // Gate 23 declares every print in the workspace.
            assert!(
                std::fs::write(root.join("child-ran"), b"discovered").is_ok(),
                "the child records that it ran both arms"
            );
            return;
        }
        let root = crate::scratch::path("ranking-unpinned-empty-root");
        let _ignored = std::fs::remove_dir_all(&root);
        assert!(std::fs::create_dir_all(&root).is_ok(), "the child's root");
        let _out = crate::isolated::rerun(
            "indexstoprankingjson::tests::an_unpinned_request_discovers_its_checkpoint_and_a_pinned_one_does_not",
            &[
                (UNPINNED_CHILD, root.as_os_str()),
                (
                    "BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES",
                    std::ffi::OsStr::new("1000"),
                ),
            ],
        );
        let proof = std::fs::read(root.join("child-ran"));
        let _ignored = std::fs::remove_dir_all(&root);
        assert_eq!(
            proof.ok().as_deref(),
            Some(&b"discovered"[..]),
            "the child ran both arms"
        );
    }
}
