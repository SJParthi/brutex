//! Generated ledgers keep selection, quality and read-only reports consistent.

use crate::results::{Record, Results, field};

struct Fixture(std::path::PathBuf);

impl Fixture {
    fn new(rows: &[Record]) -> Result<Self, Box<dyn std::error::Error>> {
        // CANONICAL FROM THE START. macOS hands out a temp dir under `/var`,
        // a symlink to `/private/var`. The public listing resolves the root it
        // is configured with and prints the resolved spelling, so a fixture
        // named by the unresolved one compared two spellings of one file and
        // failed on every Mac while passing on Linux.
        let fixture = Self(std::fs::canonicalize(crate::verification_scratch()?)?);
        let mut ledger = Results::open(&fixture.0)?;
        for row in rows {
            ledger.append(row)?;
        }
        Ok(fixture)
    }

    fn bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        std::fs::read(Results::path(&self.0))
    }

    fn unchanged(&self, before: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(self.bytes()?, before);
        assert!(!crate::frontier::Frontier::path(&self.0).exists());
        assert!(!crate::result_set::Receipts::path(&self.0).exists());
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn row(id: u8, profit: i64, trades: u64) -> Record {
    Record {
        identity: [id; 32],
        finished_micros: i64::from(id),
        feed: field("zerodha"),
        underlying: field("NIFTY"),
        timeframe: field("15min"),
        from_year: 2026,
        from_month: 1,
        to_year: 2026,
        to_month: 1,
        months_asked: 1,
        months_found: 1,
        bars: 1_000,
        min_hits: 100 + u64::from(id),
        combinations: 1_000 + u64::from(id),
        depth: 1,
        halted: 0,
        trades,
        pessimistic: profit,
        optimistic: profit + 100,
        worst_trade: -100 * i64::from(id),
        max_drawdown: 100 * i64::from(id),
        winner_mae: 100 * i64::from(id),
        winner_mfe: 200 * i64::from(id),
        all_mae: 300 * i64::from(id),
        exit_rungs: [-1; 5],
        mask_words: [0; 6],
    }
}

#[test]
fn tied_list_footer_and_quality_choose_the_same_newest_run_as_top()
-> Result<(), Box<dyn std::error::Error>> {
    let older = row(1, -500, 2);
    let newer = Record {
        // Append order remains authoritative if the wall clock moves backward.
        finished_micros: -1,
        ..row(2, -500, 4)
    };
    let fixture = Fixture::new(&[older, newer])?;
    let before = fixture.bytes()?;
    let top = crate::top_at(&fixture.0, None, None);
    assert!(top.contains(&newer.identity_hex()), "{top}");
    let listing = crate::results_at(&fixture.0, None, None);
    let best = listing
        .lines()
        .find(|line| line.contains("BEST COMPLETE RUN"))
        .ok_or("the generated complete trades need a winner")?;
    assert!(best.contains("at min_hits 102"), "{listing}");
    assert!(
        listing.ends_with(&crate::quality_block(&newer)),
        "{listing}"
    );
    fixture.unchanged(&before)
}

#[test]
fn unpriced_zero_does_not_beat_a_loss_or_supply_another_runs_quality()
-> Result<(), Box<dyn std::error::Error>> {
    let loss = row(3, -1_000, 2);
    let unpriced = row(4, 0, 0);
    let halted = Record {
        halted: 1,
        ..row(5, 10_000, 10)
    };
    let fixture = Fixture::new(&[loss, unpriced, halted])?;
    let before = fixture.bytes()?;
    let listing = crate::results_at(&fixture.0, None, None);
    assert!(listing.contains("at min_hits 103"), "{listing}");
    assert!(listing.ends_with(&crate::quality_block(&loss)), "{listing}");
    let top = crate::top_at(&fixture.0, None, None);
    assert!(top.contains(&loss.identity_hex()), "{top}");
    fixture.unchanged(&before)
}

#[test]
fn top_never_selects_the_zero_trade_sentinel_over_a_real_loss()
-> Result<(), Box<dyn std::error::Error>> {
    let loss = row(6, -1_000, 2);
    let fixture = Fixture::new(&[loss, row(7, 0, 0)])?;
    let before = fixture.bytes()?;
    let top = crate::top_at(&fixture.0, None, None);
    assert!(top.contains(&loss.identity_hex()), "{top}");
    fixture.unchanged(&before)
}

#[test]
fn an_unpriced_or_halted_only_ledger_has_no_winner_and_no_trade_quality()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[
        row(8, 0, 0),
        Record {
            halted: u8::MAX,
            ..row(9, 100_000, u64::MAX)
        },
    ])?;
    let before = fixture.bytes()?;
    let listing = crate::results_at(&fixture.0, None, None);
    assert!(listing.contains("NO COMPLETE RUN"), "{listing}");
    assert!(!listing.contains("TRADE QUALITY"), "{listing}");
    let top = crate::top_at(&fixture.0, None, None);
    assert!(top.contains("NO COMPLETE RUN"), "{top}");
    assert!(top.contains("traded nothing"), "{top}");
    fixture.unchanged(&before)
}

