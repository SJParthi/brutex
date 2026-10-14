//! Every route has a limits row and a growth bench (R3-14; RA-031, RA-064).
//!
//! # What was missing
//!
//! `docs/06-limits.md` is where a route's bound and its measured speed are
//! written, and `crates/api/benches/ratio.rs` is where a route proves it stays
//! flat as the store grows (1x, 10x, 100x). Nothing compared either with the
//! routes the server actually serves: the document fell days and dozens of
//! merges behind the app, and most `/db` routes had no growth bench at all,
//! with nobody told.
//!
//! # The rule
//!
//! * THE ROUTES are read from the server's own table: every `.route("..."`
//!   in `fn route_table` and `pub fn audited_router_serving` of
//!   `crates/api/src/server.rs`.
//! * A LIMITS ROW is a table row of `docs/06-limits.md`'s section whose
//!   heading starts `## Every route:`, whose first cell is the route in a
//!   code span: one row per route, so the reader finds it in one place.
//! * A GROWTH BENCH is a route named in a `ROUTES: &[&str] = &[...]`
//!   declaration of `crates/api/benches/ratio.rs` or of a file under
//!   `crates/api/benches/ratio/`: the bench that times it at 1x, 10x and 100x
//!   declares it.
//! * What is OWED is named here, with its owner and why: the whole route
//!   section while the document has none ([`SECTION_OWED`]), a row
//!   ([`ROWS_OWED`]) and a bench ([`BENCHES_OWED`]).
//!
//! A `/db` READ MAY NOT BE OWED A BENCH: every route of `logs::DB_READS` (what
//! a `/db` page reads its data from) must be declared by a bench, so the
//! operator's pages are measured at 1x, 10x and 100x without exception.
//!
//! RED: a route with no row or no bench that nothing here names; an owed
//! entry that is stale (the route left the table, or the row, bench or
//! section it says is missing exists); a row or a bench declaring a route the
//! table does not serve; a route owed twice. So a route cannot be added
//! without its row and its bench, or a line here saying who owes them, and
//! the owed lists can only shrink honestly.
//!
//! Std only, built by rustc directly (`rustc --edition=2024 --test` proves it
//! first); it reads files and starts no program. Run from the repository
//! root: `gates_routes .` prints every finding and exits non-zero on any.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::ExitCode;

/// The server's source, relative to the root.
pub const SERVER: &str = "crates/api/src/server.rs";
/// The limits document.
pub const LIMITS: &str = "docs/06-limits.md";
/// The bench root file.
pub const BENCH: &str = "crates/api/benches/ratio.rs";
/// The bench module directory.
pub const BENCH_DIR: &str = "crates/api/benches/ratio";
/// The heading prefix of the one section whose rows are route rows.
pub const SECTION: &str = "## Every route:";
/// Where the `/db` reads are listed (`logs::DB_READS`).
pub const LOGS: &str = "crates/api/src/logs.rs";

/// A group of routes that owe the same thing to the same owner, and why.
#[derive(Debug, Clone, Copy)]
pub struct Owed {
    /// Who owes it: a package, or `no package` with the reason in `why`.
    pub owner: &'static str,
    /// Why it is owed rather than present.
    pub why: &'static str,
    /// The routes.
    pub routes: &'static [&'static str],
}

/// The whole route section, while `docs/06-limits.md` has none: every route's
/// row is owed by this one line, and once the section exists this line is
/// stale and every route needs its row or a [`ROWS_OWED`] entry.
pub const SECTION_OWED: Option<(&str, &str)> = Some((
    "WP-99",
    "docs/06-limits.md has no `## Every route:` section: WP-99 (ledger close: limits with \
     p50/p99) writes it, one row per route with its bound and its measured p50, p99 and max",
));

/// Rows owed once the section exists. Empty while [`SECTION_OWED`] stands.
pub const ROWS_OWED: &[Owed] = &[];

