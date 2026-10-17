//! `store::verdict` through real files: IA-VD-01 to IA-VD-07, IA-VD-09,
//! IA-VD-10, IA-VD-11 (the record half), IA-VD-12, IA-VD-13, IA-VD-14 and
//! IA-VD-16 of the ingest-automation design (D-4808, D-4809, D-4844, D-4865,
//! D-4879).
//!
//! Every test builds its own store root under the temporary directory and
//! writes invented verdicts: every key is a synthetic `ZZ`-prefixed name no
//! exchange lists, and every count is arbitrary. No vendor row is read or
//! copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::vendor::Vendor;
use store::path::Timeframe;
use store::provenance::Day;
use store::verdict::{
    self, Carry, Check, Code, Committed, DETAIL_CAP, DETAIL_NAME, DayBlock, DayState, Detail,
    Entry, Facts, Feed, Found, Generation, INDEX_LEN, JournalOutcome, KEY_LEN, Key, MAX_LINK_TRIES,
    Mode, MonthNow, NONE, NoRecord, Now, ORPHAN_SCAN, RECORD_STRIDE, Reader, Series, SourceHeld,
    Stale, Unit, Unknown, V3Manifest, VerdictError, flags,
};

const FORMAT_DOC: &str = include_str!("../../../docs/02-store-format.md");

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-vd-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn series(&self) -> Series<'_> {
        Series {
            root: &self.0,
            feed: Feed::Gdfl,
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            timeframe: Timeframe::SECOND_1,
        }
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn day() -> Day {
    Day::new(2031, 3, 4).unwrap()
}

/// A synthetic instrument key, about as long as a real option key.
fn name(i: usize) -> String {
    format!("NSE-FNO-ZZSYN{:02}-{:07}-CE-{:09}", i % 37, i, i * 7 + 3)
}

fn entry(name: &str, code: Code) -> Entry {
    Entry {
        key: Key::instrument(name).unwrap(),
        facts: Facts::of(code),
        details: Vec::new(),
    }
}

fn generation(run: u64, entries: Vec<Entry>) -> Generation {
    Generation {
        day: day(),
        run,
        written_at: 1_900_000_000,
        verifier: 3,
        mode: Mode::Cold,
        rule_set: 2,
        origin: [0xAB; 16],
        day_block: DayBlock::of(Code::Match),
        day_details: Vec::new(),
        entries,
        carry: Carry::WholeDay,
    }
}

/// A code for the i-th synthetic key: mostly matches, with every other code
/// sprinkled through.
fn code_of(i: usize) -> Code {
    if i.is_multiple_of(5) {
        Code::ALL[i / 5 % Code::ALL.len()]
    } else {
        Code::Match
    }
}

/// A day of `n` synthetic records, the i-th carrying `source_units = i`.
fn day_of(n: usize) -> Vec<Entry> {
    (0..n)
        .map(|i| {
            let mut e = entry(&name(i), code_of(i));
            e.facts.source_units = u32::try_from(i).unwrap();
            e
        })
        .collect()
}

fn found(answer: &verdict::Answer) -> &verdict::Record {
    match &answer.found {
        Found::Record { record, .. } => record,
        Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
    }
}

fn home(hash: u64, capacity: u32) -> u64 {
    hash & u64::from(capacity - 1)
}

/// `Key::stem(text)` made on a worker and given 20 seconds, so a cut that
/// never steps back to a character fails the test instead of hanging the
/// binary.
fn stem_within(text: &str) -> Result<Key, VerdictError> {
    let (send, answer) = std::sync::mpsc::channel();
    let text = text.to_owned();
    std::thread::spawn(move || {
        let _receiver_gone = send.send(Key::stem(&text));
    });
    answer
        .recv_timeout(Duration::from_secs(20))
        .expect("the stem returned")
}

/// IA-VD-01 — a verdict is found by one bounded probe at every day size: at
/// 1x (2,758 records), 10x (27,580, past the largest day measured, 26,820)
/// and 50x, a present key costs 5 reads and 2 opens cold (0 opens warm), an
/// absent key at most 5 reads, and keys that share a home slot are each
/// found. At 1x an absent key whose home is a crowded slot is also answered
/// in at most 5 reads.
///
/// 100x (275,800 records) is past what format version 1 holds: no table of
/// at most 2^20 slots keeps every key within 8 slots of home at that count,
/// so it is refused `table_too_large` by name and nothing is written. That
/// limit is pinned here rather than hidden.
#[test]
fn a_verdict_is_found_by_one_bounded_probe_at_every_day_size() {
    let root = Root::new();
    let refused = verdict::commit(&root.series(), &generation(40, day_of(275_800)));
    match refused {
        Err(VerdictError::TableTooLarge { records }) => assert_eq!(records, 275_800),
        other => panic!("100x must be refused table_too_large, got {other:?}"),
    }
    assert_eq!(
        fs::read_dir(&root.0).unwrap().count(),
        0,
        "a refused day writes nothing: no directory, lock, index or image"
    );
    assert_eq!(
        Reader::new()
            .lookup(&root.series(), day(), &Key::instrument(&name(0)).unwrap())
            .unwrap()
            .answer,
        None,
        "a refused day writes nothing"
    );
    for n in [2_758usize, 27_580, 137_900] {
        let root = Root::new();
        let series = root.series();
        let done = verdict::commit(&series, &generation(41, day_of(n))).unwrap();
        assert_eq!(done.records, u32::try_from(n).unwrap());
        // Half the measured ceiling is named while the day still fits
        // (RA-261): 1x and 10x are under it, 50x is at the ceiling itself.
        assert_eq!(
            done.near_ceiling(),
            n >= 68_950,
            "n={n}: the watch is half of the {} records placed",
            verdict::RECORDS_PLACED
        );
        assert_eq!(done.staging_cleared, 0, "n={n}: no leftover to clear");

        // A present key, cold and then warm.
        let probe_at = [0, n / 2, n - 1];
        for i in probe_at {
            let key = Key::instrument(&name(i)).unwrap();
            let mut cold = Reader::new();
            let got = cold.lookup(&series, day(), &key).unwrap();
            assert_eq!(
                got.cost.opens, 2,
                "n={n}: a cold lookup opens the index and the image"
            );
            assert_eq!(
                got.cost.reads, 5,
                "n={n}: 2 slots, the prefix, the window, the record"
            );
            let answer = got.answer.unwrap();
            let record = found(&answer);
            assert_eq!(record.facts.source_units, u32::try_from(i).unwrap());
            assert_eq!(record.facts.code, code_of(i));
            assert_eq!(record.run, 41);
            let warm = cold.lookup(&series, day(), &key).unwrap();
            assert_eq!(warm.cost.opens, 0, "n={n}: a warm lookup opens nothing");
            assert_eq!(warm.cost.reads, 5);
        }

        // An absent key: its own answer from the day block's state, in at
        // most 5 reads.
        let absent = Key::instrument("NSE-FNO-ZZABSENT-1").unwrap();
        let mut reader = Reader::new();
        let got = reader.lookup(&series, day(), &absent).unwrap();
        assert!(
            got.cost.reads <= 5 && got.cost.opens <= 2,
            "n={n}: {:?}",
            got.cost
        );
        let answer = got.answer.unwrap();
        assert_eq!(answer.found, Found::NoRecord(NoRecord::NotInDay));
        assert_eq!(answer.code(), None, "no_record is no code");
        let capacity = answer.header.capacity;

        // Colliding keys: every pair of records sharing a home slot.
        let mut by_home: HashMap<u64, Vec<usize>> = HashMap::with_capacity(n);
        for i in 0..n {
            by_home
                .entry(home(verdict::key_hash(name(i).as_bytes()), capacity))
                .or_default()
                .push(i);
        }
        let crowded: Vec<&Vec<usize>> = by_home.values().filter(|v| v.len() >= 3).take(8).collect();
        assert!(
            !crowded.is_empty(),
            "n={n}: a fixture this size has crowded home slots"
        );
        for group in &crowded {
            for i in group.iter().copied() {
                let got = reader
                    .lookup(&series, day(), &Key::instrument(&name(i)).unwrap())
                    .unwrap();
                assert!(got.cost.reads <= 5, "n={n}: {:?}", got.cost);
                assert_eq!(
                    found(got.answer.as_ref().unwrap()).facts.source_units,
                    u32::try_from(i).unwrap()
                );
            }
        }

        // At 1x: an absent key whose home is the most crowded slot.
        if n == 2_758 {
            let (&slot, _) = by_home.iter().max_by_key(|(_, v)| v.len()).unwrap();
            let absent = (0..200_000)
                .map(|j| format!("NSE-FNO-ZZABSENT-{j}"))
                .find(|k| home(verdict::key_hash(k.as_bytes()), capacity) == slot)
                .expect("an absent key homed on the crowded slot");
            let got = reader
                .lookup(&series, day(), &Key::instrument(&absent).unwrap())
                .unwrap();
            assert!(got.cost.reads <= 5);
            assert_eq!(
                got.answer.unwrap().found,
                Found::NoRecord(NoRecord::NotInDay)
            );
        }
        assert_eq!(
            verdict::check_trailer(&done.path).unwrap().records,
            done.records
        );
    }
}

/// IA-VD-02 — re-verification is a new generation; the old image stays
/// byte for byte and the index points at the new one.
#[test]
fn a_reverified_day_is_a_new_generation_and_the_old_is_kept() {
    let root = Root::new();
    let series = root.series();
    let first = verdict::commit(&series, &generation(51, day_of(40))).unwrap();
    assert_eq!((first.generation, first.previous), (1, 0));
    let before = fs::read(&first.path).unwrap();

    let mut again = day_of(40);
    for e in &mut again {
        e.facts.code = Code::ValueMismatch;
    }
    let second = verdict::commit(&series, &generation(52, again)).unwrap();
    assert_eq!((second.generation, second.previous), (2, 1));
    assert_ne!(first.path, second.path);
    assert_eq!(
        fs::read(&first.path).unwrap(),
        before,
        "generation 1 is never rewritten"
    );

    let mut reader = Reader::new();
    let answer = reader
        .lookup(&series, day(), &Key::instrument(&name(7)).unwrap())
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(answer.header.generation, 2);
    assert_eq!(answer.header.previous, 1);
    assert_eq!(answer.code(), Some(Code::ValueMismatch));
    let year = reader.year(&series, 2031).unwrap().unwrap();
    let entry = year.entry(day());
    assert_eq!(entry.generation, 2);
    assert_eq!(entry.worst_code(), Some(Code::ValueMismatch));
    assert_eq!(entry.mismatches, 40);
    assert_eq!(entry.records, 40);
    assert_eq!(entry.verifier, 3);
    assert_eq!(year.commit, 2);
}

/// IA-VD-03 — a torn year-index slot falls back to the other; two torn
/// slots, or a short file, are refused by name.
#[test]
fn a_torn_year_index_slot_falls_back_to_the_other() {
    let root = Root::new();
    let series = root.series();
    verdict::commit(
        &series,
        &generation(61, vec![entry("NSE-FNO-ZZTORN-A", Code::Match)]),
    )
    .unwrap();
    verdict::commit(
        &series,
        &generation(62, vec![entry("NSE-FNO-ZZTORN-A", Code::MissingInStore)]),
    )
    .unwrap();
    let key = Key::instrument("NSE-FNO-ZZTORN-A").unwrap();
    let index = series.year_index(2031);
    let mut reader = Reader::new();
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_eq!((year.commit, year.slot), (2, 0), "commit 2 is in slot 0");

    // Tear slot 0: one flipped byte inside its day entries.
    let mut bytes = fs::read(&index).unwrap();
    bytes[24 + 24 * 62] ^= 0x40;
    fs::write(&index, &bytes).unwrap();
    let mut reader = Reader::new();
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(
        answer.header.generation, 1,
        "slot 1 still commits generation 1"
    );
    assert_eq!(answer.code(), Some(Code::Match));
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_eq!((year.commit, year.slot), (1, 1));

    // Tear slot 1 as well: nothing valid is left, and it is said.
    bytes[16_384 + 100] ^= 0x01;
    fs::write(&index, &bytes).unwrap();
    let mut reader = Reader::new();
    let refused = reader.lookup(&series, day(), &key).unwrap_err();
    assert_eq!(refused.word(), "index_unreadable");

    // A short index is refused, never read as "never verified".
    fs::write(&index, &bytes[..INDEX_LEN - 1]).unwrap();
    let mut reader = Reader::new();
    assert_eq!(
        reader.lookup(&series, day(), &key).unwrap_err().word(),
        "index_short"
    );
    let refused = verdict::commit(
        &series,
        &generation(63, vec![entry("NSE-FNO-ZZTORN-A", Code::Match)]),
    );
    assert_eq!(refused.unwrap_err().word(), "index_short");
}