#[test]
fn list_filters_and_omitted_rows_do_not_change_the_selected_totals()
-> Result<(), Box<dyn std::error::Error>> {
    let first = row(10, 1_000, 2);
    let mut records = vec![first];
    for id in 11..=55 {
        records.push(row(id, 500, 1));
    }
    let other_feed = Record {
        feed: field("groww"),
        ..row(56, 3_000, 3)
    };
    let other_instrument = Record {
        underlying: field("BANKNIFTY"),
        ..row(57, 2_000, 4)
    };
    records.extend([other_feed, other_instrument]);
    let fixture = Fixture::new(&records)?;
    let before = fixture.bytes()?;
    for (feed, underlying, expected) in [
        (None, None, other_feed),
        (Some("zerodha"), None, other_instrument),
        (None, Some("NIFTY"), other_feed),
        (Some("zerodha"), Some("NIFTY"), first),
    ] {
        let listing = crate::results_at(&fixture.0, feed, underlying);
        assert!(listing.contains("further row(s) NOT SHOWN"), "{listing}");
        assert!(
            listing.contains(&format!("at min_hits {}", expected.min_hits)),
            "{listing}"
        );
        assert!(
            listing.ends_with(&crate::quality_block(&expected)),
            "{listing}"
        );
        let top = crate::top_at(&fixture.0, feed, underlying);
        assert!(top.contains(&expected.identity_hex()), "{top}");
    }
    let none = crate::results_at(&fixture.0, Some("unmatched"), Some("NIFTY"));
    assert!(none.contains("NO COMPLETE RUN"), "{none}");
    assert!(!none.contains("TRADE QUALITY"), "{none}");
    fixture.unchanged(&before)
}