/// The routes this build serves without a growth bench, each group with its
/// owner and why. Written from the table on 13 October 2026; a new route
/// arrives with its bench or with a line here.
pub const BENCHES_OWED: &[Owed] = &[
    Owed {
        owner: "no package (not a read)",
        why: "a write or a command: its cost is the work it starts (a pull, an import, a sweep, \
              a report line), measured by that work's own bench; the request does not grow with \
              the store",
        routes: &[
            "/autopilot/control",
            "/autopilot/pause",
            "/autopilot/resume",
            "/backtest/descend",
            "/backtest/run",
            "/engine/command",
            "/ingest/queue",
            "/masters/refresh",
            "/page-report",
            "/pull/fno",
            "/pull/recovery",
            "/pull/repair",
            "/pull/run",
            "/pull/run/stop",
            "/pull/spot",
            "/universe/resolve",
        ],
    },
    Owed {
        owner: "no package (does not read the store)",
        why: "fixed content, a table compiled into the binary, or the process's own counters \
              (`/speed.json` is O(routes x buckets), measured in R3-14's report)",
        routes: &[
            "/health",
            "/inspection.json",
            "/masters.js",
            "/speed.json",
            "/typeahead.js",
            "/vocab.json",
        ],
    },
    Owed {
        owner: "no package (the engine's read surface)",
        why: "reads the run ledger and the sweep evidence, not the bar store; its growth is the \
              ledger's, which no /db package owns",
        routes: &[
            "/backtest",
            "/backtest.json",
            "/backtest/audit.json",
            "/backtest/run.json",
            "/boolean-admission.json",
            "/boolean-campaign.json",
            "/boolean-candidates.json",
            "/boolean-oos.json",
            "/boolean-qualification.json",
            "/boolean-qualified-campaign.json",
            "/boolean-qualified-search.json",
            "/boolean-statistics.json",
            "/candidate-trades.json",
            "/engine/boolean-launch.json",
            "/engine/index-stop-launch.json",
            "/engine/top.json",
            "/expression-search.json",
            "/frontier.json",
            "/index-stop-candles.json",
            "/index-stop-qualification.json",
            "/index-stop-ranking.json",
            "/index-stop-vix.json",
            "/index-stop.json",
            "/live.json",
            "/selection-v6.json",
            "/sweep-evidence.json",
            "/trades.json",
        ],
    },
    Owed {
        owner: "no package (an operational page)",
        why: "reads the pull journal, the autopilot, the audit streams, the masters, the disk or \
              a fixed list, each under its own cap in docs/06-limits.md; not a /db read",
        routes: &[
            "/aliases.json",
            "/audit.json",
            "/audit/page",
            "/autopilot.json",
            "/bars",
            "/disk.json",
            "/feeds.json",
            "/folder.json",
            "/indexmap.json",
            "/ingest/refusals.json",
            "/ingest/status.json",
            "/logs",
            "/masters",
            "/masters/status.json",
            "/pull",
            "/pull/recovery.json",
            "/pull/repair.json",
            "/pull/run.json",
            "/scrub.json",
            "/store",
            "/universes.json",
            "/verify.json",
        ],
    },
];

/// The routes the server's table registers, in table order, each once.
///
/// # Errors
///
/// When neither table function is found, or a `.route(` is not followed by a
/// string literal: the table changed shape and this reading would be blind.
pub fn routes(server: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut found = 0;
    for head in ["\nfn route_table(", "\npub fn audited_router_serving("] {
        let Some(start) = server.find(head) else {
            continue;
        };
        found += 1;
        let body = server.get(start + 1..).unwrap_or("");
        let body = body
            .find("\n}\n")
            .map_or(body, |end| body.get(..end).unwrap_or(""));
        let mut rest = body;
        while let Some(at) = rest.find(".route(") {
            rest = rest.get(at + ".route(".len()..).unwrap_or("");
            let trimmed = rest.trim_start();
            let Some(lit) = trimmed.strip_prefix('"') else {
                return Err(format!(
                    "a `.route(` in {SERVER} is not followed by a string literal: {}",
                    trimmed.lines().next().unwrap_or("")
                ));
            };
            let Some(end) = lit.find('"') else {
                return Err(format!("an unclosed route literal in {SERVER}"));
            };
            let route = lit.get(..end).unwrap_or("").to_owned();
            if !out.contains(&route) {
                out.push(route);
            }
        }
    }
    if found == 0 {
        return Err(format!(
            "neither `fn route_table` nor `pub fn audited_router_serving` is in {SERVER}: the table \
             moved and this check would see no route"
        ));
    }
    if out.is_empty() {
        return Err(format!(
            "{SERVER}'s route table registered no route this check could read"
        ));
    }
    Ok(out)
}