/// IA-VD-04 — an instrument key longer than the record is refused, never
/// cut; a raw stem over 64 bytes is cut at a character boundary, carries
/// `name_cut`, and is kept whole in the detail section.
#[test]
fn a_long_key_is_refused_and_a_long_stem_is_kept_whole() {
    let fits = "Z".repeat(KEY_LEN);
    assert_eq!(Key::instrument(&fits).unwrap().text(), fits);
    let long = format!("{fits}Z");
    match Key::instrument(&long) {
        Err(VerdictError::KeyTooLong { len }) => assert_eq!(len, KEY_LEN + 1),
        other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
    }
    assert_eq!(Key::instrument("").unwrap_err().word(), "key_invalid");
    assert_eq!(Key::instrument("A\0B").unwrap_err().word(), "key_invalid");
    assert_eq!(Key::stem("").unwrap_err().word(), "key_invalid");

    let stem = format!("ZZ_REFUSED_{}_TAIL_ONE.csv", "X".repeat(80));
    let twin = format!("ZZ_REFUSED_{}_TAIL_TWO.csv", "X".repeat(80));
    let cut = Key::stem(&stem).unwrap();
    assert_eq!(cut.text().len(), KEY_LEN);
    assert_eq!(cut.whole(), Some(stem.as_str()));
    assert_eq!(
        cut.hash(),
        verdict::key_hash(stem.as_bytes()),
        "the hash is of the whole name"
    );
    // A multi-byte character straddling byte 64 is not split.
    let accented = format!("{}é{}", "Y".repeat(63), "Y".repeat(10));
    let kept = stem_within(&accented).unwrap();
    assert_eq!(kept.text(), "Y".repeat(63));
    assert_eq!(Key::stem("ZZ_SHORT.csv").unwrap().whole(), None);

    let root = Root::new();
    let series = root.series();
    let mut refused = Facts::of(Code::SourceRefused);
    refused.reason = 41;
    let entries = vec![
        Entry {
            key: cut.clone(),
            facts: refused,
            details: vec![Detail {
                kind: 2,
                payload: b"line 7: invented why".to_vec(),
            }],
        },
        Entry {
            key: Key::stem(&twin).unwrap(),
            facts: refused,
            details: Vec::new(),
        },
        Entry {
            key: kept.clone(),
            facts: refused,
            details: Vec::new(),
        },
    ];
    verdict::commit(&series, &generation(71, entries)).unwrap();
    let mut reader = Reader::new();
    for (whole, other_kinds) in [(&stem, vec![2u16]), (&twin, vec![]), (&accented, vec![])] {
        let answer = reader
            .lookup(&series, day(), &Key::stem(whole).unwrap())
            .unwrap()
            .answer
            .unwrap();
        let Found::Record { ordinal, record } = &answer.found else {
            panic!("{whole} must be found");
        };
        assert_ne!(
            record.facts.flags & flags::NAME_CUT,
            0,
            "{whole} carries name_cut"
        );
        assert_eq!(record.facts.reason, 41);
        let details = reader
            .details(
                &series,
                day(),
                answer.header.generation,
                Some((*ordinal, record)),
            )
            .unwrap();
        assert_eq!(details[0].kind, DETAIL_NAME);
        assert_eq!(
            details[0].payload,
            whole.as_bytes(),
            "the whole name is kept"
        );
        let kinds: Vec<u16> = details[1..].iter().map(|d| d.kind).collect();
        assert_eq!(kinds, other_kinds);
    }
}

/// IA-VD-05 — a `.tmp` image is never read: a stray temporary of another
/// writer, even one holding a whole image of the next generation, changes
/// no answer, and the next commit publishes past it untouched.
#[test]
fn a_temporary_day_image_is_never_read() {
    let root = Root::new();
    let series = root.series();
    let first = verdict::commit(
        &series,
        &generation(81, vec![entry("NSE-FNO-ZZTMP-A", Code::Match)]),
    )
    .unwrap();
    // A whole image of another day's content, left under a temporary name.
    let other = Root::new();
    let decoy = verdict::commit(
        &other.series(),
        &generation(82, vec![entry("NSE-FNO-ZZTMP-A", Code::StoreUnreadable)]),
    )
    .unwrap();
    let stray = first
        .path
        .with_file_name("2031-03-04.g0000000002.vdt.99999-82.tmp");
    fs::copy(&decoy.path, &stray).unwrap();
    let stray_bytes = fs::read(&stray).unwrap();

    let key = Key::instrument("NSE-FNO-ZZTMP-A").unwrap();
    let mut reader = Reader::new();
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(answer.code(), Some(Code::Match));
    assert_eq!(answer.header.generation, 1);

    let next = verdict::commit(
        &series,
        &generation(83, vec![entry("NSE-FNO-ZZTMP-A", Code::NotCompared)]),
    )
    .unwrap();
    assert_eq!(next.generation, 2);
    assert_eq!(
        fs::read(&stray).unwrap(),
        stray_bytes,
        "another writer's temporary is left alone"
    );
    assert_eq!(
        reader
            .lookup(&series, day(), &key)
            .unwrap()
            .answer
            .unwrap()
            .code(),
        Some(Code::NotCompared)
    );
}

/// IA-VD-06 — records are sorted by (code, key); one code is one slice,
/// found from the header's counts, and the bytes do not depend on the order
/// the entries arrived in.
#[test]
fn one_codes_records_are_one_contiguous_slice() {
    let n = 600;
    let mut forward = day_of(n);
    let root = Root::new();
    let series = root.series();
    let a = verdict::commit(&series, &generation(91, forward.clone())).unwrap();
    forward.reverse();
    forward.rotate_left(17);
    let other = Root::new();
    let b = verdict::commit(&other.series(), &generation(91, forward)).unwrap();
    assert_eq!(
        fs::read(&a.path).unwrap(),
        fs::read(&b.path).unwrap(),
        "the same verdicts in another order are the same bytes"
    );

    let image = verdict::read_image(&a.path).unwrap();
    let order: Vec<(u8, String)> = image
        .records
        .iter()
        .map(|(r, _)| (r.facts.code.code(), r.key.text().to_owned()))
        .collect();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted);

    let mut reader = Reader::new();
    let mut total = 0u32;
    let mut mismatches = 0u32;
    for code in Code::ALL {
        let (start, count) = image.header.slice_of(code);
        let want: Vec<String> = image
            .records
            .iter()
            .filter(|(r, _)| r.facts.code == code)
            .map(|(r, _)| r.key.text().to_owned())
            .collect();
        assert_eq!(usize::try_from(count).unwrap(), want.len(), "{code}");
        let slice: Vec<String> = image.records
            [usize::try_from(start).unwrap()..usize::try_from(start + count).unwrap()]
            .iter()
            .map(|(r, _)| r.key.text().to_owned())
            .collect();
        assert_eq!(slice, want, "{code}'s records are one contiguous slice");
        let page = reader
            .page(&series, day(), code, 0, count)
            .unwrap()
            .unwrap();
        let paged: Vec<String> = page.iter().map(|r| r.key.text().to_owned()).collect();
        assert_eq!(paged, want, "{code}'s page");
        if count > 2 {
            let tail = reader.page(&series, day(), code, 1, 2).unwrap().unwrap();
            assert_eq!(tail.len(), 2);
            assert_eq!(tail[0].key.text(), want[1]);
        }
        total += count;
        if code.is_mismatch() {
            mismatches += count;
        }
    }
    assert_eq!(total, u32::try_from(n).unwrap());
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_eq!(year.entry(day()).mismatches, mismatches);

    // Every mismatch is in at most two slices, located from the counts: codes
    // 2 to 5, then 7 to 9, with code 6 (not a mismatch) between them.
    let [low, high] = image.header.mismatch_slices();
    assert_eq!(low.1 + high.1, mismatches);
    for (start, count) in [low, high] {
        let from = usize::try_from(start).unwrap();
        let to = usize::try_from(start + count).unwrap();
        assert!(
            image.records[from..to]
                .iter()
                .all(|(r, _)| r.facts.code.is_mismatch()),
            "a mismatch slice holds only mismatches"
        );
    }
    let (six, sixes) = image.header.slice_of(Code::SourceRefused);
    assert!(sixes > 0, "the fixture has source_refused records");
    assert_eq!((low.0 + low.1, six + sixes), (six, high.0));
    let past = reader
        .page(&series, day(), Code::NoLedger, 1_000, 5)
        .unwrap()
        .unwrap();
    assert!(
        past.is_empty(),
        "a page past the slice is empty, never another code's"
    );
}

/// IA-VD-07 — two writers on one day never lose a record: each commits its
/// own keys one at a time with `Carry::Others`, and the newest generation
/// holds every key, each with the run that wrote it.
#[test]
fn two_writers_on_one_day_never_lose_a_record() {
    let root = Root::new();
    let per = 15u64;
    std::thread::scope(|s| {
        for writer in 0..2u64 {
            let root = &root;
            s.spawn(move || {
                let series = root.series();
                for i in 0..per {
                    let run = 1_000 + writer * 100 + i;
                    let mut g = generation(
                        run,
                        vec![entry(&format!("NSE-FNO-ZZW{writer}-{i:03}"), Code::Match)],
                    );
                    g.carry = Carry::Others;
                    verdict::commit(&series, &g).unwrap();
                }
            });
        }
    });
    let series = root.series();
    let mut reader = Reader::new();
    let year = reader.year(&series, 2031).unwrap().unwrap();
    let entry = year.entry(day());
    assert_eq!(
        u64::from(entry.generation),
        2 * per,
        "one generation per commit"
    );
    assert_eq!(u64::from(entry.records), 2 * per, "no record is lost");
    for writer in 0..2u64 {
        for i in 0..per {
            let key = Key::instrument(&format!("NSE-FNO-ZZW{writer}-{i:03}")).unwrap();
            let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
            assert_eq!(u64::from(found(&answer).run), 1_000 + writer * 100 + i);
        }
    }
    for g in 1..=entry.generation {
        let path = series.image_path(day(), g);
        assert_eq!(
            verdict::check_trailer(&path).unwrap().records,
            g,
            "generation {g} carried all before it"
        );
    }
}

/// A carried record keeps its own run, flags and details; this run's entry
/// replaces the committed one of the same key.
#[test]
fn a_partial_run_carries_the_others_whole() {
    let root = Root::new();
    let series = root.series();
    let mut first = day_of(5);
    first[2].facts.flags = flags::SPOT_MISSING;
    first[2].details = vec![Detail {
        kind: 11,
        payload: vec![1, 2, 3],
    }];
    verdict::commit(&series, &generation(111, first)).unwrap();
    let mut g = generation(112, vec![entry(&name(4), Code::ExtraInStore)]);
    g.carry = Carry::Others;
    let done = verdict::commit(&series, &g).unwrap();
    assert_eq!((done.records, done.carried), (5, 4));
    let image = verdict::read_image(&done.path).unwrap();
    assert_eq!(image.header.carried, 4);
    let by_key: HashMap<String, (verdict::Record, Vec<Detail>)> = image
        .records
        .into_iter()
        .map(|(r, d)| (r.key.text().to_owned(), (r, d)))
        .collect();
    let (two, details) = &by_key[&name(2)];
    assert_eq!(two.run, 111);
    assert_eq!(two.facts.flags, flags::SPOT_MISSING);
    assert_eq!(
        details,
        &vec![Detail {
            kind: 11,
            payload: vec![1, 2, 3]
        }]
    );
    let (four, _) = &by_key[&name(4)];
    assert_eq!((four.run, four.facts.code), (112, Code::ExtraInStore));

    // A whole-day run drops what it did not verify.
    let whole = verdict::commit(
        &series,
        &generation(113, vec![entry(&name(0), Code::Match)]),
    )
    .unwrap();
    assert_eq!((whole.records, whole.carried), (1, 0));

    // A carry whose committed image is gone is refused by name, never a
    // generation that silently lost the other keys.
    fs::remove_file(&whole.path).unwrap();
    let mut g = generation(114, vec![entry(&name(1), Code::Match)]);
    g.carry = Carry::Others;
    match verdict::commit(&series, &g) {
        Err(VerdictError::ImageMissing { path }) => assert_eq!(path, whole.path),
        other => panic!("expected image_missing, got {other:?}"),
    }
    let mut reader = Reader::new();
    assert_eq!(
        reader
            .lookup(&series, day(), &Key::instrument(&name(0)).unwrap())
            .unwrap_err()
            .word(),
        "image_missing",
        "a lookup of a committed image that is gone says so"
    );
}

