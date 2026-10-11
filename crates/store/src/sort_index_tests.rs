//! The `.srt` codec, builder, walk, binding and reader, against invented
//! bars. D-3349, WP-39.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// 2030-05, a month no real bar is stamped in.
fn month() -> YearMonth {
    YearMonth::new(2030, 5).expect("a month")
}

fn shape() -> Shape {
    Shape::of(month(), 1, 0x00C0_FFEE)
}

/// Second `sec` of the month, as a stamp.
fn at(sec: i64) -> i64 {
    shape().origin_micros + sec * 1_000_000
}

/// An invented bar: every price a function of `seed`, far from any market.
fn bar(sec: i64, close: i64, volume: i64, oi: i64) -> Bar {
    Bar {
        ts_micros: at(sec),
        open: close + 3,
        high: close + 7,
        low: close - 5,
        close,
        volume,
        open_interest: oi,
    }
}

/// A run's bytes, written behind a header, as a file in a temp dir.
struct Disk {
    dir: PathBuf,
}

impl Disk {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-srt-unit-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        Self { dir }
    }

    fn write(&self, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join("2030-05.srt");
        std::fs::write(&path, bytes).expect("write");
        path
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The header followed by `runs`.
fn file_of(runs: &[Vec<u8>]) -> Vec<u8> {
    let mut out = shape().encode().to_vec();
    for run in runs {
        out.extend_from_slice(run);
    }
    out
}

/// Every entry of `column` in run `run`, read block by block.
fn entries(index: &SortIndex, run: usize, column: Column) -> Vec<Entry> {
    let fences = index.fences(run, column).expect("fences");
    let blocks = index.runs()[run].column(column).blocks;
    let mut out = Vec::new();
    for block in 0..blocks {
        out.extend(index.block(run, column, block, &fences).expect("block"));
    }
    out
}

/// **EVERY COLUMN IS ORDERED BY VALUE, THEN TIME, AND READS BACK WHOLE.**
///
/// 700 bars — three blocks of 256 — whose closes repeat, so ties exist and
/// must be broken by time; the reader's fences are each block's first entry.
#[test]
fn a_run_orders_every_column_by_value_then_time_and_reads_back() {
    let bars: Vec<Bar> = (0..700)
        .map(|i| {
            bar(
                i * 60,
                900_000 + (i * 37) % 101,
                (i * 13) % 17,
                40 + (i % 9),
            )
        })
        .collect();
    let run = build_run(&shape(), 0, &bars).expect("built");
    let disk = Disk::new("order");
    let path = disk.write(&file_of(&[run]));
    let index = SortIndex::open(&path, &shape()).expect("opens");
    assert_eq!(index.runs().len(), 1);
    assert_eq!(index.coverage(), 700);
    assert!(index.torn().is_none());
    for column in Column::ALL {
        let got = entries(&index, 0, column);
        let mut want: Vec<Entry> = bars
            .iter()
            .map(|b| Entry {
                v: column.of(b),
                sec: u32::try_from((b.ts_micros - shape().origin_micros) / 1_000_000).unwrap(),
            })
            .collect();
        want.sort_by_key(|e| (e.v, e.sec));
        assert_eq!(got, want, "column {}", column.as_str());
        let fences = index.fences(0, column).expect("fences");
        assert_eq!(fences.entries.len(), 3);
        for (j, fence) in fences.entries.iter().enumerate() {
            assert_eq!(*fence, want[j * 256], "fence {j} of {}", column.as_str());
        }
        let info = index.runs()[0].column(column);
        assert_eq!(info.base, want[0].v);
        assert_eq!(info.max, want[699].v);
        assert_eq!(info.width, NARROW);
    }
}

/// Open interest's null is no number: the nulls come first, in time order,
/// and the values after them are ordered as values.
#[test]
fn open_interest_nulls_lead_the_column_in_time_order() {
    let bars = vec![
        bar(0, 100_000, 5, OI_NULL),
        bar(60, 100_100, 5, 70),
        bar(120, 100_200, 5, OI_NULL),
        bar(180, 100_300, 5, 10),
        bar(240, 100_400, 5, 70),
    ];
    let run = build_run(&shape(), 0, &bars).expect("built");
    let disk = Disk::new("nulls");
    let index = SortIndex::open(&disk.write(&file_of(&[run])), &shape()).expect("opens");
    let got = entries(&index, 0, Column::OpenInterest);
    assert_eq!(index.runs()[0].column(Column::OpenInterest).nulls, 2);
    assert_eq!(
        got,
        vec![
            Entry { v: OI_NULL, sec: 0 },
            Entry {
                v: OI_NULL,
                sec: 120
            },
            Entry { v: 10, sec: 180 },
            Entry { v: 70, sec: 60 },
            Entry { v: 70, sec: 240 },
        ]
    );
    // No other column counts a null, whatever its value.
    assert_eq!(index.runs()[0].column(Column::Volume).nulls, 0);
}

/// A column whose span does not fit four bytes is written wide, and reads
/// back the same.
#[test]
fn a_column_wider_than_four_bytes_is_written_wide() {
    let bars = vec![
        bar(0, 100_000, 0, 1),
        bar(1, 100_000, 9_000_000_000, 1),
        bar(2, 100_000, 3, 1),
    ];
    let run = build_run(&shape(), 0, &bars).expect("built");
    let disk = Disk::new("wide");
    let index = SortIndex::open(&disk.write(&file_of(&[run])), &shape()).expect("opens");
    assert_eq!(index.runs()[0].column(Column::Volume).width, WIDE);
    assert_eq!(index.runs()[0].column(Column::Close).width, NARROW);
    let got: Vec<i64> = entries(&index, 0, Column::Volume)
        .iter()
        .map(|e| e.v)
        .collect();
    assert_eq!(got, vec![0, 3, 9_000_000_000]);
}

/// The extremes keep the EARLIEST bar of a tie — the window's rule.
#[test]
fn a_runs_extremes_keep_the_earliest_bar_of_a_tie() {
    let bars = vec![
        bar(0, 100, 4, 1),
        bar(10, 300, 9, 1),
        bar(20, 300, 9, 1),
        bar(30, 50, 1, 1),
        bar(40, 50, 1, 1),
    ];
    let run = build_run(&shape(), 0, &bars).expect("built");
    let disk = Disk::new("extremes");
    let index = SortIndex::open(&disk.write(&file_of(&[run])), &shape()).expect("opens");
    let x = index.runs()[0].extremes;
    assert_eq!(x.close_max, Extreme { v: 300, ts: at(10) });
    assert_eq!(x.close_min, Extreme { v: 50, ts: at(30) });
    assert_eq!(x.high_max, Extreme { v: 307, ts: at(10) });
    assert_eq!(x.low_min, Extreme { v: 45, ts: at(30) });
    assert_eq!(x.range_max, Extreme { v: 12, ts: at(0) });
    assert_eq!(x.volume_max, Extreme { v: 9, ts: at(10) });
}

/// Runs chain: each starts where the one before ended, and together they
/// cover a prefix. A run that does not start there is not a run.
#[test]
fn runs_chain_from_row_zero_and_a_gap_ends_the_walk() {
    let bars: Vec<Bar> = (0..10).map(|i| bar(i, 1_000 + i, 1, 1)).collect();
    let first = build_run(&shape(), 0, &bars[..4]).expect("first");
    let second = build_run(&shape(), 4, &bars[4..]).expect("second");
    let disk = Disk::new("chain");
    let index =
        SortIndex::open(&disk.write(&file_of(&[first.clone(), second])), &shape()).expect("opens");
    assert_eq!(index.runs().len(), 2);
    assert_eq!(index.coverage(), 10);
    assert_eq!((index.runs()[1].start, index.runs()[1].end), (4, 10));

    // A run built for rows 5.. after one that ended at 4 is refused, named.
    let skipped = build_run(&shape(), 5, &bars[5..]).expect("built");
    let index = SortIndex::open(&disk.write(&file_of(&[first, skipped])), &shape()).expect("opens");
    assert_eq!(index.coverage(), 4);
    let (offset, why) = index.torn().expect("the gap is named");
    assert_eq!(*offset, HEADER_LEN + index.runs()[0].len);
    assert!(why.contains("does not start where"), "{why}");
}

/// A run cut short — a writer that died — is never read as a run, and the
/// runs before it still serve.
#[test]
fn a_torn_run_is_ignored_and_named() {
    let bars: Vec<Bar> = (0..300).map(|i| bar(i, 5_000 + i, 1, 1)).collect();
    let first = build_run(&shape(), 0, &bars[..100]).expect("first");
    let second = build_run(&shape(), 100, &bars[100..]).expect("second");
    let mut bytes = file_of(&[first, second]);
    bytes.truncate(bytes.len() - 7);
    let disk = Disk::new("torn");
    let index = SortIndex::open(&disk.write(&bytes), &shape()).expect("opens");
    assert_eq!(index.coverage(), 100);
    let (_, why) = index.torn().expect("named");
    assert!(why.contains("bytes"), "{why}");
}

/// **A RUN WHOSE HEADER LANDED AND WHOSE BODY DID NOT IS TORN, NOT SERVED.**
/// A crash while appending can leave the last run's header whole over entries
/// that are not there; the walk checks the LAST appended run's body against
/// its header and drops it, so the rows it claimed are read by every reader
/// and appended again by the next writer. An earlier run, synced before the
/// next began, and a whole image, synced before its rename, are not read
/// whole on a walk: a flipped byte in either is a block refusal when read.
#[test]
fn the_last_appended_runs_body_is_checked_on_every_walk() {
    let bars: Vec<Bar> = (0..900).map(|i| bar(i, 3_000 + (i % 77), 1, 1)).collect();
    let first = build_run(&shape(), 0, &bars[..400]).expect("first");
    let second = build_run(&shape(), 400, &bars[400..]).expect("second");
    let disk = Disk::new("body");
    let clean = file_of(&[first.clone(), second.clone()]);
    let index = SortIndex::open(&disk.write(&clean), &shape()).expect("opens");
    assert_eq!(index.coverage(), 900);
    assert!(index.torn().is_none());
    assert_eq!(index.runs()[1].kind, RunKind::Appended);

    // THE SECOND RUN'S LAST BYTES NEVER LANDED: zeros where entries were.
    let mut torn = clean.clone();
    let len = torn.len();
    for byte in &mut torn[len - 300..] {
        *byte = 0;
    }
    let index = SortIndex::open(&disk.write(&torn), &shape()).expect("opens");
    assert_eq!(index.coverage(), 400, "the torn run is not a run");
    let (offset, why) = index.torn().expect("named");
    assert_eq!(*offset, HEADER_LEN + first.len() as u64);
    assert_eq!(
        index.walked().end,
        *offset,
        "the next run is appended there"
    );
    assert!(why.contains("body fails its checksum"), "{why}");

    // AN EARLIER RUN is not read whole on a walk: its damage is a block
    // refusal when read.
    let mut early = clean.clone();
    let info = *index.runs()[0].column(Column::Close);
    let at = usize::try_from(HEADER_LEN + info.entries_at()).unwrap() + 5;
    early[at] ^= 0x08;
    let index = SortIndex::open(&disk.write(&early), &shape()).expect("opens");
    assert_eq!(index.coverage(), 900);
    let fences = index.fences(0, Column::Close).expect("the table is whole");
    assert!(matches!(
        index.block(0, Column::Close, 0, &fences),
        Err(Why::Block { block: 0, .. })
    ));

    // A WHOLE IMAGE is synced before it is renamed into place: not read
    // whole on a walk either.
    let whole = build_run_as(&shape(), 0, &bars, RunKind::Whole).expect("whole");
    let mut image = file_of(&[whole]);
    let len = image.len();
    image[len - 3] ^= 0x01;
    let index = SortIndex::open(&disk.write(&image), &shape()).expect("opens");
    assert_eq!(index.runs()[0].kind, RunKind::Whole);
    assert_eq!(index.coverage(), 900);
    assert!(index.torn().is_none());

    // A kind no build writes, or a reserved byte set, is not a run.
    for (at, value) in [(436_usize, 2_u8), (480, 1)] {
        let mut odd = second.clone();
        odd[at] = value;
        let crc = crc32c(&odd[..508]);
        odd[508..512].copy_from_slice(&crc.to_le_bytes());
        let index =
            SortIndex::open(&disk.write(&file_of(&[first.clone(), odd])), &shape()).expect("opens");
        assert_eq!(index.coverage(), 400);
        let (_, why) = index.torn().expect("named");
        assert!(why.contains("kind") || why.contains("reserved"), "{why}");
    }
}

/// A flipped byte in a block fails that block's checksum; a flipped byte in
/// a fence table fails the table; a flipped byte in a run header ends the
/// walk there.
#[test]
fn a_damaged_block_fence_table_or_run_header_is_refused_by_name() {
    let bars: Vec<Bar> = (0..600).map(|i| bar(i, 7_000 + (i % 50), 1, 1)).collect();
    // A WHOLE IMAGE: a walk does not read its body, so the damage below is
    // met where a reader meets it (an appended last run's would be torn).
    let run = build_run_as(&shape(), 0, &bars, RunKind::Whole).expect("built");
    let clean = file_of(&[run]);
    let disk = Disk::new("damage");
    let index = SortIndex::open(&disk.write(&clean), &shape()).expect("opens");
    let info = *index.runs()[0].column(Column::Close);
    let base = usize::try_from(HEADER_LEN).unwrap();

    let mut block = clean.clone();
    let in_block_one = base + usize::try_from(info.entries_at()).unwrap() + 256 * 8 + 3;
    block[in_block_one] ^= 0x40;
    let index = SortIndex::open(&disk.write(&block), &shape()).expect("opens");
    let fences = index.fences(0, Column::Close).expect("the table is whole");
    assert!(index.block(0, Column::Close, 0, &fences).is_ok());
    assert_eq!(
        index.block(0, Column::Close, 1, &fences),
        Err(Why::Block {
            start: 0,
            column: Column::Close,
            block: 1
        })
    );

    let mut table = clean.clone();
    table[base + usize::try_from(info.offset).unwrap() + 1] ^= 0x01;
    let index = SortIndex::open(&disk.write(&table), &shape()).expect("opens");
    assert_eq!(
        index.fences(0, Column::Close),
        Err(Why::Fences {
            start: 0,
            column: Column::Close
        })
    );

    let mut header = clean;
    header[base + 20] ^= 0x01;
    let index = SortIndex::open(&disk.write(&header), &shape()).expect("opens");
    assert_eq!(index.coverage(), 0);
    assert!(index.torn().expect("named").1.contains("checksum"));
}

/// **A RUN A DAY BOUND CUTS IS READ FROM ITS OWN ENTRIES.** `entries_where`
/// keeps exactly the entries a filter keeps, in the column's order, nulls
/// counted apart; across a read boundary of 1,024 blocks too; and a damaged
/// block anywhere in the column refuses the whole read by name.
#[test]
fn entries_where_keeps_the_filtered_entries_in_column_order() {
    // 264,744 bars, one a second: 1,035 blocks, so the read crosses its
    // 1,024-block boundary; open interest null on every fifth.
    let bars: Vec<Bar> = (0..264_744)
        .map(|i| {
            bar(
                i,
                600_000 + (i * 29) % 97,
                (i * 11) % 13,
                if i % 5 == 0 { OI_NULL } else { 70 + (i % 4) },
            )
        })
        .collect();
    let run = build_run_as(&shape(), 0, &bars, RunKind::Whole).expect("built");
    let clean = file_of(&[run]);
    let disk = Disk::new("where");
    let index = SortIndex::open(&disk.write(&clean), &shape()).expect("opens");
    let (lo, hi) = (12_000_u32, 263_030_u32);
    let keep = |e: Entry| e.sec >= lo && e.sec < hi;
    for column in Column::ALL {
        let fences = index.fences(0, column).expect("fences");
        let (got, nulls) = index
            .entries_where(0, column, &fences, keep)
            .expect("reads");
        let all = entries(&index, 0, column);
        let want: Vec<Entry> = all.iter().copied().filter(|e| keep(*e)).collect();
        assert_eq!(got, want, "column {}", column.as_str());
        let null_count = all
            .iter()
            .take(usize::try_from(index.runs()[0].column(column).nulls).unwrap())
            .filter(|e| keep(**e))
            .count();
        assert_eq!(nulls, null_count as u64, "column {}", column.as_str());
        if column == Column::OpenInterest {
            assert!(nulls > 0, "the bound keeps some nulls");
        }
        // Everything, through the same door.
        let (whole, _) = index
            .entries_where(0, column, &fences, |_| true)
            .expect("reads");
        assert_eq!(whole, all);
    }
    // One flipped byte in the tenth block refuses the read by name.
    let info = *index.runs()[0].column(Column::Close);
    let mut damaged = clean;
    let base = usize::try_from(HEADER_LEN).unwrap();
    damaged[base + usize::try_from(info.entries_at()).unwrap() + 9 * 256 * 8 + 5] ^= 0x10;
    let index = SortIndex::open(&disk.write(&damaged), &shape()).expect("opens");
    let fences = index.fences(0, Column::Close).expect("the table is whole");
    assert_eq!(
        index.entries_where(0, Column::Close, &fences, |_| true),
        Err(Why::Block {
            start: 0,
            column: Column::Close,
            block: 9
        })
    );
}

/// A file for another month, rung or symbol is refused before a run is read.
#[test]
fn a_file_for_another_month_rung_or_symbol_is_refused() {
    let disk = Disk::new("shape");
    let path = disk.write(&file_of(&[]));
    assert!(SortIndex::open(&path, &shape()).is_ok());
    let other = Shape::of(month(), 60, 0x00C0_FFEE);
    assert!(matches!(
        SortIndex::open(&path, &other),
        Err(Why::Header(_))
    ));
    let symbol = Shape::of(month(), 1, 7);
    assert!(matches!(
        SortIndex::open(&path, &symbol),
        Err(Why::Header(_))
    ));
    assert_eq!(
        SortIndex::open(&disk.dir.join("absent.srt"), &shape()).err(),
        Some(Why::Absent)
    );
    let short = disk.write(&[1, 2, 3]);
    assert!(matches!(
        SortIndex::open(&short, &shape()),
        Err(Why::Header(_))
    ));
}

/// A stamp that is not a whole second, or lies outside the month, cannot be
/// named by an entry; a run of no bars is never written.
#[test]
fn a_bar_no_entry_can_name_refuses_the_run() {
    let mut odd = bar(5, 100, 1, 1);
    odd.ts_micros += 1;
    assert_eq!(
        build_run(&shape(), 7, &[bar(1, 100, 1, 1), odd]),
        Err(Why::NotWholeSecond {
            row: 8,
            ts_micros: odd.ts_micros
        })
    );
    let mut early = bar(0, 100, 1, 1);
    early.ts_micros -= 1_000_000;
    assert!(matches!(
        build_run(&shape(), 0, &[early]),
        Err(Why::Outside { row: 0, .. })
    ));
    assert_eq!(build_run(&shape(), 0, &[]), Err(Why::Empty));
}

/// The binding: a run past the committed count, or one whose first or last
/// stamp is not the month's stamp at that row, does not describe the bars.
#[test]
fn runs_bind_to_the_bars_by_their_end_stamps() {
    let bars: Vec<Bar> = (0..10).map(|i| bar(i * 2, 1_000 + i, 1, 1)).collect();
    let first = build_run(&shape(), 0, &bars[..6]).expect("first");
    let second = build_run(&shape(), 6, &bars[6..]).expect("second");
    let disk = Disk::new("bind");
    let index = SortIndex::open(&disk.write(&file_of(&[first, second])), &shape()).expect("opens");
    let mut reads = 0;
    let ts_at = |row: u64| {
        reads += 1;
        Ok(bars[usize::try_from(row).unwrap()].ts_micros)
    };
    assert_eq!(
        bind(
            index.runs(),
            10,
            bars[0].ts_micros,
            bars[9].ts_micros,
            ts_at
        ),
        Ok(10)
    );
    // Two boundaries inside the month, read; the two ends come off the header.
    assert_eq!(reads, 2);
    assert!(matches!(
        bind(
            index.runs(),
            9,
            bars[0].ts_micros,
            bars[8].ts_micros,
            |row| Ok(bars[usize::try_from(row).unwrap()].ts_micros)
        ),
        Err(Why::Stale(_))
    ));
    let moved = |row: u64| Ok(bars[usize::try_from(row).unwrap()].ts_micros + 1_000_000);
    assert!(matches!(
        bind(index.runs(), 12, bars[0].ts_micros, at(999), moved),
        Err(Why::Stale(_))
    ));
}

/// A run's header is checked field by field: the walk stops at the first
/// run whose columns do not tile its length.
#[test]
fn a_run_header_that_lies_about_its_length_is_refused() {
    let bars: Vec<Bar> = (0..3).map(|i| bar(i, 10 + i, 1, 1)).collect();
    let run = build_run(&shape(), 0, &bars).expect("built");
    let mut header = [0u8; 512];
    header.copy_from_slice(&run[..512]);
    assert!(decode_run_header(&header, 64, 0).is_ok());
    assert!(matches!(
        decode_run_header(&header, 64, 1),
        Err(Why::Run { offset: 64, .. })
    ));
    let mut lying = header;
    lying[40] ^= 0x01;
    let crc = crc32c(&lying[..508]);
    lying[508..].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(
        decode_run_header(&lying, 64, 0),
        Err(Why::Run {
            offset: 64,
            why: "its columns do not fill its stated length"
        })
    );
}

/// More runs than a reader walks is a refusal, named — never a silent cut.
#[test]
fn more_runs_than_a_reader_walks_is_refused() {
    let mut runs = Vec::new();
    for i in 0..=u64::try_from(MAX_RUNS).unwrap() {
        let b = bar(i64::try_from(i).unwrap(), 50, 1, 1);
        runs.push(build_run(&shape(), i, &[b]).expect("built"));
    }
    let disk = Disk::new("many");
    assert_eq!(
        SortIndex::open(&disk.write(&file_of(&runs)), &shape()).err(),
        Some(Why::TooManyRuns(MAX_RUNS + 1))
    );
}