/// **A listing that prints a stock's figures says what they are made of,
/// before the table; a listing of index runs is what it was.** AF-19.
///
/// `cli results` names a BEST COMPLETE RUN on the worst-case total and prices
/// what it risked, so it ranks. With a RELIANCE row as that winner it printed
/// neither the gross label nor the corporate-action sentence, while `cli top`
/// on the same row printed both. The statement is decided on the rows the
/// page prints a figure for, the table's and the winner's: a stock in the
/// table under an index winner puts it there, a stock that is the winner but
/// beyond the table puts it there, and a stock that is neither adds nothing.
#[test]
fn a_listing_that_prints_a_stock_states_gross_and_corporate_actions_and_an_index_listing_does_not()
-> Result<(), Box<dyn std::error::Error>> {
    let note = runner::audit::CostScope::CashEquity.report_note();
    let stock = |id: u8, profit: i64| Record {
        underlying: field("RELIANCE"),
        ..row(id, profit, 3)
    };
    // Nothing inserted between the counts and the table on an index listing.
    let bare = |listing: &str, matching: usize| {
        listing.contains(&format!(
            "  matching                                {matching}\n\n  feed     rung"
        ))
    };

    let index = Fixture::new(&[row(62, 500, 2), row(63, 700, 3)])?;
    let listing = crate::results_at(&index.0, None, None);
    assert!(bare(&listing, 2), "{listing}");
    for equity_only in ["GROSS OF EVERY CHARGE", "CORPORATE ACTIONS"] {
        assert!(!listing.contains(equity_only), "{listing}");
    }

    let mixed = Fixture::new(&[row(64, 500, 2), stock(65, 900), row(66, 700, 3)])?;
    let before = mixed.bytes()?;
    let listing = crate::results_at(&mixed.0, None, None);
    assert!(
        listing.contains("BEST COMPLETE RUN: zerodha RELIANCE"),
        "premise: the stock wins:\n{listing}"
    );
    let at = listing.find(&note).ok_or("the note is on the page")?;
    let table = listing.find("\n  feed     rung").ok_or("the table")?;
    assert!(at < table, "before the table:\n{listing}");
    assert!(
        listing.contains(&format!(
            "  matching                                3\n\n{note}  feed"
        )),
        "straight after the counts:\n{listing}"
    );
    assert_eq!(
        listing
            .matches(runner::audit::CORPORATE_ACTIONS_UNCHECKED)
            .count(),
        1,
        "{listing}"
    );
    mixed.unchanged(&before)?;
    drop(mixed);

    // A stock in the table with an index as the winner puts it on the page
    // too: the row's worst and best are the stock's figures. Deciding on the
    // winner alone would print this page bare.
    let shown = Fixture::new(&[row(149, 10_000, 3), stock(150, -10_000)])?;
    let listing = crate::results_at(&shown.0, None, None);
    assert!(
        listing.contains("BEST COMPLETE RUN: zerodha NIFTY"),
        "premise: the index wins:\n{listing}"
    );
    assert!(
        listing.contains(&format!(
            "  matching                                2\n\n{note}  feed"
        )),
        "straight after the counts, before the table:\n{listing}"
    );
    assert_eq!(
        listing
            .matches(runner::audit::CORPORATE_ACTIONS_UNCHECKED)
            .count(),
        1,
        "{listing}"
    );
    drop(shown);

    // A stock beyond the table that is still the winner puts it on the page.
    let mut records = vec![stock(67, 10_000)];
    records.extend((68..=107).map(|id| row(id, 500, 1)));
    let winner = Fixture::new(&records)?;
    let listing = crate::results_at(&winner.0, None, None);
    assert!(listing.contains("further row(s) NOT SHOWN"), "{listing}");
    assert!(
        listing.contains("BEST COMPLETE RUN: zerodha RELIANCE"),
        "{listing}"
    );
    assert!(listing.contains(&note), "{listing}");
    drop(winner);

    // A stock that is neither in the table nor the winner adds nothing.
    let mut records = vec![stock(108, -10_000)];
    records.extend((109..=148).map(|id| row(id, 500, 1)));
    let hidden = Fixture::new(&records)?;
    let listing = crate::results_at(&hidden.0, None, None);
    assert!(listing.contains("further row(s) NOT SHOWN"), "{listing}");
    assert!(bare(&listing, 41), "{listing}");
    assert!(!listing.contains("CORPORATE ACTIONS"), "{listing}");
    Ok(())
}

#[test]
fn absent_empty_and_corrupt_ledgers_are_read_without_creating_or_repairing_files()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[])?;
    let missing = fixture.0.join("uncreated");
    assert!(crate::results_at(&missing, None, None).starts_with("refused:"));
    assert!(crate::top_at(&missing, None, None).contains("NO RUN HAS BEEN RECORDED YET"));
    assert!(!missing.exists());
    let before = fixture.bytes()?;
    assert!(crate::results_at(&fixture.0, None, None).contains("Nothing has been recorded yet"));
    fixture.unchanged(&before)?;

    let complete = row(58, 100, 1);
    Results::open(&fixture.0)?.append(&complete)?;
    let intact = fixture.bytes()?;
    let mut ragged = intact.clone();
    ragged.push(1);
    let mut bad_seal = intact.clone();
    *bad_seal.get_mut(17).ok_or("record payload exists")? ^= 1;
    for damaged in [ragged, bad_seal] {
        std::fs::write(Results::path(&fixture.0), &damaged)?;
        let listing = crate::results_at(&fixture.0, None, None);
        let top = crate::top_at(&fixture.0, None, None);
        assert!(listing.starts_with("refused:"), "{listing}");
        assert!(top.starts_with("refused:"), "{top}");
        fixture.unchanged(&damaged)?;
    }
    std::fs::write(Results::path(&fixture.0), &intact)?;
    assert!(crate::top_at(&fixture.0, None, None).contains(&complete.identity_hex()));
    fixture.unchanged(&intact)
}

