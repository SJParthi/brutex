//! `store::verdict` through real files: IA-VD-01 to IA-VD-13 of the
//! ingest-automation design (D-4808, D-4809, D-4844, D-4845, D-4865).
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

use brutex_core::instrument::Segment;
use brutex_core::vendor::Vendor;
use store::path::Timeframe;
use store::provenance::Day;
use store::verdict::{
    self, Carry, Check, Code, Committed, DETAIL_CAP, DETAIL_NAME, DayBlock, DayState, Detail,
    Entry, Facts, Feed, Found, Generation, INDEX_LEN, JournalOutcome, KEY_LEN, Key, MAX_LINK_TRIES,
    Mode, MonthNow, NONE, Now, RECORD_STRIDE, Reader, Series, SourceHeld, Stale, Unit, V3Manifest,
    VerdictError, flags,
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
        Found::DayBlock => panic!("expected a record, got the day block"),
    }
}

fn home(hash: u64, capacity: u32) -> u64 {
    hash & u64::from(capacity - 1)
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

        // An absent key: the day block answers, in at most 5 reads.
        let absent = Key::instrument("NSE-FNO-ZZABSENT-1").unwrap();
        let mut reader = Reader::new();
        let got = reader.lookup(&series, day(), &absent).unwrap();
        assert!(
            got.cost.reads <= 5 && got.cost.opens <= 2,
            "n={n}: {:?}",
            got.cost
        );
        let answer = got.answer.unwrap();
        assert_eq!(answer.found, Found::DayBlock);
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
            assert_eq!(got.answer.unwrap().found, Found::DayBlock);
        }
        assert_eq!(verdict::scrub(&done.path).unwrap().records, done.records);
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
    assert_eq!(answer.code(), Code::ValueMismatch);
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
    assert_eq!(answer.code(), Code::Match);
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
    let kept = Key::stem(&accented).unwrap();
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
        .with_file_name("2031-03-04.g000002.vdt.99999-82.tmp");
    fs::copy(&decoy.path, &stray).unwrap();
    let stray_bytes = fs::read(&stray).unwrap();

    let key = Key::instrument("NSE-FNO-ZZTMP-A").unwrap();
    let mut reader = Reader::new();
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(answer.code(), Code::Match);
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
        Code::NotCompared
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
            verdict::scrub(&path).unwrap().records,
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
        Code::VendorHole
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
/// verifier is older, whose day's definition moved, whose cross-kind input is
/// pending, or whose month changed reads stale; a month with no `.tix` reads
/// `stale: unknown`; a check not made is named and the answer is not fresh.
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
        definition_moved: Some(false),
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
    let appended = a.staleness(&Now {
        month: MonthNow::Read {
            generation: 2,
            first_row: 4_000,
            rows: 22_500,
        },
        ..now
    });
    assert_eq!(
        appended.stale,
        vec![Stale::StoreChanged {
            verified: (2, 4_000, 375),
            now: (2, 4_000, 22_500)
        }],
        "a resume appended the rest of the day"
    );
    let repaired = a.staleness(&Now {
        month: MonthNow::Read {
            generation: 3,
            first_row: 4_000,
            rows: 375,
        },
        ..now
    });
    assert!(
        matches!(repaired.stale[..], [Stale::StoreChanged { .. }]),
        "a repair published a generation"
    );
    let moved = a.staleness(&Now {
        definition_moved: Some(true),
        ..now
    });
    assert_eq!(moved.stale, vec![Stale::Definition]);
    let no_tix = a.staleness(&Now {
        month: MonthNow::NoTix,
        ..now
    });
    assert_eq!(no_tix.stale, vec![Stale::StoreUnknown]);
    assert!(!no_tix.is_fresh());
    let unchecked = a.staleness(&Now {
        verifier: 3,
        definition_moved: None,
        month: MonthNow::NotChecked,
    });
    assert!(unchecked.stale.is_empty());
    assert_eq!(unchecked.unchecked, vec![Check::Definition, Check::Month]);
    assert!(!unchecked.is_fresh(), "an unchecked verdict is never fresh");

    let b = reader
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZSTALE-B").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    let b_now = Now {
        month: MonthNow::Read {
            generation: 0,
            first_row: NONE,
            rows: 0,
        },
        ..now
    };
    assert_eq!(b.staleness(&b_now).stale, vec![Stale::CrossKindPending]);

    let none = reader
        .lookup(
            &series,
            day(),
            &Key::instrument("NSE-FNO-ZZSTALE-Z").unwrap(),
        )
        .unwrap()
        .answer
        .unwrap();
    assert_eq!(none.found, Found::DayBlock);
    assert_eq!(
        none.staleness(&now).unchecked,
        vec![Check::Month],
        "a day-block answer has no row range to compare"
    );
}

