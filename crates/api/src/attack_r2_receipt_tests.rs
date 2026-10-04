#![cfg(test)]
//! ATTACK ROUND 2, RECEIPT: the candles a decoder skipped (D-3122) must reach
//! the two surfaces an operator reads — the live spot receipt and the journal
//! record behind `/audit` — and neither may print an equation that is false.
//!
//! Round 1 carried `DecodeSkips` from the decoder into `Ingested`, counted the
//! skipped candles into `rows_read`, and made `balances` account for them. The
//! receipt that renders `balances` was not touched: it printed
//! `"yes — 3 read = 2 stored + 0 folded + 0 dropped"`, an equation that is
//! arithmetically false, and the journal note said every row was "stored,
//! folded into an open bar, or dropped" over a candle that was none of the
//! three. D-3180.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("attack-r2-receipt-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("manifest")).expect("mkdir");
    dir
}

fn window() -> pull::session::Window {
    let from = Day::new(2025, 7, 1).expect("a real day");
    let to = Day::new(2025, 7, 3).expect("a real day");
    pull::session::Window::new(from, to).expect("a legal window")
}

/// A clean run of `read` candles: `stored` became bars, the rest were skipped
/// by the decoder under the four reasons in turn.
fn run(stored: usize, skips: pull::fetch::DecodeSkips) -> pull::ingest::Ingested {
    pull::ingest::Ingested {
        members: 1,
        rows_read: stored + skips.total(),
        bars_stored: stored,
        bars_committed: stored,
        counted: 1,
        decoder_skips: skips,
        ..pull::ingest::Ingested::default()
    }
}

/// Every printed "a = b + c + d (+ e)" on the page must add up.
fn printed_balance_line(page: &str) -> &str {
    let at = page
        .find("<th>Balances</th><td>")
        .expect("the receipt has a Balances row");
    let rest = &page[at + "<th>Balances</th><td>".len()..];
    &rest[..rest.find("</td>").expect("the row closes")]
}

/// The integers printed in a balance line, in order.
fn numbers(line: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut digits = String::new();
    for c in line.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if !digits.is_empty() {
            out.push(digits.parse().expect("digits"));
            digits.clear();
        }
    }
    if !digits.is_empty() {
        out.push(digits.parse().expect("digits"));
    }
    out
}

/// **THE PRINTED EQUATION ADDS UP AND NAMES THE SKIPPED CANDLES.** For every
/// split of a fixed seed's skip counts across the four reasons, a balanced run
/// prints `read = stored + folded + dropped + skipped`, and the sum on the
/// right is the number on the left.
#[test]
fn a_balanced_receipt_prints_an_equation_that_adds_up_including_decoder_skips() {
    let journal = audit::Journal::at(&scratch("equation"));
    let mut seed = 0x5EED_0000_0000_3180_u64;
    for case in 0..64_u32 {
        seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let skips = pull::fetch::DecodeSkips {
            null_price: usize::try_from(z & 7).expect("small"),
            negative_volume: usize::try_from((z >> 3) & 7).expect("small"),
            negative_open_interest: usize::try_from((z >> 6) & 7).expect("small"),
            impossible_ohlc: usize::try_from((z >> 9) & 7).expect("small"),
        };
        let stored = usize::try_from((z >> 12) & 1023).expect("small") + 1;
        let done = run(stored, skips);
        assert!(done.balances(), "case {case}: the fixture balances");
        let (_, page) = landed_answer(
            &done,
            window(),
            std::time::SystemTime::UNIX_EPOCH,
            "fixture",
            &journal,
            Vec::new(),
            1,
        );
        let line = printed_balance_line(&page);
        assert!(line.starts_with("yes"), "case {case}: {line}");
        let n = numbers(line);
        let (read, parts) = n.split_first().expect("a read count");
        assert_eq!(
            *read,
            parts.iter().sum::<usize>(),
            "case {case}: the receipt prints a false equation: {line}"
        );
        if skips.total() > 0 {
            assert!(
                line.contains("skipped"),
                "case {case}: {} skipped candle(s) are not named: {line}",
                skips.total()
            );
        }
    }
}

/// **THE JOURNAL NOTE DOES NOT CLAIM THREE PLACES FOR A ROW IN A FOURTH.** A
/// balanced run whose decoder skipped a candle is recorded with a note that
/// names the decoder, so `/audit` — whose fixed-stride record has no field for
/// the count — does not assert that every row was stored, folded or dropped.
#[test]
fn the_journal_note_of_a_run_with_decoder_skips_names_them() {
    let skips = pull::fetch::DecodeSkips {
        null_price: 1,
        ..pull::fetch::DecodeSkips::default()
    };
    let done = run(2, skips);
    assert!(done.balances());
    let record = audit::Record::of_run(
        audit::Scope::Spot,
        std::time::SystemTime::UNIX_EPOCH,
        1,
        "fixture",
        window(),
        &done,
    );
    assert_eq!(record.outcome, audit::Outcome::Stored);
    assert!(
        record.note.contains("skipped"),
        "the note hides the skipped candle: {}",
        record.note
    );
    assert!(record.note.contains("1 skipped"), "{}", record.note);
    assert_eq!(
        record.note_bytes as usize,
        record.note.len(),
        "the note is not cut"
    );
    // And a run with no skip keeps the sentence it always had.
    let clean = audit::Record::of_run(
        audit::Scope::Spot,
        std::time::SystemTime::UNIX_EPOCH,
        1,
        "fixture",
        window(),
        &run(2, pull::fetch::DecodeSkips::default()),
    );
    assert_eq!(
        clean.note,
        "every row accounted for: stored, folded into an open bar, or dropped"
    );
}