/// The routes the limits document's route section has a row for, with the
/// line each is on; `None` when the document has no route section at all.
#[must_use]
pub fn limits_rows(doc: &str) -> Option<BTreeMap<String, usize>> {
    let mut out = BTreeMap::new();
    let mut seen = false;
    let mut inside = false;
    for (n, line) in doc.lines().enumerate() {
        if line.starts_with("## ") {
            inside = line.starts_with(SECTION);
            seen |= inside;
            continue;
        }
        if !inside {
            continue;
        }
        let Some(cell) = line.strip_prefix('|') else {
            continue;
        };
        let first = cell.split('|').next().unwrap_or("").trim();
        if let Some(route) = first.strip_prefix('`').and_then(|r| r.strip_suffix('`'))
            && route.starts_with('/')
        {
            let _first_wins = out.entry(route.to_owned()).or_insert(n + 1);
        }
    }
    seen.then_some(out)
}

/// The routes a bench file declares in its `ROUTES` lists.
#[must_use]
pub fn bench_routes(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("ROUTES: &[&str] = &[") {
        rest = rest.get(at + "ROUTES: &[&str] = &[".len()..).unwrap_or("");
        let list = rest
            .find(']')
            .map_or(rest, |end| rest.get(..end).unwrap_or(""));
        for part in list.split(',') {
            let part = part.trim();
            if let Some(route) = part.strip_prefix('"').and_then(|p| p.strip_suffix('"')) {
                out.push(route.to_owned());
            }
        }
    }
    out
}

/// What is owed, as the check reads it.
#[derive(Debug, Clone, Copy)]
pub struct Ledger<'a> {
    /// The whole route section, while the document has none.
    pub section: Option<(&'a str, &'a str)>,
    /// Rows owed once the section exists.
    pub rows: &'a [Owed],
    /// Benches owed.
    pub benches: &'a [Owed],
}

/// The owed routes of `groups`, each with its owner; a route owed twice is
/// reported in `twice`.
fn owed_map<'a>(groups: &'a [Owed], twice: &mut Vec<String>) -> BTreeMap<&'a str, &'a str> {
    let mut out = BTreeMap::new();
    for group in groups {
        for route in group.routes {
            if out.insert(*route, group.owner).is_some() {
                twice.push((*route).to_owned());
            }
        }
    }
    out
}

