#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
//! BASE-COV-LIBS (D-7325): the sort index's header and run-header refusals,
//! its words, its bounded byte helpers and its opens, each driven by the
//! input that reaches it. Every value is invented.

use super::*;

fn shape() -> Shape {
    Shape::of(YearMonth::new(2030, 5).expect("a month"), 1, 0x00C0_FFEE)
}

fn bar(sec: i64, close: i64) -> Bar {
    Bar {
        ts_micros: shape().origin_micros + sec * 1_000_000,
        open: close + 3,
        high: close + 7,
        low: close - 5,
        close,
        volume: 4,
        open_interest: 9,
    }
}

struct Disk(PathBuf);

impl Disk {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-srt-cover-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        Self(dir)
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn resealed_header(mut bytes: [u8; 64]) -> [u8; 64] {
    let crc = crc32c(&bytes[..60]);
    bytes[60..].copy_from_slice(&crc.to_le_bytes());
    bytes
}

fn resealed_run(mut bytes: [u8; 512]) -> [u8; 512] {
    let crc = crc32c(&bytes[..508]);
    bytes[508..].copy_from_slice(&crc.to_le_bytes());
    bytes
}

fn header_of(rows: i64) -> [u8; 512] {
    let bars: Vec<Bar> = (0..rows).map(|i| bar(i, 100 + i)).collect();
    let run = build_run(&shape(), 0, &bars).expect("built");
    let mut header = [0u8; 512];
    header.copy_from_slice(&run[..512]);
    header
}

#[test]
fn every_column_has_its_word_and_its_place() {
    let words: Vec<&str> = Column::ALL.iter().map(|column| column.as_str()).collect();
    assert_eq!(words, ["o", "h", "l", "c", "v", "oi"]);
    for (at, column) in Column::ALL.into_iter().enumerate() {
        assert_eq!(column.index(), at, "{}", column.as_str());
    }
    let origin = shape().origin_micros;
    assert_eq!(Entry { v: 1, sec: 2 }.ts(origin), origin + 2_000_000);
    assert_eq!(Entry { v: 1, sec: 0 }.ts(origin), origin);
    assert_eq!(
        Entry {
            v: 1,
            sec: u32::MAX
        }
        .ts(i64::MAX),
        i64::MAX
    );
    assert_eq!(
        Entry {
            v: 1,
            sec: u32::MAX
        }
        .ts(i64::MIN),
        i64::MIN + i64::from(u32::MAX) * 1_000_000
    );
}

#[test]
fn a_file_header_names_the_first_thing_that_disagrees() {
    let good = shape().encode();
    assert_eq!(shape().check(&good), Ok(()));
    let mut unsealed = good;
    unsealed[30] ^= 1;
    assert_eq!(
        shape().check(&unsealed),
        Err(Why::Header("its header fails its checksum"))
    );
    let mut magic = good;
    magic[0] ^= 1;
    assert_eq!(
        shape().check(&resealed_header(magic)),
        Err(Why::Header("its magic is not BRUTEXS1"))
    );
    let mut version = good;
    version[8] = 2;
    assert_eq!(
        shape().check(&resealed_header(version)),
        Err(Why::Header("its version is not 1"))
    );
    // Magic and version both wrong at once: the magic is named first.
    let mut both = good;
    both[0] ^= 1;
    both[8] = 2;
    assert_eq!(
        shape().check(&resealed_header(both)),
        Err(Why::Header("its magic is not BRUTEXS1"))
    );
}

#[test]
fn every_run_header_refusal_names_its_rule() {
    let good = header_of(3);
    assert!(decode_run_header(&good, 64, 0).is_ok());
    let column = |index: u64| usize::try_from(COLUMNS_AT + index * COLUMN_LEN).unwrap();
    let word = |bytes: &mut [u8; 512], at: usize, value: u64| {
        bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
    };
    let read =
        |bytes: &[u8; 512], at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    let refusal = |bytes: [u8; 512]| decode_run_header(&resealed_run(bytes), 64, 0);
    let named = |why: &'static str| Err(Why::Run { offset: 64, why });

    let mut magic = good;
    magic[0] ^= 1;
    assert_eq!(refusal(magic), named("its magic is not BRUTEXSR"));

    let mut no_rows = good;
    word(&mut no_rows, 16, 0);
    assert_eq!(refusal(no_rows), named("it orders no rows"));

    let mut width = good;
    width[column(0)] = 9;
    assert_eq!(
        refusal(width),
        named("a column's entry width is neither 8 nor 12")
    );

    let mut blocks = good;
    let at = column(0) + 40;
    let held = read(&blocks, at);
    word(&mut blocks, at, held + 1);
    assert_eq!(
        refusal(blocks),
        named("a column's section is not where its run puts it")
    );

    let mut offset = good;
    let at = column(0) + 32;
    let held = read(&offset, at);
    word(&mut offset, at, held + 1);
    assert_eq!(
        refusal(offset),
        named("a column's section is not where its run puts it")
    );

    // A null in a column that cannot hold one, and more nulls than rows in
    // the one that can.
    let mut null_open = good;
    word(&mut null_open, column(0) + 8, 1);
    assert_eq!(
        refusal(null_open),
        named("a column counts nulls it cannot hold")
    );
    let mut many_nulls = good;
    word(&mut many_nulls, column(5) + 8, 4);
    assert_eq!(
        refusal(many_nulls),
        named("a column counts nulls it cannot hold")
    );

    let mut upside_down = good;
    let base = read(&upside_down, column(0) + 16);
    word(&mut upside_down, column(0) + 24, base.wrapping_sub(1));
    assert_eq!(
        refusal(upside_down),
        named("a column's largest value is below its smallest")
    );

    // Two faults at once: the first rule in the header's order is named.
    let mut two = no_rows;
    two[column(0)] = 9;
    assert_eq!(refusal(two), named("it orders no rows"));
}

#[test]
fn every_refusal_reads_as_its_own_sentence() {
    let cases = [
        (Why::TooManyRuns(9), "more than"),
        (
            Why::Block {
                start: 3,
                column: Column::Close,
                block: 2,
            },
            "block 2 of column c in the run from row 3",
        ),
        (
            Why::Fences {
                start: 4,
                column: Column::OpenInterest,
            },
            "fence table of column oi in the run from row 4",
        ),
        (
            Why::Outside {
                row: 5,
                ts_micros: -1,
            },
            "bar 5 is stamped -1, outside the month",
        ),
        (Why::Empty, "a sort run of no rows is never written"),
    ];
    for (why, said) in cases {
        assert!(why.to_string().contains(said), "{why}");
    }
}

#[test]
fn the_byte_helpers_never_reach_past_their_buffer() {
    assert_eq!(take::<4>(&[1], 0), [0; 4]);
    assert_eq!(take::<2>(&[1, 2, 3], 1), [2, 3]);
    assert_eq!(take::<2>(&[1, 2, 3], usize::MAX), [0; 2]);
    let mut out = [5u8; 2];
    put(&mut out, 1, &[1, 2]);
    assert_eq!(out, [5, 5]);
    put(&mut out, usize::MAX, &[1]);
    assert_eq!(out, [5, 5]);
    put(&mut out, 0, &[1, 2]);
    assert_eq!(out, [1, 2]);
}

#[test]
fn an_open_refuses_a_link_a_directory_a_short_file_and_a_path_under_a_file() {
    let disk = Disk::new("opens");
    let bars: Vec<Bar> = (0..300).map(|i| bar(i, 100 + i % 7)).collect();
    let mut bytes = shape().encode().to_vec();
    bytes.extend_from_slice(&build_run(&shape(), 0, &bars).expect("built"));
    let path = disk.0.join("2030-05.srt");
    std::fs::write(&path, &bytes).expect("write");
    let index = SortIndex::open(&path, &shape()).expect("opens");
    assert_eq!(index.path(), path.as_path());
    assert_eq!(index.shape(), &shape());
    let fences = index.fences(0, Column::Close).expect("fences");
    assert_eq!(fences.entries.len(), 2);
    assert_eq!(fences.weight(), 2 * std::mem::size_of::<Entry>() + 2 * 4);
    assert_eq!(
        index.block(0, Column::Close, 2, &fences),
        Err(Why::Block {
            start: 0,
            column: Column::Close,
            block: 2,
        })
    );
    assert_eq!(
        index.block(0, Column::Close, u64::MAX, &fences),
        Err(Why::Block {
            start: 0,
            column: Column::Close,
            block: u64::MAX,
        })
    );
    let walked = index.walked().clone();
    let again = SortIndex::open_walked(&path, &shape(), walked.clone()).expect("the same file");
    assert_eq!(again.walked(), &walked);
    assert_eq!(again.path(), path.as_path());

    // The file cut below what the walk read is not the file that was walked.
    let short = disk.0.join("short.srt");
    std::fs::write(&short, &bytes[..100]).expect("write");
    assert!(matches!(
        SortIndex::open_walked(&short, &shape(), walked.clone()),
        Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
    ));
    // A directory at the name is no file, both ways.
    let dir = disk.0.join("dir.srt");
    std::fs::create_dir(&dir).expect("dir");
    assert!(matches!(
        SortIndex::open_walked(&dir, &shape(), walked.clone()),
        Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
    ));
    assert!(matches!(
        SortIndex::open(&dir, &shape()),
        Err(Why::Unreadable(why)) if why.contains("is not a regular file")
    ));
    // A link at the name is never followed, even to the good file.
    let link = disk.0.join("link.srt");
    std::os::unix::fs::symlink(&path, &link).expect("link");
    assert!(matches!(
        SortIndex::open_walked(&link, &shape(), walked.clone()),
        Err(Why::Unreadable(why)) if why.contains("is a symbolic link")
    ));
    // A name under a file: the host's own refusal, named with the path.
    let under = path.join("x.srt");
    assert!(matches!(
        SortIndex::open_walked(&under, &shape(), walked.clone()),
        Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())
    ));
    assert!(matches!(
        SortIndex::open_walked(&disk.0.join("absent.srt"), &shape(), walked),
        Err(Why::Absent)
    ));
}