/// IA-VD-12 — the detail section round-trips every kind, and past 4 MiB the
/// day says `details_capped`: the owners that did not fit keep their counts
/// only and carry the flag.
#[test]
fn every_detail_survives_and_the_cap_is_named() {
    let root = Root::new();
    let series = root.series();
    let all_kinds: Vec<Detail> = (1..=15u16)
        .map(|kind| Detail {
            kind,
            payload: (0..usize::from(kind) * 3)
                .map(|b| u8::try_from(b).unwrap())
                .collect(),
        })
        .collect();
    let mut g = generation(151, day_of(3));
    g.entries[1].details = all_kinds.clone();
    g.day_details = vec![Detail {
        kind: 6,
        payload: b"ZZ/first/path".to_vec(),
    }];
    let done = verdict::commit(&series, &g).unwrap();
    assert!(!done.details_capped);
    let image = verdict::read_image(&done.path).unwrap();
    assert_eq!(image.day_details, g.day_details);
    let (record, details) = image
        .records
        .iter()
        .find(|(r, _)| r.key.text() == name(1))
        .unwrap();
    assert_eq!(details, &all_kinds);
    let mut reader = Reader::new();
    let ordinal = u32::try_from(
        image
            .records
            .iter()
            .position(|(r, _)| r.key.text() == name(1))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        reader
            .details(&series, day(), 1, Some((ordinal, record)))
            .unwrap(),
        all_kinds
    );
    assert_eq!(
        reader.details(&series, day(), 1, None).unwrap(),
        g.day_details
    );

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
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_ne!(year.entry(day()).flags & flags::DETAILS_CAPPED, 0);
}