/// IA-VD-09 — an image left linked and uncommitted by a crash is kept, and
/// the next writer steps past it; sixteen such images are refused by name
/// with nothing committed.
#[test]
fn a_crash_between_link_and_commit_never_overwrites_an_image() {
    let root = Root::new();
    let series = root.series();
    let key = Key::instrument("NSE-FNO-ZZLINK-A").unwrap();
    let first = verdict::commit(
        &series,
        &generation(121, vec![entry("NSE-FNO-ZZLINK-A", Code::Match)]),
    )
    .unwrap();
    // The state a writer killed after its link leaves: image 2 in place,
    // the index still at generation 1.
    let orphan = series.image_path(day(), 2);
    fs::hard_link(&first.path, &orphan).unwrap();
    let orphan_bytes = fs::read(&orphan).unwrap();

    let next = verdict::commit(
        &series,
        &generation(122, vec![entry("NSE-FNO-ZZLINK-A", Code::VendorHole)]),
    )
    .unwrap();
    assert_eq!(
        (next.generation, next.previous, next.stepped_past),
        (3, 1, 1)
    );
    assert_eq!(fs::read(&orphan).unwrap(), orphan_bytes);
    let mut reader = Reader::new();
    assert_eq!(
        reader
            .lookup(&series, day(), &key)
            .unwrap()
            .answer
            .unwrap()
            .code(),
        Some(Code::VendorHole)
    );

    // Every generation the next writer could take is already linked.
    for g in 4..4 + MAX_LINK_TRIES {
        fs::write(series.image_path(day(), g), b"left by a crash").unwrap();
    }
    let refused = verdict::commit(
        &series,
        &generation(123, vec![entry("NSE-FNO-ZZLINK-A", Code::Match)]),
    );
    match refused {
        Err(VerdictError::GenerationCollision { path }) => {
            assert_eq!(path, series.image_path(day(), 3 + MAX_LINK_TRIES));
        }
        other => panic!("expected generation_collision, got {other:?}"),
    }
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(answer.header.generation, 3, "nothing was committed");
    for g in 4..4 + MAX_LINK_TRIES {
        assert_eq!(
            fs::read(series.image_path(day(), g)).unwrap(),
            b"left by a crash"
        );
    }
    let left: Vec<String> = fs::read_dir(series.dir().join("2031"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| Path::new(n).extension().is_some_and(|e| e == "tmp"))
        .collect();
    assert!(
        left.is_empty(),
        "the refused writer removed its temporaries: {left:?}"
    );
}

/// IA-VD-10 — an interrupted `.vdi` creation never leaves a half file in
/// place: a stray temporary of a killed creator is not an index, and the
/// next writer makes the index whole.
#[test]
fn a_year_index_is_born_whole() {
    let root = Root::new();
    let series = root.series();
    fs::create_dir_all(series.dir()).unwrap();
    let stray = series.dir().join("2031.vdi.99999-7.tmp");
    fs::write(&stray, vec![0x5A; 9_000]).unwrap();
    let key = Key::instrument("NSE-FNO-ZZBORN-A").unwrap();
    let mut reader = Reader::new();
    let none = reader.lookup(&series, day(), &key).unwrap();
    assert_eq!(none.answer, None, "no index: never verified");
    assert_eq!(none.cost.opens, 0);
    assert_eq!(reader.year(&series, 2031).unwrap(), None);

    verdict::commit(
        &series,
        &generation(131, vec![entry("NSE-FNO-ZZBORN-A", Code::Match)]),
    )
    .unwrap();
    assert_eq!(fs::metadata(series.year_index(2031)).unwrap().len(), 32_768);
    assert!(
        stray.exists(),
        "another creator's temporary is never opened or removed"
    );
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_eq!(year.days.len(), 366);
    assert_eq!(year.entry(day()).generation, 1);
    // Every other day of the year is never verified.
    let other = Day::new(2031, 3, 5).unwrap();
    assert_eq!(reader.lookup(&series, other, &key).unwrap().answer, None);
}

/// IA-VD-11 (the record half; the month probe is IA-6's) — a verdict whose
/// month was appended to, repaired or re-keyed after its verification reads
/// stale, and so does one an older verifier wrote or whose cross-kind input
/// is pending; a month with no `.tix` reads `stale: unknown (no .tix)` and a
/// month a writer holds (`MonthNow::Busy`) reads `stale: unknown (month
/// busy)`, never fresh; a check not made is named and the answer is not
/// fresh. There is no definition check (round 4, BL-F): a later close that
/// changed the stored bars is `store_changed`.
#[test]
fn a_verdict_whose_store_or_verifier_changed_reads_stale() {
    let root = Root::new();
    let series = root.series();
    let mut held = Facts::of(Code::Match);
    held.month_generation = 2;
    held.first_row = 4_000;
    held.rows = 375;
    let mut pending = Facts::of(Code::Match);
    pending.flags = flags::DEFERRED_PENDING;
    let entries = vec![
        Entry {
            key: Key::instrument("NSE-FNO-ZZSTALE-A").unwrap(),
            facts: held,
            details: Vec::new(),
        },
        Entry {
            key: Key::instrument("NSE-FNO-ZZSTALE-B").unwrap(),
            facts: pending,
            details: Vec::new(),
        },
    ];
    verdict::commit(&series, &generation(141, entries)).unwrap();
    let mut reader = Reader::new();
    let a = reader
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZSTALE-A").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    let same = MonthNow::Read {
        generation: 2,
        first_row: 4_000,
        rows: 375,
    };
    let now = Now {
        verifier: 3,
        month: same,
    };
    assert!(a.staleness(&now).is_fresh(), "nothing changed");

    let older = a.staleness(&Now { verifier: 4, ..now });
    assert_eq!(
        older.stale,
        vec![Stale::Verifier {
            image: 3,
            running: 4
        }]
    );
    assert!(!older.is_fresh());
    let newer = a.staleness(&Now { verifier: 2, ..now });
    assert!(
        newer.is_fresh(),
        "a newer image than the build is not stale"
    );

    let read = |generation, first_row, rows| Now {
        month: MonthNow::Read {
            generation,
            first_row,
            rows,
        },
        ..now
    };
    let appended = a.staleness(&read(2, 4_000, 22_500));
    assert_eq!(
        appended.stale,
        vec![Stale::StoreChanged {
            verified: (2, 4_000, 375),
            now: (2, 4_000, 22_500)
        }],
        "a resume appended the rest of the day"
    );
    let repaired = a.staleness(&read(3, 4_000, 375));
    assert_eq!(
        repaired.stale,
        vec![Stale::StoreChanged {
            verified: (2, 4_000, 375),
            now: (3, 4_000, 375)
        }],
        "a repair published a generation"
    );
    let rekeyed = a.staleness(&read(2, 9_100, 375));
    assert_eq!(
        rekeyed.stale,
        vec![Stale::StoreChanged {
            verified: (2, 4_000, 375),
            now: (2, 9_100, 375)
        }],
        "a re-key moved the day's rows"
    );
    for changed in [appended, repaired, rekeyed] {
        assert!(!changed.is_fresh());
        assert_eq!(changed.stale[0].to_string(), "stale: store_changed");
    }

    let no_tix = a.staleness(&Now {
        month: MonthNow::NoTix,
        ..now
    });
    assert_eq!(no_tix.stale, vec![Stale::StoreUnknown(Unknown::NoTix)]);
    assert_eq!(no_tix.stale[0].to_string(), "stale: unknown (no .tix)");
    assert!(!no_tix.is_fresh());
    let busy = a.staleness(&Now {
        month: MonthNow::Busy,
        ..now
    });
    assert_eq!(busy.stale, vec![Stale::StoreUnknown(Unknown::MonthBusy)]);
    assert_eq!(busy.stale[0].to_string(), "stale: unknown (month busy)");
    assert!(!busy.is_fresh(), "a month a writer holds is never fresh");
    assert!(busy.unchecked.is_empty(), "the probe ran; it met a writer");

    let unchecked = a.staleness(&Now {
        verifier: 3,
        month: MonthNow::NotChecked,
    });
    assert!(unchecked.stale.is_empty());
    assert_eq!(unchecked.unchecked, vec![Check::Month]);
    assert!(!unchecked.is_fresh(), "an unchecked verdict is never fresh");

    // Every reason at once, in check order.
    let all = a.staleness(&Now {
        verifier: 9,
        month: MonthNow::Busy,
    });
    assert_eq!(
        all.stale,
        vec![
            Stale::Verifier {
                image: 3,
                running: 9
            },
            Stale::StoreUnknown(Unknown::MonthBusy)
        ]
    );

    let b = reader
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZSTALE-B").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    let b_now = read(0, NONE, 0);
    assert_eq!(b.staleness(&b_now).stale, vec![Stale::CrossKindPending]);
    assert_eq!(
        b.staleness(&b_now).stale[0].to_string(),
        "stale: cross_kind_pending"
    );

    let none = reader
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZSTALE-Z").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(none.found, Found::NoRecord(NoRecord::NotInDay));
    assert_eq!(
        none.staleness(&now).unchecked,
        vec![Check::Month],
        "a no-record answer has no row range to compare"
    );
    assert!(!none.staleness(&now).is_fresh());
}

/// IA-VD-12 — the detail section round-trips entries of any u16 kind and any
/// length up to 65,535 bytes (`DETAILS` lives in `pull`, which `store` cannot
/// name, so the named kinds are IA-GV-27's), for a record and for the day,
/// through the whole-image read and through the reader; and past 4 MiB the
/// day says `details_capped`: the owners that did not fit keep their counts
/// only and carry the flag.
#[test]
fn every_detail_survives_and_the_cap_is_named() {
    let root = Root::new();
    let series = root.series();
    let pattern = |kind: u16, len: usize| -> Vec<u8> {
        (0..len)
            .map(|i| u8::try_from((i + usize::from(kind)) % 251).unwrap())
            .collect()
    };
    let shapes: [(u16, usize); 12] = [
        (1, 0),
        (2, 1),
        (18, 2),
        (19, 240),
        (255, 241),
        (256, 1_000),
        (0x1234, 4_096),
        (0x7FFF, 65_534),
        (0x8000, 65_535),
        (0xFFFE, 7),
        (u16::MAX, 65_535),
        (3, 0),
    ];
    let every: Vec<Detail> = shapes
        .iter()
        .map(|(kind, len)| Detail {
            kind: *kind,
            payload: pattern(*kind, *len),
        })
        .collect();
    let mut g = generation(151, day_of(4));
    g.entries[1].details = every.clone();
    g.entries[3].details = vec![Detail {
        kind: u16::MAX,
        payload: Vec::new(),
    }];
    g.day_details = vec![
        Detail {
            kind: u16::MAX,
            payload: pattern(9, 65_535),
        },
        Detail {
            kind: 6,
            payload: b"ZZ/first/path".to_vec(),
        },
    ];
    let done = verdict::commit(&series, &g).unwrap();
    assert!(!done.details_capped);
    let image = verdict::read_image(&done.path).unwrap();
    assert_eq!(image.day_details, g.day_details);
    assert_eq!(image.header.detail_entries, 12 + 1 + 2);
    let mut reader = Reader::new();
    assert_eq!(
        reader.details(&series, day(), 1, None).unwrap(),
        g.day_details,
        "the day's own entries"
    );
    for (i, want) in [
        (0usize, vec![]),
        (1, every.clone()),
        (2, vec![]),
        (3, g.entries[3].details.clone()),
    ] {
        let ordinal = image
            .records
            .iter()
            .position(|(r, _)| r.key.text() == name(i))
            .unwrap();
        let (record, details) = &image.records[ordinal];
        assert_eq!(details, &want, "record {i}, whole image");
        assert_eq!(record.detail_at == 0, want.is_empty());
        assert_eq!(
            reader
                .details(
                    &series,
                    day(),
                    1,
                    Some((u32::try_from(ordinal).unwrap(), record))
                )
                .unwrap(),
            want,
            "record {i}, through the reader"
        );
    }

    // Past the cap: 100 records of 3 × 60,000 bytes each is about 18 MB.
    let big = Detail {
        kind: 2,
        payload: vec![0x33; 60_000],
    };
    let mut g = generation(152, day_of(100));
    for e in &mut g.entries {
        e.details = vec![big.clone(), big.clone(), big.clone()];
    }
    let capped = verdict::commit(&series, &g).unwrap();
    assert!(capped.details_capped);
    let image = verdict::read_image(&capped.path).unwrap();
    assert_ne!(
        image.day.flags & flags::DETAILS_CAPPED,
        0,
        "the day says details_capped"
    );
    assert!(usize::try_from(image.header.detail_bytes).unwrap() <= DETAIL_CAP);
    let kept = image.records.iter().filter(|(_, d)| !d.is_empty()).count();
    let dropped = image
        .records
        .iter()
        .filter(|(r, d)| d.is_empty() && r.facts.flags & flags::DETAILS_CAPPED != 0)
        .count();
    assert_eq!(
        kept,
        DETAIL_CAP / (3 * (8 + 60_000)),
        "whole owners fit until the cap"
    );
    assert_eq!(
        kept + dropped,
        100,
        "every other owner keeps counts only and says so"
    );
    for (r, d) in &image.records {
        assert_eq!(d.is_empty(), r.detail_at == 0);
    }
    let units: u32 = image
        .records
        .iter()
        .map(|(r, _)| r.facts.source_units)
        .sum();
    assert_eq!(
        units,
        (0..100).sum::<u32>(),
        "every record keeps its counts"
    );
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_ne!(year.entry(day()).flags & flags::DETAILS_CAPPED, 0);
}