#[test]
fn quality_averages_preserve_large_trade_counts_and_ratio_intermediates()
-> Result<(), Box<dyn std::error::Error>> {
    for (total, count, expected) in [
        (-1_000, u64::MAX, "₹0.00"),
        (i64::MIN, i64::MIN.unsigned_abs(), "-₹0.01"),
        (i64::MAX, u64::MAX, "₹0.00"),
        (-1_001, 2, "-₹5.00"),
        (0, 0, "₹0.00"),
    ] {
        let record = Record {
            pessimistic: total,
            optimistic: total,
            ..row(59, 0, count)
        };
        let quality = crate::quality_block(&record);
        let line = quality
            .lines()
            .find(|line| line.contains("per trade, worst-case fills"))
            .ok_or("per-trade value must be disclosed")?;
        assert!(
            line.contains(expected),
            "total {total}, count {count}: {line}"
        );
    }
    for (adverse, expected) in [
        (i64::MAX, "1.0x"),
        (1, "9223372036854775807.0x"),
        (0, "no winner dipped"),
    ] {
        let record = Record {
            winner_mae: adverse,
            winner_mfe: i64::MAX,
            ..row(60, 0, 1)
        };
        let quality = crate::quality_block(&record);
        let line = quality
            .lines()
            .find(|line| line.contains("reward per unit of risk (means)"))
            .ok_or("risk ratio must be disclosed")?;
        assert!(line.contains(expected), "{line}");
    }
    Ok(())
}

#[test]
fn public_listing_obeys_the_configured_owned_root() -> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "BRUTEX_TEST_RESULTS_REPORT_ROOT";
    if let Some(root) = std::env::var_os(CHILD) {
        let root = std::path::PathBuf::from(root);
        let direct = crate::results_at(&root, Some("zerodha"), Some("NIFTY"));
        let public = crate::results_list(Some("zerodha"), Some("NIFTY"));
        assert!(public.contains("at min_hits 161"), "{public}");
        assert_eq!(public, direct);
        return Ok(());
    }
    let fixture = Fixture::new(&[row(61, 500, 2)])?;
    let before = fixture.bytes()?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "results_report_tests::public_listing_obeys_the_configured_owned_root",
            "--nocapture",
        ])
        .env(CHILD, &fixture.0)
        .env("BRUTEX_STORE", &fixture.0)
        .output()?;
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fixture.unchanged(&before)
}

/// Ninety rows across two feeds, three instruments, halted and unpriced rows.
fn mixed_ledger() -> Vec<Record> {
    (1_u8..=90)
        .map(|id| {
            let mut record = row(id, i64::from(id % 7) * 100 - 300, u64::from(id % 4));
            if id % 11 == 0 {
                record.halted = 1;
            }
            if id % 5 == 0 {
                record.feed = field("groww");
            }
            if id % 3 == 0 {
                record.underlying = field("BANKNIFTY");
            }
            if id % 13 == 0 {
                record.underlying = field("RELIANCE");
            }
            record
        })
        .collect()
}

const FILTERS: [(Option<&str>, Option<&str>); 6] = [
    (None, None),
    (Some("zerodha"), None),
    (None, Some("NIFTY")),
    (Some("zerodha"), Some("NIFTY")),
    (Some("groww"), Some("RELIANCE")),
    (Some("unmatched"), None),
];

fn counted<T>(call: impl FnOnce() -> T) -> (T, u64, u64) {
    crate::results::OPENS.with(|n| n.set(0));
    crate::results::ROW_READS.with(|n| n.set(0));
    let out = call();
    (
        out,
        crate::results::OPENS.with(std::cell::Cell::get),
        crate::results::ROW_READS.with(std::cell::Cell::get),
    )
}