/// IA-VD-13 — every flag bit and every parameter of §6.4 is held by format
/// version 1: a record with all 20 flags and every field set, the day block
/// with every field set, and every detail kind read back whole.
#[test]
fn format_v1_holds_every_fact_it_names() {
    assert_eq!(flags::ALL.len(), 20);
    let mut union = 0u64;
    for (i, (bit, word)) in flags::ALL.iter().enumerate() {
        assert_eq!(*bit, 1u64 << i, "{word} is bit {i}");
        union |= bit;
    }
    assert_eq!(union, flags::KNOWN);
    assert_eq!(
        flags::words(flags::NAME_CUT | flags::TIX_ABSENT),
        vec!["tix_absent", "name_cut"]
    );

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
    let details: Vec<Detail> = (1..=15u16)
        .map(|kind| Detail {
            kind,
            payload: vec![u8::try_from(kind).unwrap(); 240],
        })
        .collect();
    let day_block = DayBlock {
        calendar_kind: 9,
        source_held: SourceHeld::Yes,
        v3: V3Manifest::Disagrees,
        journal: JournalOutcome::DoneWithRefusals,
        state: DayState::Complete,
        worst: Code::Match,
        close_reason: 0xBEEF,
        flags: flags::ARCHIVE_DAY_ON_CLOSED_DAY,
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
        (h.feed, h.segment, h.timeframe, h.unit),
        (Feed::Gdfl, Segment::Fno, Timeframe::SECOND_1, Unit::Second)
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

    // The refusals of inputs version 1 cannot hold.
    let mut bad = generation(u64::from(u32::MAX) + 1, Vec::new());
    assert_eq!(
        verdict::commit(&series, &bad).unwrap_err().word(),
        "run_id_overflow"
    );
    bad.run = 5;
    bad.entries = vec![entry("NSE-FNO-ZZALL-B", Code::Match)];
    bad.entries[0].facts.flags = 1 << 20;
    match verdict::commit(&series, &bad) {
        Err(VerdictError::FlagUnknown { bits }) => assert_eq!(bits, 1 << 20),
        other => panic!("expected flag_unknown, got {other:?}"),
    }
    bad.entries[0].facts.flags = 0;
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

/// The codes, their severities and the mismatch set of §6.4, pinned.
#[test]
fn the_codes_and_their_severity_are_the_designs() {
    let mut severities: Vec<u8> = Code::ALL.iter().map(|c| c.severity()).collect();
    severities.sort_unstable();
    assert_eq!(severities, (1..=17).collect::<Vec<u8>>(), "a total order");
    let by_severity: Vec<u8> = {
        let mut all = Code::ALL.to_vec();
        all.sort_by_key(|c| c.severity());
        all.iter().map(|c| c.code()).collect()
    };
    assert_eq!(
        by_severity,
        vec![9, 2, 3, 4, 7, 8, 5, 16, 6, 12, 13, 14, 17, 15, 10, 11, 1]
    );
    let mismatches: Vec<u8> = Code::ALL
        .iter()
        .filter(|c| c.is_mismatch())
        .map(|c| c.code())
        .collect();
    assert_eq!(mismatches, vec![2, 3, 4, 5, 7, 8, 9]);
    for code in Code::ALL {
        assert_eq!(Code::from_code(code.code()), Some(code));
        assert_eq!(code.worse(Code::Match), code);
        assert_eq!(Code::StoreUnreadable.worse(code), Code::StoreUnreadable);
    }
    assert_eq!(Code::from_code(0), None, "0 is unset, never a written code");
    assert_eq!(Code::from_code(18), None);
    assert_eq!(Code::DayPartial.word(), "day_partial");
}

/// The feed, segment, timeframe and unit bytes, pinned to their sources: a
/// feed's code is its position in `Vendor::ALL` plus one.
#[test]
fn the_series_bytes_are_pinned() {
    for feed in Feed::ALL {
        let at = Vendor::ALL
            .iter()
            .position(|v| *v == feed.vendor())
            .unwrap();
        assert_eq!(usize::from(feed.code()), at + 1);
        assert_eq!(Feed::from_code(feed.code()), Some(feed));
        assert_eq!(Feed::of(feed.vendor()), Some(feed));
        assert_eq!(feed.as_str(), feed.vendor().as_str());
    }
    assert_eq!(
        Feed::of(Vendor::Dhan),
        None,
        "only the feeds the operator uses"
    );
    for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
        assert_eq!(
            verdict::segment_of(verdict::segment_code(segment)),
            Some(segment)
        );
    }
    let mut codes: Vec<u8> = Timeframe::KNOWN
        .iter()
        .map(|tf| verdict::timeframe_code(*tf).unwrap())
        .collect();
    for tf in Timeframe::KNOWN {
        assert_eq!(
            verdict::timeframe_of(verdict::timeframe_code(*tf).unwrap()),
            Some(*tf)
        );
    }
    codes.sort_unstable();
    assert_eq!(codes, (1..=10).collect::<Vec<u8>>());
    assert_eq!(verdict::unit_of(Timeframe::SECOND_1), Unit::Second);
    assert_eq!(verdict::unit_of(Timeframe::MINUTE_15), Unit::Minute);
    assert_eq!(verdict::unit_of(Timeframe::DAY_1), Unit::Day);
    let root = Root::new();
    let series = Series {
        root: &root.0,
        feed: Feed::Zerodha,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
    };
    assert_eq!(
        series.image_path(day(), 7),
        root.0
            .join("verify/zerodha/INDEX/1min/2031/2031-03-04.g000007.vdt")
    );
    assert_eq!(
        series.year_index(2031),
        root.0.join("verify/zerodha/INDEX/1min/2031.vdi")
    );
    assert_eq!(
        series.lock_path(2031),
        root.0.join("verify/zerodha/INDEX/1min/2031.vdi.lock")
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
    assert_eq!(&b[12..16], &[4, 3, 1, 0], "gdfl, FNO, 1s, seconds");
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
/// the day block, whose worst is at least `day_partial`.
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
        Code::Match,
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
    assert_eq!(unreached.found, Found::DayBlock);
    assert_eq!(unreached.code(), Code::DayPartial);
    assert_eq!(unreached.day.not_reached, 9);
    let year = reader.year(&series, 2031).unwrap().unwrap();
    assert_eq!(year.entry(day()).worst_code(), Some(Code::DayPartial));
}

/// The scrub reads the trailer and every check; a flipped byte anywhere is
/// refused by name, and a lookup that meets a flipped record refuses it.
#[test]
fn a_flipped_byte_is_refused_by_the_scrub_and_the_lookup() {
    let root = Root::new();
    let series = root.series();
    let done = verdict::commit(&series, &generation(171, day_of(30))).unwrap();
    let clean = fs::read(&done.path).unwrap();
    assert_eq!(verdict::scrub(&done.path).unwrap().records, 30);
    let image = verdict::read_image(&done.path).unwrap();
    let records_at = 512 + 8 * (usize::try_from(image.header.capacity).unwrap() + 7);
    for at in [3usize, 300, 520, records_at + 160 * 4 + 90, clean.len() - 1] {
        let mut bytes = clean.clone();
        bytes[at] ^= 0x10;
        fs::write(&done.path, &bytes).unwrap();
        let refused = verdict::scrub(&done.path).unwrap_err();
        assert!(
            ["image_corrupt", "record_corrupt"].contains(&refused.word()),
            "byte {at}: {refused}"
        );
    }
    let mut bytes = clean.clone();
    bytes[records_at + 160 * 4 + 90] ^= 0x10;
    fs::write(&done.path, &bytes).unwrap();
    let flipped = image.records[4].0.key.clone();
    let mut reader = Reader::new();
    match reader.lookup(&series, day(), &flipped) {
        Err(VerdictError::RecordCorrupt { ordinal, .. }) => assert_eq!(ordinal, 4),
        other => panic!("expected record_corrupt, got {other:?}"),
    }
    fs::write(&done.path, &clean).unwrap();
    assert!(
        reader
            .lookup(&series, day(), &flipped)
            .unwrap()
            .answer
            .is_some()
    );
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
        "`store::verdict::scrub`",
        "D-4808",
        "D-4844",
        "D-4865",
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