/// Every finding over the readings; empty is green.
#[must_use]
pub fn findings(
    table: &[String],
    rows: Option<&BTreeMap<String, usize>>,
    benched: &BTreeSet<String>,
    owed: &Ledger<'_>,
) -> Vec<String> {
    let mut out = Vec::new();
    let served: BTreeSet<&str> = table.iter().map(String::as_str).collect();
    let mut twice = Vec::new();
    let rows_owed = owed_map(owed.rows, &mut twice);
    let benches_owed = owed_map(owed.benches, &mut twice);
    for route in twice {
        out.push(format!(
            "{route} is owed twice in one list: one line per route"
        ));
    }
    match (rows, owed.section) {
        (None, None) => out.push(format!(
            "{LIMITS} has no `{SECTION}` section and SECTION_OWED names no owner"
        )),
        (Some(_), Some((owner, _))) => out.push(format!(
            "SECTION_OWED says {LIMITS} lacks its route section ({owner}), and it has one now: \
             set it to None"
        )),
        _ => {}
    }
    for route in table {
        if let Some(rows) = rows
            && !rows.contains_key(route)
            && !rows_owed.contains_key(route.as_str())
        {
            out.push(format!(
                "{route}: no limits row in {LIMITS} (`{SECTION}` section) and no ROWS_OWED line"
            ));
        }
        if !benched.contains(route) && !benches_owed.contains_key(route.as_str()) {
            out.push(format!(
                "{route}: no growth bench (a `ROUTES` list in {BENCH} or {BENCH_DIR}/) and no \
                 BENCHES_OWED line"
            ));
        }
    }
    for (list, owed_here) in [("ROWS_OWED", &rows_owed), ("BENCHES_OWED", &benches_owed)] {
        for (route, owner) in owed_here {
            if !served.contains(route) {
                out.push(format!(
                    "{list} names {route} ({owner}), which the route table does not serve: \
                     remove it"
                ));
                continue;
            }
            let present = if list == "ROWS_OWED" {
                rows.is_some_and(|rows| rows.contains_key(*route))
            } else {
                benched.contains(*route)
            };
            if present {
                out.push(format!(
                    "{list} says {route} lacks it ({owner}), and it has one now: remove the line"
                ));
            }
        }
    }
    if let Some(rows) = rows {
        for (route, line) in rows {
            if !served.contains(route.as_str()) {
                out.push(format!(
                    "{LIMITS}:{line}: a row for {route}, which the route table does not serve"
                ));
            }
        }
    }
    for route in benched {
        if !served.contains(route.as_str()) {
            out.push(format!(
                "a growth bench declares {route}, which the route table does not serve"
            ));
        }
    }
    out
}

/// The routes `logs::DB_READS` lists in `source` (`crates/api/src/logs.rs`).
///
/// # Errors
///
/// No `DB_READS` declaration, or an empty one: the rule would go blind.
pub fn db_reads(source: &str) -> Result<Vec<String>, String> {
    let Some((_, rest)) = source.split_once("pub const DB_READS: [&str; ") else {
        return Err(format!("{LOGS} declares no `pub const DB_READS`"));
    };
    let Some((_, list)) = rest.split_once("= [") else {
        return Err(format!("{LOGS}: `DB_READS` has no list"));
    };
    let Some((list, _)) = list.split_once("];") else {
        return Err(format!("{LOGS}: `DB_READS` is not closed"));
    };
    let out: Vec<String> = list
        .split(',')
        .map(str::trim)
        .filter_map(|item| item.strip_prefix('"').and_then(|i| i.strip_suffix('"')))
        .map(str::to_owned)
        .collect();
    if out.is_empty() {
        return Err(format!("{LOGS}: `DB_READS` lists nothing"));
    }
    Ok(out)
}

/// The findings of the `/db` rule: each read in `reads` that no bench
/// declares, owed or not.
#[must_use]
pub fn unbenched_reads(reads: &[String], benched: &BTreeSet<String>) -> Vec<String> {
    reads
        .iter()
        .filter(|route| !benched.contains(*route))
        .map(|route| {
            format!(
                "{route}: a /db read (`logs::DB_READS`) with no growth bench; a /db read may not \
                 be owed one (R3-14, RA-064)"
            )
        })
        .collect()
}