/// **`cli top` and `cli results` open the ledger once and read each row once.**
/// OS-7, W2-cli8-5, D-2310.
///
/// Both opened the ledger -- whose identity pass reads every row -- and then
/// read every row again through `Results::read`: two reads per row per call.
/// Counted on a 90-row ledger, every filter, intact and with two damaged rows:
/// exactly one open and exactly 90 row reads, where the old walk made 180.
#[test]
fn top_and_results_read_each_row_once_in_one_open() -> Result<(), Box<dyn std::error::Error>> {
    let records = mixed_ledger();
    let rows = u64::try_from(records.len())?;
    let fixture = Fixture::new(&records)?;
    let intact = fixture.bytes()?;
    let mut damaged = intact.clone();
    for index in [10_usize, 50] {
        *damaged
            .get_mut(crate::results::HEADER_BYTES + index * crate::results::STRIDE_BYTES + 40)
            .ok_or("the row exists")? ^= 1;
    }
    for bytes in [&intact, &damaged] {
        std::fs::write(Results::path(&fixture.0), bytes)?;
        for (feed, underlying) in FILTERS {
            let (listing, opens, reads) =
                counted(|| crate::results_at(&fixture.0, feed, underlying));
            assert_eq!((opens, reads), (1, rows), "{listing}");
            let (top, opens, reads) = counted(|| crate::top_at(&fixture.0, feed, underlying));
            assert_eq!((opens, reads), (1, rows), "{top}");
        }
    }
    // THE SAME DAMAGED ROW IS NAMED AS BEFORE: the newest-first listing met
    // row 50 first, the forward top met row 10 first.
    let listing = crate::results_at(&fixture.0, None, None);
    assert!(
        listing.starts_with("refused: record 50 does not match its seal"),
        "{listing}"
    );
    let top = crate::top_at(&fixture.0, None, None);
    assert!(
        top.starts_with("refused: record 10 does not match its seal"),
        "{top}"
    );
    fixture.unchanged(&damaged)
}

/// **The one-pass listing prints what the two-pass one printed.** D-2310.
///
/// The reference below is the walk that was removed: every row read back
/// through `Results::read`, newest first, held whole, the best chosen over the
/// whole slice. The table, the counts, the omitted-row line, the winner and its
/// quality block must come out of the fold byte for byte.
#[test]
fn the_one_pass_listing_matches_the_removed_newest_first_walk()
-> Result<(), Box<dyn std::error::Error>> {
    let records = mixed_ledger();
    let fixture = Fixture::new(&records)?;
    let mut store = Results::open_read(&fixture.0)?;
    let count = store.len()?;
    for (feed, underlying) in FILTERS {
        let mut reference = Vec::new();
        for back in 1..=count {
            let record = store.read(count - back)?;
            if feed.is_none_or(|f| crate::results::read_field(&record.feed) == f)
                && underlying.is_none_or(|u| crate::results::read_field(&record.underlying) == u)
            {
                reference.push(record);
            }
        }
        let best = reference
            .iter()
            .rev()
            .filter(|r| r.has_complete_trade_total())
            .max_by_key(|r| r.pessimistic);
        let listing = crate::results_at(&fixture.0, feed, underlying);
        let mut table = String::new();
        crate::results_table(&mut table, &reference);
        assert!(
            listing.contains(&format!(
                "  matching                                {}\n\n{}{table}",
                reference.len(),
                crate::listing_equity_note(&reference, best)
            )),
            "{listing}"
        );
        let hidden = reference.len().saturating_sub(40);
        assert_eq!(
            listing.contains(&format!("  ... {hidden} further row(s) NOT SHOWN.")),
            hidden > 0,
            "{listing}"
        );
        let mut tail = format!("\n{}", crate::best_complete_line(best));
        if let Some(best) = best {
            tail.push_str(&crate::quality_block(best));
            let top = crate::top_at(&fixture.0, feed, underlying);
            assert!(top.contains(&best.identity_hex()), "{top}");
        }
        assert!(listing.ends_with(&tail), "{listing}");
    }
    Ok(())
}