/// IA-VD-13 — every flag bit and every code of the v1 table is held by the
/// format: `flags::ALL` names exactly the 22 bits 0 to 21 and `KNOWN` is
/// `(1 << 22) - 1`; a record with all 22 named bits set and every field set
/// reads back whole, with the largest detail payload (65,535 bytes); a writer
/// refuses bit 22, and any other unnamed bit, `flag_unknown` with nothing
/// written; a reader given an image whose record carries bit 22 (bytes as a
/// later build would write them) reads it whole, names the bit `flag_22` and
/// never drops it, not even when a later commit carries the record; and code
/// 18 `owed` sorts and colours by D-4879's severity.
#[test]
fn format_v1_holds_every_fact_it_names() {
    assert_eq!(flags::ALL.len(), 22);
    let mut union = 0u64;
    for (i, (bit, word)) in flags::ALL.iter().enumerate() {
        assert_eq!(*bit, 1u64 << i, "{word} is bit {i}");
        union |= bit;
    }
    assert_eq!(union, flags::KNOWN);
    assert_eq!(flags::KNOWN, (1u64 << 22) - 1);
    assert_eq!(
        (flags::ALL[20].1, flags::ALL[21].1),
        ("lineage_unchecked", "source_crc_unrecorded")
    );
    assert_eq!(
        flags::words(flags::NAME_CUT | flags::TIX_ABSENT),
        vec!["tix_absent", "name_cut"]
    );
    assert_eq!(
        flags::words(flags::SOURCE_CRC_UNRECORDED | 1 << 22 | 1 << 63),
        vec!["source_crc_unrecorded", "flag_22", "flag_63"],
        "an unnamed bit is named by its number, never omitted"
    );
    assert_eq!(flags::words(flags::KNOWN).len(), 22);
    assert_eq!(flags::unnamed(flags::KNOWN | 1 << 22), 1 << 22);

    let facts = Facts {
        code: Code::BuiltUnderEarlierRule,
        rule_set: 0xFE,
        reason: 0xFFFE,
        flags: flags::KNOWN,
        source_units: 1,
        store_units: 2,
        missing: 3,
        extra: 4,
        value_diff: 5,
        volume_diff: 6,
        oi_diff: 7,
        first_diff: 8,
        month_generation: 9,
        first_row: 10,
        rows: 11,
        first_unit: 12,
        last_unit: u32::MAX - 1,
    };
    let details = vec![
        Detail {
            kind: 2,
            payload: vec![0x5A; 240],
        },
        Detail {
            kind: u16::MAX,
            payload: (0..65_535usize)
                .map(|i| u8::try_from(i % 253).unwrap())
                .collect(),
        },
    ];
    let day_block = DayBlock {
        calendar_kind: 9,
        source_held: SourceHeld::Yes,
        v3: V3Manifest::Disagrees,
        journal: JournalOutcome::DoneWithRefusals,
        state: DayState::Complete,
        worst: Code::Match,
        close_reason: 0xBEEF,
        flags: flags::KNOWN,
        listing_entries: 21,
        files_read: 22,
        skipped: 23,
        refused: 24,
        not_reached: 25,
        skip_census: [31, 32, 33, 34, 35, 36, 37, 38],
        journal_seconds: u64::MAX - 1,
        stored_seconds: 1 << 40,
        oracle_seconds: 7,
        shares_absent: 26,
    };
    let root = Root::new();
    let series = root.series();
    let mut g = generation(
        u64::from(u32::MAX),
        vec![Entry {
            key: Key::instrument("NSE-FNO-ZZALL-A").unwrap(),
            facts,
            details: details.clone(),
        }],
    );
    g.day_block = day_block;
    g.mode = Mode::ZerodhaCold;
    g.verifier = 0xABCD;
    g.written_at = 0x1122_3344_5566_7788;
    g.origin = *b"0123456789abcdef";
    let done = verdict::commit(&series, &g).unwrap();
    let image = verdict::read_image(&done.path).unwrap();
    let (record, read_details) = &image.records[0];
    assert_eq!(record.facts, facts);
    assert_eq!(record.run, u32::MAX);
    assert_eq!(read_details, &details);
    assert_eq!(
        image.day,
        DayBlock {
            worst: Code::BuiltUnderEarlierRule,
            ..day_block
        }
    );
    let h = image.header;
    assert_eq!(
        (h.feed, h.exchange, h.segment, h.timeframe, h.unit),
        (
            Feed::Gdfl,
            Exchange::Nse,
            Segment::Fno,
            Timeframe::SECOND_1,
            Unit::Second
        )
    );
    assert_eq!(
        (h.run, h.written_at, h.verifier, h.mode, h.rule_set),
        (
            u64::from(u32::MAX),
            0x1122_3344_5566_7788,
            0xABCD,
            Mode::ZerodhaCold,
            2
        )
    );
    assert_eq!(h.origin, *b"0123456789abcdef");
    assert_eq!(h.counts[15], 1);
    let year = Reader::new().year(&series, 2031).unwrap().unwrap();
    assert_eq!(year.entry(day()).flags, flags::KNOWN);

    // A writer refuses every bit it does not name, and writes nothing.
    let mut bad = generation(5, vec![entry("NSE-FNO-ZZALL-B", Code::Match)]);
    for bit in [22u32, 23, 40, 63] {
        bad.entries[0].facts.flags = flags::KNOWN | 1 << bit;
        match verdict::commit(&series, &bad) {
            Err(VerdictError::FlagUnknown { bits }) => assert_eq!(bits, 1 << bit),
            other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
        }
    }
    bad.entries[0].facts.flags = 0;
    bad.day_block.flags = 1 << 22;
    assert_eq!(
        verdict::commit(&series, &bad).unwrap_err().word(),
        "flag_unknown",
        "a day flag is held to the same rule"
    );
    bad.day_block.flags = 0;
    assert_eq!(
        Reader::new().year(&series, 2031).unwrap().unwrap().commit,
        1,
        "no refused writer committed"
    );
    let left: Vec<String> = fs::read_dir(series.dir().join("2031"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        left,
        vec!["2031-03-04.g0000000001.vdt".to_owned()],
        "a refused writer left nothing"
    );

    // A later build's image: its record carries bit 22. Its bytes are made
    // here as that build would write them (the record's flags, its CRC-32C
    // and the image's BLAKE3 trailer), never through this build's writer.
    let later = Root::new();
    let lseries = later.series();
    let mut lg = generation(301, vec![entry("NSE-FNO-ZZNEW-A", Code::Match)]);
    lg.entries[0].facts.flags = flags::LINEAGE_UNCHECKED;
    let written = verdict::commit(&lseries, &lg).unwrap();
    let mut bytes = fs::read(&written.path).unwrap();
    let records_at = 512 + 8 * (16 + 7);
    let with_22 = flags::LINEAGE_UNCHECKED | 1 << 22;
    bytes[records_at + 80..records_at + 88].copy_from_slice(&with_22.to_le_bytes());
    let crc = store::crc::crc32c(&bytes[records_at..records_at + 156]);
    bytes[records_at + 156..records_at + 160].copy_from_slice(&crc.to_le_bytes());
    let end = bytes.len() - 32;
    let trailer = brutex_core::blake3::hash(&bytes[..end]);
    bytes[end..].copy_from_slice(&trailer);
    fs::write(&written.path, &bytes).unwrap();

    let whole = verdict::check_trailer(&written.path).unwrap();
    assert_eq!(whole.records, 1, "the later image reads whole");
    let read = verdict::read_image(&written.path).unwrap();
    assert_eq!(read.records[0].0.facts.flags, with_22);
    let answer = Reader::new()
        .lookup(
            &lseries,
            day(),
            &Key::instrument("NSE-FNO-ZZNEW-A").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(answer.flags(), with_22, "the reader keeps bit 22");
    assert_eq!(
        flags::words(answer.flags()),
        vec!["lineage_unchecked", "flag_22"]
    );
    // A partial run of this build carries that record whole, bit 22 and all.
    let mut carry = generation(302, vec![entry("NSE-FNO-ZZNEW-B", Code::Match)]);
    carry.carry = Carry::Others;
    let carried = verdict::commit(&lseries, &carry).unwrap();
    assert_eq!(carried.carried, 1);
    let kept = Reader::new()
        .lookup(
            &lseries,
            day(),
            &Key::instrument("NSE-FNO-ZZNEW-A").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(kept.flags(), with_22, "a carry never drops a later bit");
    assert_eq!(kept.header.generation, 2);

    // Code 18 `owed`: severity 9, after `day_partial` and before
    // `source_refused`; it sorts last by code and it colours the day.
    let owed = Root::new();
    let oseries = owed.series();
    let o = verdict::commit(
        &oseries,
        &generation(
            311,
            vec![
                entry("NSE-FNO-ZZOWED-A", Code::Owed),
                entry("NSE-FNO-ZZOWED-B", Code::SourceRefused),
                entry("NSE-FNO-ZZOWED-C", Code::Match),
                entry("NSE-FNO-ZZOWED-D", Code::NotCompared),
            ],
        ),
    )
    .unwrap();
    assert_eq!(o.worst, Code::Owed);
    let image = verdict::read_image(&o.path).unwrap();
    let codes: Vec<Code> = image.records.iter().map(|(r, _)| r.facts.code).collect();
    assert_eq!(
        codes,
        vec![
            Code::Match,
            Code::SourceRefused,
            Code::NotCompared,
            Code::Owed
        ]
    );
    assert_eq!(image.header.counts[18], 1);
    assert_eq!(image.header.slice_of(Code::Owed), (3, 1));
    let owed_day = Reader::new()
        .year(&oseries, 2031)
        .unwrap()
        .unwrap()
        .entry(day());
    assert_eq!(owed_day.worst_code(), Some(Code::Owed));
    assert_eq!(owed_day.mismatches, 0, "owed is never a store defect");
    assert_eq!(Code::Owed.worse(Code::DayPartial), Code::DayPartial);
    assert_eq!(Code::Owed.worse(Code::SourceRefused), Code::Owed);

    // The other refusals of inputs version 1 cannot hold.
    let mut bad = generation(u64::from(u32::MAX) + 1, Vec::new());
    assert_eq!(
        verdict::commit(&series, &bad).unwrap_err().word(),
        "run_id_overflow"
    );
    bad.run = 5;
    bad.entries = vec![entry("NSE-FNO-ZZALL-B", Code::Match)];
    bad.entries[0].details = vec![Detail {
        kind: 0,
        payload: Vec::new(),
    }];
    assert_eq!(
        verdict::commit(&series, &bad).unwrap_err().word(),
        "detail_invalid"
    );
    bad.entries[0].details = vec![Detail {
        kind: 3,
        payload: vec![0; 65_536],
    }];
    assert_eq!(
        verdict::commit(&series, &bad).unwrap_err().word(),
        "detail_invalid"
    );
    bad.entries = vec![
        entry("NSE-FNO-ZZALL-B", Code::Match),
        entry("NSE-FNO-ZZALL-B", Code::ExtraInStore),
    ];
    assert_eq!(
        verdict::commit(&series, &bad).unwrap_err().word(),
        "duplicate_key"
    );
    bad.entries.truncate(1);
    bad.day = Day::new(1969, 12, 31).unwrap();
    assert_eq!(
        verdict::commit(&series, &bad).unwrap_err().word(),
        "day_out_of_range"
    );
    let unchanged = Reader::new()
        .lookup(&series, day(), &Key::instrument("NSE-FNO-ZZALL-A").unwrap())
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(
        unchanged.header.generation, 1,
        "no refused input committed anything"
    );
}

/// The codes, their severities and the mismatch set of §6.4, pinned: codes 1
/// to 18 (D-4844, D-4879), a total severity order with `owed` at 9.
#[test]
fn the_codes_and_their_severity_are_the_designs() {
    assert_eq!(Code::ALL.len(), 18);
    let mut severities: Vec<u8> = Code::ALL.iter().map(|c| c.severity()).collect();
    severities.sort_unstable();
    assert_eq!(severities, (1..=18).collect::<Vec<u8>>(), "a total order");
    let by_severity: Vec<u8> = {
        let mut all = Code::ALL.to_vec();
        all.sort_by_key(|c| c.severity());
        all.iter().map(|c| c.code()).collect()
    };
    assert_eq!(
        by_severity,
        vec![
            9, 2, 3, 4, 7, 8, 5, 16, 18, 6, 12, 13, 14, 17, 15, 10, 11, 1
        ]
    );
    let mismatches: Vec<u8> = Code::ALL
        .iter()
        .filter(|c| c.is_mismatch())
        .map(|c| c.code())
        .collect();
    assert_eq!(mismatches, vec![2, 3, 4, 5, 7, 8, 9]);
    for (i, code) in Code::ALL.into_iter().enumerate() {
        assert_eq!(usize::from(code.code()), i + 1, "ALL is in code order");
        assert_eq!(Code::from_code(code.code()), Some(code));
        assert_eq!(code.worse(Code::Match), code);
        assert_eq!(Code::StoreUnreadable.worse(code), Code::StoreUnreadable);
    }
    assert_eq!(Code::from_code(0), None, "0 is unset, never a written code");
    assert_eq!(Code::from_code(19), None);
    assert_eq!(Code::DayPartial.word(), "day_partial");
    assert_eq!(Code::Owed.word(), "owed");
    assert_eq!(Code::Owed.code(), 18);
}

/// The feed, exchange, segment and unit bytes, pinned to their sources: a
/// feed is the bar store's own vendor and its byte is the vendor's position
/// in `Vendor::ALL` plus one, for EVERY vendor of the table (no list of
/// feeds lives here; RA-243). The two feeds in use are frozen at the bytes
/// their images already carry, so an insertion into `Vendor::ALL` fails
/// here. An exchange's and a segment's bytes are frozen numbers.
#[test]
fn the_series_bytes_are_pinned() {
    for (at, vendor) in Vendor::ALL.iter().enumerate() {
        let feed: Feed = *vendor;
        let code = verdict::feed_code(feed).unwrap();
        assert_eq!(usize::from(code), at + 1);
        assert_eq!(verdict::feed_of(code), Some(feed));
    }
    assert_eq!(verdict::feed_code(Feed::Gdfl), Some(4));
    assert_eq!(verdict::feed_code(Feed::Zerodha), Some(5));
    assert_eq!(verdict::feed_of(0), None, "0 is no feed");
    assert_eq!(
        verdict::feed_of(u8::try_from(Vendor::ALL.len() + 1).unwrap()),
        None,
        "a byte past the table is no feed"
    );
    assert_eq!(verdict::exchange_code(Exchange::Nse), 1);
    assert_eq!(verdict::exchange_code(Exchange::Bse), 2);
    for exchange in [Exchange::Nse, Exchange::Bse] {
        assert_eq!(
            verdict::exchange_of(verdict::exchange_code(exchange)),
            Some(exchange)
        );
    }
    assert_eq!(verdict::exchange_of(0), None);
    assert_eq!(verdict::exchange_of(3), None);
    for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
        assert_eq!(
            verdict::segment_of(verdict::segment_code(segment)),
            Some(segment)
        );
    }
    // A timeframe has no byte: it is its own seconds, and its unit is the
    // largest of day, minute and second that divides them.
    for tf in Timeframe::KNOWN {
        let want = match tf.secs() {
            1 => Unit::Second,
            86_400 => Unit::Day,
            _ => Unit::Minute,
        };
        assert_eq!(verdict::unit_of(*tf), want, "{}", tf.as_str());
    }
    let root = Root::new();
    let series = Series {
        root: &root.0,
        feed: Feed::Zerodha,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
    };
    assert_eq!(
        series.image_path(day(), 7),
        root.0
            .join("verify/zerodha/NSE/INDEX/1min/2031/2031-03-04.g0000000007.vdt")
    );
    assert_eq!(
        Series {
            exchange: Exchange::Bse,
            ..series
        }
        .dir(),
        root.0.join("verify/zerodha/BSE/INDEX/1min"),
        "the exchange is a directory level, as in the bar store"
    );
    // Ten digits, the width of u32::MAX: every generation has one name, and
    // one day's names sort in generation order.
    assert_eq!(
        series.image_path(day(), u32::MAX).file_name().unwrap(),
        "2031-03-04.g4294967295.vdt"
    );
    let mut names: Vec<String> = [1_000_000u32, 999_999, 7, u32::MAX, 10]
        .into_iter()
        .map(|g| {
            series
                .image_path(day(), g)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "2031-03-04.g0000000007.vdt",
            "2031-03-04.g0000000010.vdt",
            "2031-03-04.g0000999999.vdt",
            "2031-03-04.g0001000000.vdt",
            "2031-03-04.g4294967295.vdt",
        ]
    );
    assert_eq!(
        series.year_index(2031),
        root.0.join("verify/zerodha/NSE/INDEX/1min/2031.vdi")
    );
    assert_eq!(
        series.lock_path(2031),
        root.0.join("verify/zerodha/NSE/INDEX/1min/2031.vdi.lock")
    );
}

/// The bytes on disk at the offsets §35 states: the header, the day block,
/// the table and the record.
#[test]
fn the_bytes_sit_at_the_offsets_section_35_states() {
    let root = Root::new();
    let series = root.series();
    let mut e = entry("NSE-FNO-ZZBYTE-A", Code::ValueMismatch);
    e.facts.rule_set = 7;
    e.facts.reason = 0x0102;
    e.facts.first_row = 0x0A0B_0C0D;
    let mut g = generation(0x0506_0708, vec![e]);
    g.day_block.state = DayState::Stopped;
    let done = verdict::commit(&series, &g).unwrap();
    let b = fs::read(&done.path).unwrap();
    let u16_at = |at: usize| u16::from_le_bytes(b[at..at + 2].try_into().unwrap());
    let u32_at = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    let u64_at = |at: usize| u64::from_le_bytes(b[at..at + 8].try_into().unwrap());
    assert_eq!(&b[0..8], b"BRUTEXVD");
    assert_eq!(u16_at(8), 1);
    assert_eq!(u16_at(10), 160);
    assert_eq!(&b[12..16], &[4, 3, 1, 0], "gdfl, FNO, NSE, seconds");
    assert_eq!(u32_at(184), 1, "the timeframe is its own seconds: 1s");
    assert!(b[188..252].iter().all(|x| *x == 0), "reserved, zero");
    // Another series, so no byte above is right by coincidence: Zerodha,
    // BSE, INDEX, 15-minute bars counted in minutes.
    let other = Series {
        feed: Feed::Zerodha,
        exchange: Exchange::Bse,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_15,
        ..series
    };
    let other_bytes = fs::read(verdict::commit(&other, &g).unwrap().path).unwrap();
    assert_eq!(
        &other_bytes[12..16],
        &[5, 1, 2, 1],
        "zerodha, INDEX, BSE, minutes"
    );
    assert_eq!(
        u32::from_le_bytes(other_bytes[184..188].try_into().unwrap()),
        900,
        "15 minutes"
    );
    assert_eq!(
        u32_at(16),
        22_342,
        "2031-03-04 is day 22,342 after 1970-01-01"
    );
    assert_eq!(u32_at(20), 1);
    assert_eq!(u32_at(24), 1);
    assert_eq!(u32_at(28), 16);
    assert_eq!(u32_at(32 + 4 * 2), 1, "one value_mismatch");
    assert_eq!(u64_at(128), 0x0506_0708);
    assert_eq!(u16_at(144), 3);
    assert_eq!(b[146], Mode::Cold.code());
    assert_eq!(b[147], 2);
    assert_eq!(u32_at(252), store::crc::crc32c(&b[..252]));
    // The day block: a stopped day's worst is at least day_partial; here a
    // value mismatch is worse.
    assert_eq!(b[256 + 4], DayState::Stopped.code());
    assert_eq!(b[256 + 5], Code::ValueMismatch.code());
    assert_eq!(u32_at(256 + 252), store::crc::crc32c(&b[256..256 + 252]));
    // The table: 16 + 7 slots of 8 bytes, then the record.
    let records_at = 512 + 8 * (16 + 7);
    let hash = verdict::key_hash(b"NSE-FNO-ZZBYTE-A");
    let home = usize::try_from(hash & 15).unwrap();
    assert_eq!(u32_at(512 + 8 * home), 1, "ordinal 0 + 1 at its home slot");
    assert_eq!(
        u32_at(512 + 8 * home + 4),
        u32::try_from(hash >> 32).unwrap()
    );
    let r = records_at;
    assert_eq!(u64_at(r), hash);
    assert_eq!(&b[r + 8..r + 8 + 16], b"NSE-FNO-ZZBYTE-A");
    assert!(b[r + 24..r + 72].iter().all(|x| *x == 0), "zero-padded");
    assert_eq!((b[r + 72], b[r + 73]), (Code::ValueMismatch.code(), 7));
    assert_eq!(u16_at(r + 74), 0x0102);
    assert_eq!(u32_at(r + 76), 0x0506_0708);
    assert_eq!(u32_at(r + 116), NONE, "first differing unit: none");
    assert_eq!(u32_at(r + 128), 0x0A0B_0C0D);
    assert_eq!(u32_at(r + 156), store::crc::crc32c(&b[r..r + 156]));
    assert_eq!(
        b.len(),
        records_at + RECORD_STRIDE + 32,
        "no details: the trailer follows"
    );
    assert_eq!(
        &b[b.len() - 32..],
        &brutex_core::blake3::hash(&b[..b.len() - 32])
    );

    let vdi = fs::read(series.year_index(2031)).unwrap();
    assert_eq!(vdi.len(), INDEX_LEN);
    for slot in [0usize, 16_384] {
        assert_eq!(&vdi[slot..slot + 8], b"BRUTEXVI");
        assert_eq!(
            u16::from_le_bytes(vdi[slot + 12..slot + 14].try_into().unwrap()),
            2031
        );
        let crc = u32::from_le_bytes(vdi[slot + 8_808..slot + 8_812].try_into().unwrap());
        assert_eq!(crc, store::crc::crc32c(&vdi[slot..slot + 8_808]));
    }
    // Commit 1 is in slot 1; day 62 of 2031 (4 March) is entry 62.
    let e = 16_384 + 24 + 24 * 62;
    assert_eq!(
        u64::from_le_bytes(vdi[16_384 + 16..16_384 + 24].try_into().unwrap()),
        1
    );
    assert_eq!(u32::from_le_bytes(vdi[e..e + 4].try_into().unwrap()), 1);
    assert_eq!(vdi[e + 4], Code::ValueMismatch.code());
    assert_eq!(vdi[e + 5], DayState::Stopped.code());
}

/// D-4854 — a partial day is never green: a key the run never reached reads
/// `day_partial` from the day block's state, and the day's worst is at least
/// `day_partial`.
#[test]
fn a_partial_day_is_never_green() {
    let root = Root::new();
    let series = root.series();
    let mut g = generation(161, vec![entry("NSE-FNO-ZZPART-A", Code::Match)]);
    g.day_block.state = DayState::SourceLost;
    g.day_block.not_reached = 9;
    let done = verdict::commit(&series, &g).unwrap();
    assert_eq!(done.worst, Code::DayPartial);
    let mut reader = Reader::new();
    let reached = reader
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZPART-A").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(
        reached.code(),
        Some(Code::Match),
        "the reached key keeps its own verdict"
    );
    let unreached = reader
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZPART-B").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(unreached.found, Found::NoRecord(NoRecord::DayPartial));
    assert_eq!(unreached.code(), Some(Code::DayPartial));
    assert_eq!(unreached.day.not_reached, 9);
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_eq!(year.entry(day()).worst_code(), Some(Code::DayPartial));
}

/// IA-VD-14 — `check_trailer` accepts a whole image and refuses one byte
/// changed anywhere in it, header, day block, table, records, details or the
/// trailer itself, `trailer_mismatch` by name. A lookup never reads the
/// trailer (a changed trailer byte leaves every answer as it was), and a
/// lookup that meets a changed record refuses it `record_corrupt`.
#[test]
fn a_day_images_trailer_names_any_changed_byte() {
    let root = Root::new();
    let series = root.series();
    let mut g = generation(171, day_of(3));
    g.entries[1].details = vec![Detail {
        kind: 4,
        payload: b"ZZ invented detail".to_vec(),
    }];
    g.day_details = vec![Detail {
        kind: 19,
        payload: vec![0xC3; 24],
    }];
    let done = verdict::commit(&series, &g).unwrap();
    let clean = fs::read(&done.path).unwrap();
    let checked = verdict::check_trailer(&done.path).unwrap();
    assert_eq!(
        (checked.generation, checked.records, checked.details),
        (1, 3, 2)
    );
    for at in 0..clean.len() {
        let mut bytes = clean.clone();
        bytes[at] ^= 0x01;
        fs::write(&done.path, &bytes).unwrap();
        match verdict::check_trailer(&done.path) {
            Err(VerdictError::TrailerMismatch { path }) => assert_eq!(path, done.path),
            other => panic!(
                "byte {at} of {}: expected trailer_mismatch, got {other:?}",
                clean.len()
            ),
        }
    }

    fs::write(&done.path, &clean).unwrap();
    let image = verdict::read_image(&done.path).unwrap();

    // A lookup never reads the trailer.
    let mut bytes = clean.clone();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x80;
    fs::write(&done.path, &bytes).unwrap();
    let key = image.records[1].0.key.clone();
    let mut reader = Reader::new();
    assert_eq!(
        found(&reader.lookup(&series, day(), &key).unwrap().answer.unwrap()).key,
        key,
        "a changed trailer changes no answer"
    );
    assert_eq!(
        verdict::check_trailer(&done.path).unwrap_err().word(),
        "trailer_mismatch"
    );

    // A lookup that meets a changed record refuses it by name.
    let records_at = 512 + 8 * (usize::try_from(image.header.capacity).unwrap() + 7);
    let mut bytes = clean.clone();
    bytes[records_at + 160 + 90] ^= 0x10;
    fs::write(&done.path, &bytes).unwrap();
    let mut reader = Reader::new();
    match reader.lookup(&series, day(), &key) {
        Err(VerdictError::RecordCorrupt { ordinal, .. }) => assert_eq!(ordinal, 1),
        other => panic!("expected record_corrupt, got {other:?}"),
    }
    fs::write(&done.path, &clean).unwrap();
    assert!(
        reader
            .lookup(&series, day(), &key)
            .unwrap()
            .answer
            .is_some()
    );
    assert_eq!(verdict::check_trailer(&done.path).unwrap(), checked);
}

/// The handle cache holds at most 64 handles, and a lookup over a year of
/// days stays bounded.
#[test]
fn the_handle_cache_is_bounded() {
    let root = Root::new();
    let series = root.series();
    let days: Vec<Day> = (1..=80u8)
        .map(|i| Day::new(2031, 1 + (i - 1) / 28, 1 + (i - 1) % 28).unwrap())
        .collect();
    for (i, d) in days.iter().enumerate() {
        let mut g = generation(
            200 + u64::try_from(i).unwrap(),
            vec![entry("NSE-FNO-ZZCACHE-A", Code::Match)],
        );
        g.day = *d;
        verdict::commit(&series, &g).unwrap();
    }
    let key = Key::instrument("NSE-FNO-ZZCACHE-A").unwrap();
    let mut reader = Reader::new();
    for d in &days {
        let got = reader.lookup(&series, *d, &key).unwrap();
        assert!(got.cost.opens <= 2 && got.cost.reads == 5);
        assert!(reader.cached() <= verdict::HANDLE_CACHE);
    }
    assert_eq!(reader.cached(), verdict::HANDLE_CACHE);
    // The most recent day is still warm.
    let warm = reader.lookup(&series, days[79], &key).unwrap();
    assert_eq!(warm.cost.opens, 0);
}

/// The committed result names what it did.
#[test]
fn a_commit_reports_what_it_published() {
    let root = Root::new();
    let series = root.series();
    let done: Committed = verdict::commit(&series, &generation(181, day_of(4))).unwrap();
    assert_eq!(done.commit, 1);
    assert_eq!(done.path, series.image_path(day(), 1));
    assert!(done.path.starts_with(&root.0));
    assert!(Path::new(&done.path).exists());
    assert_eq!(done.records, 4);
    assert_eq!(done.staging_cleared, 0);
    assert!(!done.near_ceiling());
    // The watch level is exact: 68,950 records is near the ceiling, one
    // fewer is not (RA-261).
    let at = Committed {
        records: verdict::RECORDS_WATCH,
        ..done.clone()
    };
    assert!(at.near_ceiling());
    let under = Committed {
        records: verdict::RECORDS_WATCH - 1,
        ..done
    };
    assert!(!under.near_ceiling());
}

/// `docs/02-store-format.md` §35 states the constants and offsets this
/// module writes.
#[test]
fn section_35_states_the_format_this_module_writes() {
    let start = FORMAT_DOC
        .find("\n## 35. ")
        .expect("docs/02-store-format.md has a §35");
    let rest = &FORMAT_DOC[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 3);
    let text = rest[..end].split_whitespace().collect::<Vec<_>>().join(" ");
    for fact in [
        "`BRUTEXVD`",
        "`BRUTEXVI`",
        "160-byte",
        "8,812",
        "16,384",
        "32,768",
        "c + 7",
        "`MAX_PROBE` = 8",
        "2^20",
        "4 MiB",
        "`details_capped`",
        "`name_cut`",
        "`key_too_long`",
        "`table_too_large`",
        "`run_id_overflow`",
        "`generation_collision`",
        "`hard_link`",
        "BLAKE3",
        "CRC-32C",
        "`store::verdict::check_trailer`",
        "`trailer_mismatch`",
        "`owed`",
        "`lineage_unchecked`",
        "`source_crc_unrecorded`",
        "`flag_<n>`",
        "`flag_unknown`",
        "`no_record`",
        "`stale: unknown (month busy)`",
        "<EXCHANGE>",
        "timeframe seconds",
        "`feed_unknown`",
        "`store::staging::create`",
        "`staging_linked`",
        "`staging_not_regular`",
        "`staging_contended`",
        "`lock_stalled`",
        "`LOCK_WAIT` = 30 s",
        "D-4922",
        "D-4923",
        "D-4924",
        "10 digits",
        "L-IA-33",
        "D-4808",
        "D-4844",
        "D-4865",
        "D-4879",
    ] {
        assert!(text.contains(fact), "§35 does not state {fact}");
    }
}

/// Every instrument key a store path can name fits the key field, so
/// `key_too_long` can only meet a key no store path holds: exchange (3),
/// segment (at most 5, `INDEX`), symbol and contract (each at most
/// `MAX_SEGMENT_LEN`) and three dashes. Measured on the real store on
/// 10 Oct 2026: the longest of 213,365 GDFL FNO contract keys is 40 bytes.
#[test]
fn every_key_a_store_path_can_name_fits_the_key_field() {
    let longest = "NSE".len() + 1 + "INDEX".len() + 1 + store::path::MAX_SEGMENT_LEN * 2 + 1;
    assert_eq!(longest, 59);
    assert!(longest <= KEY_LEN);
    let widest = format!(
        "NSE-INDEX-{}-{}",
        "S".repeat(store::path::MAX_SEGMENT_LEN),
        "C".repeat(store::path::MAX_SEGMENT_LEN)
    );
    assert_eq!(Key::instrument(&widest).unwrap().text().len(), longest);
}

/// One IA-VD-16 case: the day's state, whether the source holds the day, and
/// the no-record answer with its code, word and reason.
type Case = (
    DayState,
    SourceHeld,
    NoRecord,
    Option<Code>,
    &'static str,
    Option<&'static str>,
);

/// IA-VD-16 — a key with no record answers by the day block's state, never
/// with the day's worst code: on a complete day where one share is
/// `value_mismatch`, a share absent from the source reads `no_record`; a
/// partial day reads `day_partial`, a day the source lacks
/// `source_lacks_day`, a day not imported `owed (not_imported)`, and a day
/// imported with verification off `not_compared (verify_off)`. Each costs the
/// same reads as any lookup.
#[test]
fn a_key_without_a_record_never_shows_another_keys_code() {
    let absent = Key::instrument("NSE-CASH-ZZABSENT").unwrap();
    let cases: [Case; 9] = [
        (
            DayState::Complete,
            SourceHeld::Yes,
            NoRecord::NotInDay,
            None,
            "no_record",
            None,
        ),
        (
            DayState::Complete,
            SourceHeld::Unknown,
            NoRecord::NotInDay,
            None,
            "no_record",
            None,
        ),
        (
            DayState::Stopped,
            SourceHeld::Yes,
            NoRecord::DayPartial,
            Some(Code::DayPartial),
            "day_partial",
            None,
        ),
        (
            DayState::SourceLost,
            SourceHeld::Yes,
            NoRecord::DayPartial,
            Some(Code::DayPartial),
            "day_partial",
            None,
        ),
        (
            DayState::DiskFull,
            SourceHeld::Yes,
            NoRecord::DayPartial,
            Some(Code::DayPartial),
            "day_partial",
            None,
        ),
        (
            DayState::Census,
            SourceHeld::Yes,
            NoRecord::DayPartial,
            Some(Code::DayPartial),
            "day_partial",
            None,
        ),
        (
            DayState::HeldBackCap,
            SourceHeld::Yes,
            NoRecord::DayPartial,
            Some(Code::DayPartial),
            "day_partial",
            None,
        ),
        (
            DayState::NotImported,
            SourceHeld::Yes,
            NoRecord::NotImported,
            Some(Code::Owed),
            "owed",
            Some("not_imported"),
        ),
        (
            DayState::VerifyOff,
            SourceHeld::Yes,
            NoRecord::VerifyOff,
            Some(Code::NotCompared),
            "not_compared",
            Some("verify_off"),
        ),
    ];
    for (state, held, want, code, word, reason) in cases {
        let root = Root::new();
        let series = Series {
            segment: Segment::Cash,
            ..root.series()
        };
        let mut g = generation(
            181,
            vec![
                entry("NSE-CASH-ZZMISMATCH", Code::ValueMismatch),
                entry("NSE-CASH-ZZFINE", Code::Match),
            ],
        );
        g.day_block.state = state;
        g.day_block.source_held = held;
        let done = verdict::commit(&series, &g).unwrap();
        assert_eq!(
            done.worst,
            Code::ValueMismatch,
            "{state:?}: another key's mismatch is the day's worst"
        );
        let got = Reader::new().lookup(&series, day(), &absent).unwrap();
        assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
        let answer = got.answer.unwrap();
        assert_eq!(answer.found, Found::NoRecord(want), "{state:?}, {held:?}");
        assert_eq!(answer.code(), code, "{state:?}: never the day's worst code");
        assert_ne!(answer.code(), Some(Code::ValueMismatch));
        assert_eq!(want.word(), word);
        assert_eq!(want.reason(), reason);
        assert_eq!(NoRecord::of(&answer.day), want);
    }
    // A day the source lacks: `source_lacks_day`, whatever else the state
    // says, unless the day closed partial.
    for (state, want) in [
        (DayState::Complete, NoRecord::SourceLacksDay),
        (DayState::NotImported, NoRecord::SourceLacksDay),
        (DayState::Stopped, NoRecord::DayPartial),
    ] {
        let block = DayBlock {
            state,
            source_held: SourceHeld::No,
            ..DayBlock::of(Code::ValueMismatch)
        };
        assert_eq!(NoRecord::of(&block), want, "{state:?}");
    }
    let root = Root::new();
    let series = root.series();
    let mut g = generation(182, vec![entry("NSE-FNO-ZZHELD-A", Code::ValueMismatch)]);
    g.day_block.source_held = SourceHeld::No;
    g.day_block.state = DayState::NotImported;
    verdict::commit(&series, &g).unwrap();
    let answer = Reader::new()
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZHELD-B").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(answer.code(), Some(Code::SourceLacksDay));
    assert_eq!(NoRecord::SourceLacksDay.word(), "source_lacks_day");
}

// ------------------------------------------------ the rules audit, 11 Oct --

/// The committed entry of the fixture day in `series`' year index, or
/// `None` when the day has no committed generation.
fn committed_day(series: &Series<'_>) -> Option<verdict::DayEntry> {
    Reader::new()
        .year(series, 2031)
        .unwrap()
        .map(|year| year.entry(day()))
        .filter(|entry| entry.generation > 0)
}

/// RA-243: the exchange is part of a verdict family. NSE and BSE keep
/// different calendars, so one day is two day records, in two directories,
/// each read back as its own; and an image filed under one exchange is
/// refused when it is read as the other's.
#[test]
fn two_exchanges_on_one_day_are_two_day_records() {
    let root = Root::new();
    let nse = Series {
        segment: Segment::Cash,
        ..root.series()
    };
    let bse = Series {
        exchange: Exchange::Bse,
        ..nse
    };
    // On NSE the day's run stopped part-way and found a mismatch; on BSE
    // the same day is complete and clean.
    let mut stopped = generation(301, vec![entry("NSE-CASH-ZZTWOEX", Code::ValueMismatch)]);
    stopped.day_block.state = DayState::Stopped;
    let clean = generation(302, vec![entry("BSE-CASH-ZZTWOEX", Code::Match)]);
    let a = verdict::commit(&nse, &stopped).unwrap();
    let b = verdict::commit(&bse, &clean).unwrap();
    assert_ne!(a.path, b.path);
    assert_eq!(
        (a.generation, b.generation),
        (1, 1),
        "neither is the other's generation 2"
    );
    assert!(a.path.starts_with(root.0.join("verify/gdfl/NSE/CASH/1s")));
    assert!(b.path.starts_with(root.0.join("verify/gdfl/BSE/CASH/1s")));

    let mut reader = Reader::new();
    let first = committed_day(&nse).unwrap();
    let second = committed_day(&bse).unwrap();
    assert_eq!(first.worst_code(), Some(Code::ValueMismatch));
    assert_eq!(first.state, DayState::Stopped.code());
    assert_eq!((first.records, first.mismatches), (1, 1));
    assert_eq!(second.worst_code(), Some(Code::Match));
    assert_eq!(second.state, DayState::Complete.code());
    assert_eq!((second.records, second.mismatches), (1, 0));
    assert_eq!(
        verdict::read_image(&a.path).unwrap().header.exchange,
        Exchange::Nse
    );
    assert_eq!(
        verdict::read_image(&b.path).unwrap().header.exchange,
        Exchange::Bse
    );
    // A key of one exchange has no record on the other.
    let key = Key::instrument("NSE-CASH-ZZTWOEX").unwrap();
    assert!(matches!(
        reader
            .lookup(&nse, day(), &key)
            .unwrap()
            .answer
            .unwrap()
            .found,
        Found::Record { .. }
    ));
    assert!(matches!(
        reader
            .lookup(&bse, day(), &key)
            .unwrap()
            .answer
            .unwrap()
            .found,
        Found::NoRecord(_)
    ));
    // The NSE image put where BSE's generation 2 would be is refused: the
    // header names its own exchange, and the path is not believed over it.
    let planted = bse.image_path(day(), 1);
    fs::remove_file(&planted).unwrap();
    fs::copy(&a.path, &planted).unwrap();
    let refused = Reader::new().lookup(&bse, day(), &key).unwrap_err();
    assert_eq!(refused.word(), "image_corrupt", "{refused}");
}

/// RA-243, RA-255: nothing in the verdict store lists the feeds or the
/// timeframes. Every vendor of `Vendor::ALL` and every rung of
/// `Timeframe::KNOWN` files a verdict and reads it back with its own
/// identity, and a timeframe the running build does not know is refused by
/// name, never read as another.
#[test]
fn every_feed_and_timeframe_the_store_knows_files_a_verdict() {
    let root = Root::new();
    let key = Key::instrument("NSE-INDEX-ZZEVERY").unwrap();
    let mut last = None;
    for feed in Vendor::ALL {
        for timeframe in Timeframe::KNOWN {
            let series = Series {
                root: &root.0,
                feed,
                exchange: Exchange::Nse,
                segment: Segment::Index,
                timeframe: *timeframe,
            };
            let done = verdict::commit(
                &series,
                &generation(77, vec![entry("NSE-INDEX-ZZEVERY", Code::Match)]),
            )
            .unwrap();
            let header = verdict::read_image(&done.path).unwrap().header;
            assert_eq!(
                (header.feed, header.timeframe, header.unit),
                (feed, *timeframe, verdict::unit_of(*timeframe)),
                "{} {}",
                feed.as_str(),
                timeframe.as_str()
            );
            let got = Reader::new().lookup(&series, day(), &key).unwrap();
            assert_eq!(found(&got.answer.unwrap()).facts.code, Code::Match);
            assert_eq!((got.cost.opens, got.cost.reads), (2, 5));
            last = Some(done.path);
        }
    }
    // 7 seconds is no rung of this build: an image that says so was written
    // by a newer one.
    let path = last.unwrap();
    let mut bytes = fs::read(&path).unwrap();
    bytes[184..188].copy_from_slice(&7u32.to_le_bytes());
    let crc = store::crc::crc32c(&bytes[..252]);
    bytes[252..256].copy_from_slice(&crc.to_le_bytes());
    let end = bytes.len() - 32;
    let trailer = brutex_core::blake3::hash(&bytes[..end]);
    bytes[end..].copy_from_slice(&trailer);
    let strange = root.0.join("strange.vdt");
    fs::write(&strange, &bytes).unwrap();
    let refused = verdict::read_image(&strange).unwrap_err();
    assert_eq!(refused.word(), "written_by_newer_build", "{refused}");
    assert!(
        refused.to_string().contains("timeframe seconds 7"),
        "{refused}"
    );
}

/// The temporary a writer of this process and run would use beside `path`.
fn tmp_beside(path: &Path, run: u64) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(format!(".{}-{run}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// RA-290: a hard link planted at a writer's temporary name is never
/// opened. The commit is refused `staging_linked`, the file behind the link
/// keeps every byte, nothing is published, and once the name is gone the
/// same commit is generation 1. At the year index's temporary the same
/// holds and no index is born. A dead writer's own leftover (a regular
/// file with one name) is cleared and counted, never written through.
#[test]
fn a_hard_link_at_a_staging_name_is_refused_and_the_file_behind_it_is_untouched() {
    let root = Root::new();
    let series = root.series();
    let run = 4_242u64;
    let body = b"a month of bars that must survive".to_vec();
    let victim = root.0.join("victim.bin");
    fs::write(&victim, &body).unwrap();
    let g = generation(run, day_of(9));

    // 1. At the year index's temporary: refused before any index exists.
    let year_dir = series.dir();
    fs::create_dir_all(&year_dir).unwrap();
    let index_tmp = tmp_beside(&series.year_index(2031), run);
    fs::hard_link(&victim, &index_tmp).unwrap();
    let refused = verdict::commit(&series, &g).unwrap_err();
    assert_eq!(refused.word(), "staging_linked", "{refused}");
    assert!(refused.to_string().contains("one of 2 names"), "{refused}");
    assert_eq!(fs::read(&victim).unwrap(), body, "not one byte changed");
    assert!(!series.year_index(2031).exists(), "no index was born");
    assert!(
        fs::symlink_metadata(&index_tmp).is_ok(),
        "the name is left to be seen"
    );
    fs::remove_file(&index_tmp).unwrap();

    // 2. At the image's temporary: refused, nothing published.
    let image = series.image_path(day(), 1);
    fs::create_dir_all(image.parent().unwrap()).unwrap();
    let image_tmp = tmp_beside(&image, run);
    fs::hard_link(&victim, &image_tmp).unwrap();
    let refused = verdict::commit(&series, &g).unwrap_err();
    assert!(
        matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
        "{refused}"
    );
    assert_eq!(fs::read(&victim).unwrap(), body, "not one byte changed");
    assert!(!image.exists(), "no image was linked");
    assert_eq!(
        committed_day(&series),
        None,
        "the day has no committed generation"
    );
    fs::remove_file(&image_tmp).unwrap();

    // 3. A symbolic link at the name is refused the same way.
    std::os::unix::fs::symlink(&victim, &image_tmp).unwrap();
    let refused = verdict::commit(&series, &g).unwrap_err();
    assert_eq!(refused.word(), "staging_not_regular", "{refused}");
    assert_eq!(fs::read(&victim).unwrap(), body);
    fs::remove_file(&image_tmp).unwrap();

    // 4. A dead writer's leftover: cleared, counted, and the day commits.
    fs::write(&image_tmp, b"half an image").unwrap();
    let done = verdict::commit(&series, &g).unwrap();
    assert_eq!(done.generation, 1);
    assert_eq!(done.staging_cleared, 1, "the leftover is reported");
    assert!(!image_tmp.exists(), "the temporary is gone after the link");
    assert_eq!(fs::read(&victim).unwrap(), body);
    assert_eq!(verdict::read_image(&done.path).unwrap().records.len(), 9);

    // 5. The commit lock file is shared, never a temporary: it is opened
    // without truncation and never written, so even a hard link at ITS name
    // costs the file behind it nothing.
    let lock_path = series.lock_path(2031);
    fs::remove_file(&lock_path).unwrap();
    fs::hard_link(&victim, &lock_path).unwrap();
    let again = Generation {
        run: run + 1,
        ..g.clone()
    };
    assert_eq!(verdict::commit(&series, &again).unwrap().generation, 2);
    assert_eq!(fs::read(&victim).unwrap(), body, "not one byte changed");
}

/// D-4924 (the critic's D-4906 for this lock): a commit lock held past the
/// wait is a refusal by name, never a writer that waits for ever. Nothing
/// is published while it is held, and the same commit goes through once it
/// is free.
#[test]
fn a_commit_lock_held_past_the_wait_is_refused_by_name() {
    let root = Root::new();
    let series = root.series();
    let g = generation(55, day_of(5));
    verdict::commit(&series, &g).unwrap();

    let lock_path = series.lock_path(2031);
    let holder = fs::File::options()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    holder.lock().unwrap();
    let again = Generation {
        run: 56,
        ..g.clone()
    };
    let started = std::time::Instant::now();
    let refused = verdict::commit_within(&series, &again, Duration::from_millis(60)).unwrap_err();
    let waited = started.elapsed();
    match &refused {
        VerdictError::LockStalled {
            path,
            waited_ms,
            tries,
        } => {
            assert_eq!(path, &lock_path);
            assert_eq!(*waited_ms, 60);
            assert_eq!(*tries, 7, "one try, and one more per 10 ms step of 60 ms");
        }
        other => panic!("expected lock_stalled, got {other:?}"),
    }
    assert_eq!(refused.word(), "lock_stalled");
    assert!(
        waited >= Duration::from_millis(60),
        "it waited the whole wait"
    );
    assert!(
        waited < Duration::from_secs(20),
        "and no longer: {waited:?}"
    );
    assert_eq!(
        committed_day(&series).unwrap().generation,
        1,
        "nothing was published under the held lock"
    );
    assert!(!series.image_path(day(), 2).exists());
    // No wait at all is one try.
    let once = verdict::commit_within(&series, &again, Duration::ZERO).unwrap_err();
    assert!(
        matches!(
            &once,
            VerdictError::LockStalled {
                tries: 1,
                waited_ms: 0,
                ..
            }
        ),
        "{once}"
    );
    assert!(once.to_string().contains("1 tries over 0 ms"), "{once}");
    assert_eq!(verdict::LOCK_WAIT, Duration::from_secs(30));

    holder.unlock().unwrap();
    let done = verdict::commit_within(&series, &again, Duration::from_millis(60)).unwrap();
    assert_eq!(done.generation, 2);
}

/// A deterministic 64-bit generator (splitmix64) for the fit test: the
/// table's rule reads a key only through its 64-bit hash, so a random key
/// set is a set of random hashes.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// How many of `sets` random key sets of `n` keys the table places, and the
/// largest capacity any of them needed.
fn fit(n: usize, sets: usize, seed: u64) -> (usize, usize) {
    let mut state = seed;
    let mut placed = 0usize;
    let mut largest = 0usize;
    for _ in 0..sets {
        let hashes: Vec<u64> = (0..n).map(|_| splitmix(&mut state)).collect();
        if let Ok((capacity, _)) = verdict::table(&hashes) {
            placed += 1;
            largest = largest.max(capacity);
        }
    }
    (placed, largest)
}

/// RA-261: whether a day fits depends on its keys, so one made-up key set
/// per size proved little. Over 1,000 random key sets at the largest real
/// day's size (26,820 records) every set is placed with a doubling to
/// spare, and every one of 1,000 sets at the watch level (68,950, half the
/// measured ceiling) is placed too: `near_ceiling` fires while every day
/// still fits.
///
/// The ceiling itself is a band, not a number, and the last two lines pin
/// what was measured so nobody reads 137,900 as a promise: at 137,900
/// records 184 of 200 random key sets fit (16 are refused
/// `table_too_large`), and at 206,850 (1.5x) 16 of 200 fit. The seeds are
/// fixed, so the counts are exact; a change to the placement rule moves
/// them and must be measured again (L-IA-33).
#[test]
fn a_days_keys_fit_whatever_the_keys_are() {
    let (placed, largest) = fit(26_820, 1_000, 0x00C0_FFEE);
    assert_eq!(placed, 1_000, "every key set of the largest real day fits");
    assert!(
        largest <= verdict::MAX_CAPACITY / 2,
        "with at least one doubling to spare: the largest table was {largest} slots"
    );
    let watch = usize::try_from(verdict::RECORDS_WATCH).unwrap();
    assert_eq!(watch, 68_950);
    let (placed, largest) = fit(watch, 1_000, 0x5EED_CE13);
    assert_eq!(placed, 1_000, "every key set at the watch level fits");
    assert!(largest <= verdict::MAX_CAPACITY);
    let top = usize::try_from(verdict::RECORDS_PLACED).unwrap();
    assert_eq!(top, 137_900);
    let (placed_top, _) = fit(top, 200, 0x5EED_CE11);
    let (placed_over, _) = fit(top * 3 / 2, 200, 0x5EED_CE12);
    assert_eq!(
        (placed_top, placed_over),
        (184, 16),
        "the ceiling is a band: measured fits of 200 random key sets at 137,900 and at 206,850"
    );
}

/// The year, the day's key and the name of the lost index's refusal.
fn index_missing(refused: &VerdictError) -> (bool, u32) {
    assert_eq!(refused.word(), "index_missing", "{refused}");
    match refused {
        VerdictError::IndexMissing { path, image, seen } => {
            assert!(path.ends_with("2031.vdi"), "{}", path.display());
            if let Some(image) = image {
                assert!(
                    image.extension().is_some_and(|ext| ext == "vdt")
                        && image.parent().is_some_and(|dir| dir.ends_with("2031"))
                        && image.exists(),
                    "the refusal names an image that is there: {}",
                    image.display()
                );
            }
            (image.is_some(), *seen)
        }
        other => panic!("expected index_missing, got {other}"),
    }
}

/// A lost year index is never born again empty (CLAUDE.md section 4: a
/// fallback never hides a failure). With `2031.vdi` gone and `2031/` still
/// holding committed images, every reader call refuses `index_missing`
/// naming an image, where it used to answer "no generation"; and a commit
/// refuses before it builds, bears or links anything, where it used to
/// start the year again at generation 1 and carry nothing. The images are
/// not touched, so putting the index back restores every answer. A year
/// directory that holds no image is still a year in the making: its first
/// commit bears the index.
#[test]
fn a_lost_year_index_is_refused_and_never_born_again_empty() {
    let root = Root::new();
    let series = root.series();
    let next_day = Day::new(2031, 3, 5).unwrap();
    let third_day = Day::new(2031, 3, 6).unwrap();
    let bad = Key::instrument("NSE-FNO-ZZLOST-A").unwrap();
    verdict::commit(
        &series,
        &generation(401, vec![entry("NSE-FNO-ZZLOST-A", Code::ValueMismatch)]),
    )
    .unwrap();
    let mut second = generation(402, vec![entry("NSE-FNO-ZZLOST-B", Code::Match)]);
    second.day = next_day;
    verdict::commit(&series, &second).unwrap();

    let index = series.year_index(2031);
    let aside = index.with_extension("vdi.lost");
    fs::rename(&index, &aside).unwrap();
    let year_dir = series.dir().join("2031");
    let images = || {
        let mut names: Vec<String> = fs::read_dir(&year_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let before = images();
    assert_eq!(
        before,
        ["2031-03-04.g0000000001.vdt", "2031-03-05.g0000000001.vdt"]
    );

    // The reader: every call that reads the year refuses by name.
    let mut reader = Reader::new();
    let (named, seen) = index_missing(&reader.lookup(&series, day(), &bad).unwrap_err());
    assert!(named && (1..=2).contains(&seen), "seen {seen}");
    index_missing(&reader.lookup(&series, third_day, &bad).unwrap_err());
    index_missing(&reader.year(&series, 2031).unwrap_err());
    index_missing(
        &reader
            .page(&series, day(), Code::ValueMismatch, 0, 10)
            .unwrap_err(),
    );
    // A year the series never wrote is still "no generation", at no open.
    let other_year = reader
        .lookup(&series, Day::new(2032, 3, 4).unwrap(), &bad)
        .unwrap();
    assert_eq!((other_year.answer, other_year.cost.opens), (None, 0));
    assert_eq!(reader.year(&series, 2032).unwrap(), None);

    // The writer: a whole day of a day never written, and a carrying
    // generation of a committed day, are both refused with nothing born.
    let mut third = generation(403, vec![entry("NSE-FNO-ZZLOST-C", Code::Match)]);
    third.day = third_day;
    index_missing(&verdict::commit(&series, &third).unwrap_err());
    let mut carrying = generation(404, vec![entry("NSE-FNO-ZZLOST-D", Code::Match)]);
    carrying.carry = Carry::Others;
    index_missing(&verdict::commit(&series, &carrying).unwrap_err());
    assert!(!index.exists(), "no empty index was born over the images");
    assert_eq!(images(), before, "no image was linked and none was changed");

    // The index back in place: the committed records answer as before, and
    // a carrying generation carries them.
    fs::rename(&aside, &index).unwrap();
    let mut reader = Reader::new();
    let held = reader.lookup(&series, day(), &bad).unwrap();
    assert_eq!(held.answer.unwrap().code(), Some(Code::ValueMismatch));
    let carried = verdict::commit(&series, &carrying).unwrap();
    assert_eq!(
        (carried.generation, carried.previous, carried.carried),
        (2, 1, 1)
    );

    // A year directory with no image is a year in the making, whatever
    // else it holds up to the scan's bound: the first commit bears the
    // index. One name past the bound and no image can be ruled out, so the
    // year is refused out loud and not listed to its end.
    let fresh = Series {
        segment: Segment::Cash,
        ..root.series()
    };
    let fresh_dir = fresh.dir().join("2031");
    fs::create_dir_all(&fresh_dir).unwrap();
    for i in 0..ORPHAN_SCAN {
        fs::write(fresh_dir.join(format!("zz-left-{i:04}.tmp")), b"").unwrap();
    }
    let mut fresh_reader = Reader::new();
    assert_eq!(fresh_reader.year(&fresh, 2031).unwrap(), None);
    let extra = fresh_dir.join("zz-left-over.tmp");
    fs::write(&extra, b"").unwrap();
    let crowded = verdict::commit(
        &fresh,
        &generation(405, vec![entry("NSE-CASH-ZZLOST-E", Code::Match)]),
    )
    .unwrap_err();
    assert_eq!(
        index_missing(&crowded),
        (false, u32::try_from(ORPHAN_SCAN).unwrap() + 1)
    );
    index_missing(&fresh_reader.year(&fresh, 2031).unwrap_err());
    assert!(!fresh.year_index(2031).exists());
    fs::remove_file(&extra).unwrap();
    let born = verdict::commit(
        &fresh,
        &generation(406, vec![entry("NSE-CASH-ZZLOST-E", Code::Match)]),
    )
    .unwrap();
    assert_eq!((born.generation, born.commit), (1, 1));
}

/// The details of a record are asked by its ordinal: one the generation
/// does not hold is refused `ordinal_past_records` (at `u32::MAX` the owner
/// used to overflow: a panic in a debug build and the day's owner 0 in a
/// build without overflow checks; any other answered an empty list), and a
/// record that is not
/// the one at that ordinal is refused `image_corrupt`, never answered with
/// nothing.
#[test]
fn the_details_of_an_ordinal_the_day_does_not_hold_are_refused_by_name() {
    let root = Root::new();
    let series = root.series();
    let mut g = generation(411, day_of(4));
    g.entries[1].details = vec![Detail {
        kind: 6,
        payload: b"ZZ/own/detail".to_vec(),
    }];
    g.day_details = vec![Detail {
        kind: 7,
        payload: b"ZZ/day/detail".to_vec(),
    }];
    let done = verdict::commit(&series, &g).unwrap();
    let image = verdict::read_image(&done.path).unwrap();
    let (at, with) = image
        .records
        .iter()
        .enumerate()
        .find(|(_, (r, _))| r.detail_at != 0)
        .map(|(i, (r, _))| (u32::try_from(i).unwrap(), r.clone()))
        .unwrap();
    let mut reader = Reader::new();
    assert_eq!(
        reader
            .details(&series, day(), 1, Some((at, &with)))
            .unwrap(),
        g.entries[1].details
    );
    for ordinal in [4, 5, u32::MAX - 1, u32::MAX] {
        let refused = reader
            .details(&series, day(), 1, Some((ordinal, &with)))
            .unwrap_err();
        assert_eq!(
            refused.word(),
            "ordinal_past_records",
            "{ordinal}: {refused}"
        );
        match refused {
            VerdictError::OrdinalPastRecords {
                ordinal: asked,
                records,
                ..
            } => assert_eq!((asked, records), (ordinal, 4)),
            other => panic!("{other}"),
        }
    }
    // The same record under another ordinal of the day: not its own entry.
    let other = (at + 1) % 4;
    let refused = reader
        .details(&series, day(), 1, Some((other, &with)))
        .unwrap_err();
    assert_eq!(refused.word(), "image_corrupt", "{refused}");
    // An offset past the section is refused too, never read.
    let mut past = with.clone();
    past.detail_at = u32::MAX - 1;
    let refused = reader
        .details(&series, day(), 1, Some((at, &past)))
        .unwrap_err();
    assert_eq!(refused.word(), "image_corrupt", "{refused}");
    // The day's own details and a record with none answer as before.
    assert_eq!(
        reader.details(&series, day(), 1, None).unwrap(),
        g.day_details
    );
    let (none_at, none) = image
        .records
        .iter()
        .enumerate()
        .find(|(_, (r, _))| r.detail_at == 0)
        .map(|(i, (r, _))| (u32::try_from(i).unwrap(), r.clone()))
        .unwrap();
    assert_eq!(
        reader
            .details(&series, day(), 1, Some((none_at, &none)))
            .unwrap(),
        Vec::new()
    );
}

/// A file longer than any day image can be is refused by its length alone,
/// before a byte is read or allocated for: a sparse file of the ceiling
/// plus one byte is named `image_corrupt` with both lengths, and one byte
/// shorter is read and refused for what it holds.
#[test]
fn a_file_longer_than_any_day_image_is_refused_unread() {
    let root = Root::new();
    let path = root.0.join("zz-too-long.vdt");
    let file = fs::File::create(&path).unwrap();
    assert_eq!(verdict::MAX_IMAGE_LEN, 180_355_672, "section 35.9's number");
    let ceiling = u64::try_from(verdict::MAX_IMAGE_LEN).unwrap();
    file.set_len(ceiling + 1).unwrap();
    let refused = verdict::check_trailer(&path).unwrap_err();
    assert_eq!(refused.word(), "image_corrupt");
    let said = refused.to_string();
    assert!(
        said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
        "{said}"
    );
    file.set_len(1_024).unwrap();
    let refused = verdict::read_image(&path).unwrap_err();
    assert!(!refused.to_string().contains("not read"), "{refused}");
}