/// Reads the tree at `root` and returns how many routes were read, how many
/// have a bench, and the findings.
///
/// # Errors
///
/// A file that cannot be read, or a route table this check cannot read.
pub fn check(root: &Path) -> Result<(usize, usize, Vec<String>), String> {
    let read = |path: &Path| {
        std::fs::read_to_string(path).map_err(|why| format!("{}: {why}", path.display()))
    };
    let table = routes(&read(&root.join(SERVER))?)?;
    let rows = limits_rows(&read(&root.join(LIMITS))?);
    let mut benched: BTreeSet<String> = bench_routes(&read(&root.join(BENCH))?)
        .into_iter()
        .collect();
    let mut modules: Vec<_> = std::fs::read_dir(root.join(BENCH_DIR))
        .map_err(|why| format!("{BENCH_DIR}: {why}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .collect();
    modules.sort();
    for module in modules {
        benched.extend(bench_routes(&read(&module)?));
    }
    let ledger = Ledger {
        section: SECTION_OWED,
        rows: ROWS_OWED,
        benches: BENCHES_OWED,
    };
    let with_bench = table.iter().filter(|r| benched.contains(*r)).count();
    let mut found = findings(&table, rows.as_ref(), &benched, &ledger);
    found.extend(unbenched_reads(
        &db_reads(&read(&root.join(LOGS))?)?,
        &benched,
    ));
    Ok((table.len(), with_bench, found))
}

#[allow(
    dead_code,
    reason = "the entry point when built by rustc; unused under --test"
)]
fn main() -> ExitCode {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    match check(Path::new(&root)) {
        Err(why) => {
            eprintln!("gates_routes refused: {why}");
            ExitCode::FAILURE
        }
        Ok((n, benched, found)) if found.is_empty() => {
            let owed: usize = BENCHES_OWED.iter().map(|g| g.routes.len()).sum();
            println!(
                "gates_routes: {n} routes; {benched} with a growth bench, {owed} owed with an \
                 owner; limits rows: {}",
                SECTION_OWED.map_or_else(
                    || "every route has one or an owner".to_owned(),
                    |(owner, _)| format!("the whole route section is owed by {owner}")
                )
            );
            ExitCode::SUCCESS
        }
        Ok((n, _, found)) => {
            eprintln!("gates_routes: {} finding(s) over {n} routes:", found.len());
            for line in &found {
                eprintln!("  {line}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER_SRC: &str = "
fn other() {
    axum::Router::new().route(\"/ok\", get(ok))
}

fn route_table(assets: A) -> Router {
    axum::Router::new()
        .route(\"/a.json\", get(a))
        // a comment
        .route(
            \"/b.json\",
            get(b),
        )
        .route(\"/a.json\", post(a2))
        .fallback(f)
}

pub fn audited_router_serving(site: S) -> Router {
    let journaled = route_table(front)
        .route(\"/inspection.json\", get(i));
    admitted(journaled)
}
";

    const DOC: &str = "
## 3. Something else
| `/x.json` | a row of another section |

## Every route: its bound, its measurement and its growth bench
| route | bound |
|---|---|
| `/a.json` | O(1) |
| `/gone.json` | O(1) |
| not a route | x |

## Next
| `/b.json` | outside the section |
";

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    const NONE: Ledger<'static> = Ledger {
        section: None,
        rows: &[],
        benches: &[],
    };

    #[test]
    fn the_table_is_read_from_both_functions_once_each_and_nothing_else() {
        let table = routes(SERVER_SRC).expect("readable");
        assert_eq!(table, ["/a.json", "/b.json", "/inspection.json"]);
        assert!(
            routes("fn main() {}").is_err(),
            "no table is refused, not empty"
        );
        assert!(routes("\nfn route_table() {\n    x.route(path, get(h))\n}\n").is_err());
    }

    #[test]
    fn only_the_route_sections_rows_count_and_no_section_is_none() {
        let rows = limits_rows(DOC).expect("the section is there");
        assert_eq!(
            rows.keys().cloned().collect::<Vec<_>>(),
            ["/a.json", "/gone.json"]
        );
        assert_eq!(limits_rows("## 3. Other\n| `/a.json` | x |\n"), None);
    }

    #[test]
    fn bench_lists_are_read() {
        let src = "pub(crate) const ROUTES: &[&str] = &[\"/a.json\", \"/c.json\"];\nconst ROUTES: &[&str] = &[\n    \"/d.json\",\n];";
        assert_eq!(bench_routes(src), ["/a.json", "/c.json", "/d.json"]);
    }

    #[test]
    fn a_route_with_no_row_and_no_bench_is_red_and_an_owed_line_names_it() {
        let table = vec!["/a.json".to_owned(), "/b.json".to_owned()];
        let rows = limits_rows(DOC);
        let benched = set(&["/a.json"]);
        let red = findings(&table, rows.as_ref(), &benched, &NONE);
        assert!(
            red.iter().any(|f| f.starts_with("/b.json: no limits row")),
            "{red:?}"
        );
        assert!(
            red.iter()
                .any(|f| f.starts_with("/b.json: no growth bench")),
            "{red:?}"
        );
        assert!(
            red.iter()
                .any(|f| f.contains("/gone.json, which the route table")),
            "{red:?}"
        );
        let owed = Ledger {
            section: None,
            rows: &[Owed {
                owner: "WP-99",
                why: "why",
                routes: &["/b.json"],
            }],
            benches: &[Owed {
                owner: "X",
                why: "why",
                routes: &["/b.json"],
            }],
        };
        let named = findings(&table, rows.as_ref(), &benched, &owed);
        assert_eq!(named.len(), 1, "only the stale row is left: {named:?}");
    }

    #[test]
    fn a_missing_section_is_owed_whole_or_red_and_a_stale_owed_line_is_red() {
        let table = vec!["/a.json".to_owned()];
        let benched = set(&["/a.json"]);
        let none = findings(&table, None, &benched, &NONE);
        assert_eq!(none.len(), 1, "{none:?}");
        assert!(none[0].contains("no `## Every route:` section"));
        let section = Ledger {
            section: Some(("WP-99", "why")),
            ..NONE
        };
        assert!(findings(&table, None, &benched, &section).is_empty());
        let rows = limits_rows("## Every route: x\n| `/a.json` | b |\n");
        let stale = findings(&table, rows.as_ref(), &benched, &section);
        assert!(stale[0].contains("set it to None"), "{stale:?}");

        let lines = Ledger {
            section: None,
            rows: &[Owed {
                owner: "WP-99",
                why: "why",
                routes: &["/a.json", "/gone"],
            }],
            benches: &[
                Owed {
                    owner: "X",
                    why: "why",
                    routes: &["/a.json"],
                },
                Owed {
                    owner: "Y",
                    why: "why",
                    routes: &["/a.json"],
                },
            ],
        };
        let red = findings(&table, rows.as_ref(), &set(&["/a.json", "/z.json"]), &lines);
        assert!(red.iter().any(|f| f.contains("owed twice")), "{red:?}");
        assert!(
            red.iter()
                .any(|f| f.starts_with("ROWS_OWED says /a.json lacks it")),
            "{red:?}"
        );
        assert!(
            red.iter()
                .any(|f| f.starts_with("BENCHES_OWED says /a.json lacks it")),
            "{red:?}"
        );
        assert!(
            red.iter().any(|f| f.contains("ROWS_OWED names /gone")),
            "{red:?}"
        );
        assert!(
            red.iter().any(|f| f.contains("declares /z.json")),
            "{red:?}"
        );
    }

    /// **EVERY ROUTE HAS A LIMITS ROW AND A GROWTH BENCH** (R3-14), or a line
    /// here naming who owes it, over the real tree when this test runs from
    /// the repository root (as CI and the checks command run it). Every
    /// `/db` read (`api::logs::DB_READS`) is held to it with the rest.
    /// A `/db` read owed a bench is still red, and the list is read whole.
    #[test]
    fn a_db_read_may_not_be_owed_a_bench() {
        let logs = "x\npub const DB_READS: [&str; 2] = [\n    \"/a.json\",\n    \"/b.json\",\n];\n";
        assert_eq!(db_reads(logs).expect("read"), ["/a.json", "/b.json"]);
        assert!(db_reads("nothing").is_err());
        assert!(db_reads("pub const DB_READS: [&str; 0] = [];").is_err());
        let benched = set(&["/a.json"]);
        let found = unbenched_reads(&db_reads(logs).expect("read"), &benched);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("/b.json: a /db read"), "{found:?}");
    }

    #[test]
    fn every_route_has_a_limits_row_and_a_growth_bench() {
        let root = std::env::var("BRUTEX_ROOT").unwrap_or_else(|_| ".".to_owned());
        let root = Path::new(&root);
        assert!(
            root.join(SERVER).is_file(),
            "run from the repository root, or set BRUTEX_ROOT: {SERVER} is not here"
        );
        let (n, benched, found) = check(root).expect("the tree is readable");
        assert!(n > 50, "read only {n} routes: the table reading went blind");
        assert!(benched >= 19, "only {benched} routes read as benched");
        assert!(
            found.is_empty(),
            "{} finding(s):\n{}",
            found.len(),
            found.join("\n")
        );
    }
}
