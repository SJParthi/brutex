#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
//! BASE-COV-PULL-PS1 (D-7335): what `fno_membership` does that Route B on
//! `2e7a0b6d3b56` found no test reading: a closed day the source refuses, a
//! reader thread that dies, the fold at the spelling bound and on three
//! handover sides, every check of one file's bytes no test had failed, the
//! generation directory's and the writer's own refusals, the chain's last
//! checks, the windows a build refuses, a resume after a session with no
//! file, and what a month's report carries. Every share is invented
//! (`ZQ...`); the one index ticker is the existing fixtures' `NIFTY` at a
//! strike (99,999.5) no contract ever traded at.

use super::*;
use crate::gdfl_cm::{CmRefusal, ListedFile};
use crate::gdfl_fixtures::scratch;
use crate::gdfl_nfo::{NfoDay, day_folder_name};
use crate::session::MAX_DAY_NUMBER;

fn d(year: u16, month: u8, day: u8) -> Day {
    Day::new(year, month, day).unwrap()
}

fn symbol(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

/// Three contract keys of the March 2024 expiry under `share`.
fn keys(share: &str) -> Vec<String> {
    ["28MAR24500CE", "28MAR24520CE", "28MAR24480PE"]
        .iter()
        .map(|key| format!("{share}{key}"))
        .collect()
}

/// `day` listing `tickers`, decoded.
fn listed(day: Day, tickers: &[String]) -> DayInput {
    DayInput::Listed(evidence(day, tickers.iter().map(String::as_str)))
}

/// The `n`th invented share: `ZQ` and four letters, one name per `n` below
/// 26 to the 4th.
fn invented(n: usize) -> String {
    let letter =
        |place: u32| char::from(b'A' + u8::try_from(n / 26_usize.pow(place) % 26).unwrap());
    format!("ZQ{}{}{}{}", letter(3), letter(2), letter(1), letter(0))
}

/// [`MAX_SPELLINGS`] invented shares, one March 2024 key each.
fn the_format_bound_of_shares() -> Vec<String> {
    (0..MAX_SPELLINGS)
        .map(|n| format!("{}28MAR24500CE", invented(n)))
        .collect()
}

/// `inputs` folded in order from an empty state into one month table begun
/// at the first of them; the state after the last.
fn month_of(inputs: Vec<(Day, DayInput)>) -> (MonthTable, State) {
    let mut state = State::default();
    let mut table: Option<MonthTable> = None;
    for (day, input) in inputs {
        let month = table.get_or_insert_with(|| MonthTable::begin(day, &state));
        let record = state.fold(day, input).unwrap();
        month.push(record, &state).unwrap();
    }
    (table.unwrap(), state)
}

/// Fri 1 Mar to Tue 5 Mar 2024: a known session, a weekend, a known session
/// that adds ZQBETA, and a session with no file.
fn five_days() -> MonthTable {
    let (friday, monday) = (d(2024, 3, 1), d(2024, 3, 4));
    let closed = || DayInput::Unknown(DayStatus::NotSession, None);
    month_of(vec![
        (friday, listed(friday, &keys("ZQALPHA"))),
        (d(2024, 3, 2), closed()),
        (d(2024, 3, 3), closed()),
        (
            monday,
            listed(monday, &[keys("ZQALPHA"), keys("ZQBETA")].concat()),
        ),
        (d(2024, 3, 5), DayInput::Unknown(DayStatus::NoSource, None)),
    ])
    .0
}

/// March 2024 from Mon 4 Mar, `days` records: ZQALPHA lists every day and
/// ZQBETA from Tue 5 Mar.
fn march_from_monday(days: u8) -> MonthTable {
    month_of(
        (0..days)
            .map(|at| {
                let day = d(2024, 3, 4 + at);
                let mut tickers = keys("ZQALPHA");
                if at > 0 {
                    tickers.extend(keys("ZQBETA"));
                }
                (day, listed(day, &tickers))
            })
            .collect(),
    )
    .0
}

/// Mon 4 Mar 2024 lists ZQOLD; Tue 5 Mar ZQNEW on its three keys: one
/// month, one measured handover.
fn handed_over() -> MonthTable {
    let (monday, tuesday) = (d(2024, 3, 4), d(2024, 3, 5));
    month_of(vec![
        (monday, listed(monday, &keys("ZQOLD"))),
        (tuesday, listed(tuesday, &keys("ZQNEW"))),
    ])
    .0
}

/// An invented options source: the tickers of each listed day, the days it
/// refuses, and a day its reader dies on.
#[derive(Default)]
struct Fake {
    days: HashMap<Day, Vec<String>>,
    refused: Vec<Day>,
    dies_on: Option<Day>,
}

impl Fake {
    fn list<T: AsRef<str>>(mut self, day: Day, tickers: &[T]) -> Self {
        self.days.insert(
            day,
            tickers
                .iter()
                .map(|ticker| ticker.as_ref().to_owned())
                .collect(),
        );
        self
    }

    fn refuse(mut self, day: Day) -> Self {
        self.refused.push(day);
        self
    }
}

impl NfoSource for Fake {
    type Locator = ();

    fn day(&self, day: Day) -> Result<Option<NfoDay<()>>, CmRefusal> {
        assert_ne!(self.dies_on, Some(day), "an invented reader dies on {day}");
        if self.refused.contains(&day) {
            return Err(CmRefusal::FolderUnknown);
        }
        Ok(self.days.get(&day).map(|tickers| {
            let folder = day_folder_name(day);
            let mut listing = NfoDay::new(day, tickers.len() + 1);
            // A non-option entry is skipped by the listing, never a share.
            listing.push(&format!("{folder}/readme.txt"), 1, 0, ());
            for ticker in tickers {
                listing.push(&format!("{folder}/Options/{ticker}.NFO.csv"), 1, 0, ());
            }
            listing
        }))
    }

    fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
        Err(CmRefusal::FolderUnknown)
    }
}

/// Builds `dir` from `source` over a typed window: what the build said and
/// every month it reported.
fn build_into(
    source: &Fake,
    dir: &Path,
    from: Option<Day>,
    to: Day,
) -> Result<(Built, Vec<MonthBuilt>), MembershipError> {
    let mut months = Vec::new();
    let built = build(source, dir, Window::Typed { from, to }, 2, &mut |month| {
        months.push(month.clone());
    })?;
    Ok((built, months))
}

/// Every name in `dir`, sorted.
fn files(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// The newest generation of every month of `dir`: month and bytes.
fn newest(dir: &Path) -> Vec<((u16, u8), Vec<u8>)> {
    generations(dir)
        .unwrap()
        .0
        .into_iter()
        .map(|(month, (_, path))| (month, fs::read(path).unwrap()))
        .collect()
}

/// Re-seals `bytes`' digest after a change to its body.
fn reseal(mut bytes: Vec<u8>) -> Vec<u8> {
    let body = bytes.len() - DIGEST_LEN;
    let digest = brutex_core::blake3::hash(&bytes[..body]);
    bytes[body..].copy_from_slice(&digest);
    bytes
}

/// The words of the decoder's refusal of `bytes`, which names no file.
fn decode_refusal(bytes: &[u8]) -> String {
    let refusal = MonthTable::decode(bytes).unwrap_err();
    let MembershipError::Corrupt { path, why } = &refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(
        *path,
        PathBuf::new(),
        "a table read from bytes names no file"
    );
    why.clone()
}

/// The words of the chain's refusal of `table` after `damage`.
fn stale_after(table: &MonthTable, damage: fn(&mut MonthTable)) -> String {
    let mut damaged = table.clone();
    damage(&mut damaged);
    let refusal = Membership::of([damaged]).unwrap_err();
    let MembershipError::Stale { path, why } = &refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(*path, PathBuf::new(), "a table served without a file");
    why.clone()
}

/// Sat 9 Mar 2024 is closed and the source refuses its probe: the day reads
/// as refused with the source's own words, never as `NotSession`, and a
/// build reaching it stops there with those words after installing every
/// record before it (D-5168); the closed Fri 8 Mar before it is not sealed.
#[test]
fn a_closed_day_the_source_refuses_is_refused_with_its_words_and_stops_the_build() {
    let saturday = d(2024, 3, 9);
    assert_eq!(
        sourced_kind_of(i64::from(saturday.days_from_epoch())),
        DayKind::Closed
    );
    let ticker = ["ZQALPHA28MAR24500CE"];
    let source = (4..=7)
        .fold(Fake::default(), |source, day| {
            source.list(d(2024, 3, day), &ticker)
        })
        .refuse(saturday);
    let words = CmRefusal::FolderUnknown.to_string();
    assert_eq!(
        read_day(&source, saturday),
        DayInput::Unknown(DayStatus::SourceRefused, Some(words.clone()))
    );
    let dir = scratch("fnm-ps1-closed-refused");
    let built = build_into(&source, &dir, Some(d(2024, 3, 4)), d(2024, 3, 15)).map(|_| ());
    assert_eq!(
        built,
        Err(MembershipError::SourceRefused {
            day: saturday,
            why: words
        })
    );
    let held = Membership::open(&dir).unwrap();
    assert_eq!(
        held.sealed_through(),
        Some(d(2024, 3, 7)),
        "Fri 8 Mar waits with the 9th"
    );
}

/// Four sessions read by two readers, the second of which dies on its first
/// day: both of its days are refused by name ("a reader thread panicked"),
/// never absent, and the first reader's days read as they read alone. One
/// reader holding all four refuses all four, those it read before dying
/// included.
#[test]
fn a_reader_thread_that_panics_refuses_every_day_it_held_and_no_other() {
    let days = [d(2024, 3, 4), d(2024, 3, 5), d(2024, 3, 6), d(2024, 3, 7)];
    let ticker = ["ZQALPHA28MAR24500CE"];
    let source = Fake {
        dies_on: Some(days[2]),
        ..days
            .iter()
            .fold(Fake::default(), |source, &day| source.list(day, &ticker))
    };
    let refused = DayInput::Unknown(
        DayStatus::SourceRefused,
        Some("a reader thread panicked".to_owned()),
    );
    let two = read_days(&source, &days, 2);
    assert!(two[0].is_known(), "the first reader's days are listed");
    assert_eq!(
        two,
        [
            read_day(&source, days[0]),
            read_day(&source, days[1]),
            refused.clone(),
            refused.clone(),
        ]
    );
    let one = read_days(&source, &days, 1);
    assert_eq!(
        one,
        [refused.clone(), refused.clone(), refused.clone(), refused]
    );
}

/// [`MAX_SPELLINGS`] invented shares fold on one day, the bound itself held
/// (every member entry still names its spelling in a `u16`); a share never
/// seen before on the next day is refused as past the bound, by that day,
/// and nothing is registered past it.
#[test]
fn the_fold_registers_spellings_to_the_format_bound_and_refuses_one_more() {
    let (monday, tuesday) = (d(2024, 3, 4), d(2024, 3, 5));
    let mut state = State::default();
    let full = state
        .fold(monday, listed(monday, &the_format_bound_of_shares()))
        .unwrap();
    assert_eq!(
        (usize::from(full.row.pool), state.spellings()),
        (MAX_SPELLINGS, MAX_SPELLINGS)
    );
    let one_more = [format!("{}28MAR24900PE", invented(MAX_SPELLINGS))];
    assert_eq!(
        state.fold(tuesday, listed(tuesday, &one_more)),
        Err(MembershipError::TooManySpellings { day: tuesday })
    );
    assert_eq!(
        state.spellings(),
        MAX_SPELLINGS,
        "nothing is registered past the bound"
    );
}

/// A month file whose carry out names a live series the registry does not
/// hold (series 9 beside ZQOLD's series 0) passes the decoder and the chain.
/// Resumed from it, the handover test skips the series it cannot name, with
/// no report of it, and still joins ZQNEW to ZQOLD; the fold then refuses the
/// day, by name, because a live series has no spelling to file it under.
#[test]
fn a_carried_series_with_no_spelling_is_no_handovers_old_side_and_the_fold_refuses_it() {
    let (monday, tuesday) = (d(2024, 3, 4), d(2024, 3, 5));
    let until = d(2024, 3, 28).days_from_epoch();
    let (mut crafted, _) = month_of(vec![(monday, listed(monday, &keys("ZQOLD")))]);
    assert_eq!(crafted.carry_out().live, [(0, until)]);
    crafted.carry_out.live.push((9, until));
    let decoded = MonthTable::decode(&crafted.encode()).unwrap();
    assert_eq!(
        decoded, crafted,
        "a file holding the carry reads as written"
    );
    let held = Membership::of([decoded]).unwrap();
    let source = Fake::default().list(monday, &keys("ZQOLD"));
    let mut state = held.resume_state(&source).unwrap();
    let today = evidence(tuesday, keys("ZQNEW").iter().map(String::as_str));
    let (joined, ambiguous) = state.handovers_of(
        tuesday.days_from_epoch(),
        &today,
        state.previous.as_ref().unwrap(),
    );
    let pairs: Vec<_> = joined
        .iter()
        .map(|one| {
            (
                one.series,
                one.old,
                one.new,
                one.shared,
                one.old_keys,
                one.new_keys,
            )
        })
        .collect();
    assert_eq!(pairs, [(0, symbol("ZQOLD"), symbol("ZQNEW"), 3, 3, 3)]);
    assert!(ambiguous.is_empty(), "{ambiguous:?}");
    assert_eq!(
        state.fold(tuesday, DayInput::Listed(today)),
        Err(MembershipError::Corrupt {
            path: PathBuf::new(),
            why: "2024-03-05: a live series has no spelling".to_owned(),
        })
    );
}

/// Mon 4 Mar 2024 lists ZQIDLE and ZQOLD, Tue 5 Mar only ZQOLD, Wed 6 Mar
/// only ZQNEW on ZQOLD's three keys. ZQIDLE is live on Wednesday and listed
/// nothing on Tuesday, so it is no old side and makes no report, and the
/// test goes on past it to ZQOLD (a later series), which hands over.
#[test]
fn a_live_series_that_listed_nothing_the_session_before_is_no_handovers_old_side() {
    let (monday, tuesday, wednesday) = (d(2024, 3, 4), d(2024, 3, 5), d(2024, 3, 6));
    let mut both = keys("ZQOLD");
    both.push("ZQIDLE28MAR24700PE".to_owned());
    let mut state = State::default();
    state.fold(monday, listed(monday, &both)).unwrap();
    state
        .fold(tuesday, listed(tuesday, &keys("ZQOLD")))
        .unwrap();
    let record = state
        .fold(wednesday, listed(wednesday, &keys("ZQNEW")))
        .unwrap();
    assert_eq!(state.spelling_named(0), Some(symbol("ZQIDLE")), "series 0");
    assert!(record.ambiguous.is_empty(), "{:?}", record.ambiguous);
    assert_eq!(
        record.handovers,
        [Handover {
            old: 1,
            new: 2,
            day: wednesday,
            shared: 3,
            old_keys: 3,
        }]
    );
    assert_eq!(
        (record.row.pool, record.row.filed),
        (2, 1),
        "ZQIDLE stays a member with no file"
    );
}

/// Mon 4 Mar 2024 lists ZQOLDA and ZQOLDB on the same three keys; Tue 5 Mar
/// only ZQNEW on them. ZQNEW matches two live series: neither pair joins,
/// both are reported with why, and ZQNEW opens a series of its own.
#[test]
fn a_new_spelling_that_matches_two_live_series_joins_neither() {
    let (monday, tuesday) = (d(2024, 3, 4), d(2024, 3, 5));
    let mut state = State::default();
    state
        .fold(
            monday,
            listed(monday, &[keys("ZQOLDA"), keys("ZQOLDB")].concat()),
        )
        .unwrap();
    let record = state
        .fold(tuesday, listed(tuesday, &keys("ZQNEW")))
        .unwrap();
    assert!(record.handovers.is_empty(), "{:?}", record.handovers);
    assert_eq!(
        record.ambiguous,
        [
            "ZQOLDA -> ZQNEW (3 of 3 old keys, 3 of 3 new keys) not joined: a side matches a \
             second candidate",
            "ZQOLDB -> ZQNEW (3 of 3 old keys, 3 of 3 new keys) not joined: a side matches a \
             second candidate",
        ]
    );
    assert_eq!((state.series(), record.row.pool), (3, 3));
}

/// A table holding generation 1's records and one day more is a new
/// generation; the same table with a changed span start, a changed series
/// count before it, a changed member entry or a changed spelling of a sealed
/// day is refused as a rewrite of generation 1, said as such, and nothing is
/// written until the unchanged extension is.
#[test]
fn a_new_generation_that_changes_any_sealed_part_is_refused_as_a_rewrite() {
    let dir = scratch("fnm-ps1-rewrite");
    let sealed = march_from_monday(1);
    let first = dir.join(file_name(2024, 3, 1));
    assert_eq!(
        install(&dir, &sealed),
        Ok(Installed::Written {
            path: first.clone(),
            generation: 1,
            cleared: None,
        })
    );
    let longer = march_from_monday(2);
    assert!(longer.extends(&sealed));
    let changes: [fn(&mut MonthTable); 4] = [
        |table| table.known_from += 1,
        |table| table.series_before += 1,
        |table| table.entries[0].filed = false,
        |table| table.added[0].first_expiry += 1,
    ];
    for change in changes {
        let mut rewritten = longer.clone();
        change(&mut rewritten);
        let refusal = install(&dir, &rewritten).unwrap_err();
        assert_eq!(
            refusal,
            MembershipError::WouldRewrite {
                path: first.clone()
            }
        );
        assert_eq!(
            refusal.to_string(),
            format!(
                "the new generation of {} would change its sealed records",
                first.display()
            )
        );
    }
    assert_eq!(files(&dir), [file_name(2024, 3, 1)]);
    assert_eq!(
        install(&dir, &longer),
        Ok(Installed::Written {
            path: dir.join(file_name(2024, 3, 2)),
            generation: 2,
            cleared: None,
        })
    );
}

/// Every check of one file's bytes no test had failed, each refused with its
/// own words: a name with a byte after its zero padding, a last held day
/// before the first, a day row's reserved byte, and a carry run in and a
/// carry run out that do not strictly ascend.
#[test]
fn every_check_of_a_files_own_bytes_refuses_its_damage() {
    let table = five_days();
    let good = table.encode();
    assert_eq!(MonthTable::decode(&good), Ok(table.clone()));
    let damaged = |damage: &dyn Fn(&mut Vec<u8>)| {
        let mut bytes = good.clone();
        damage(&mut bytes);
        decode_refusal(&reseal(bytes))
    };
    let rows = HEADER_LEN + table.added().len() * SPELLING_LEN;
    assert_eq!(
        damaged(&|bytes: &mut Vec<u8>| bytes[HEADER_LEN + NAME_LEN - 1] = b'X'),
        "a name has bytes after its padding"
    );
    assert_eq!(
        damaged(&|bytes: &mut Vec<u8>| bytes[15] = bytes[16] + 1),
        "the last held day is before the first"
    );
    assert_eq!(
        damaged(&|bytes: &mut Vec<u8>| bytes[rows + 1] = 1),
        "a day row's reserved byte is not zero"
    );
    let mut carry_in = table.clone();
    carry_in.carry_in.live = vec![(1, 5), (0, 5)];
    assert_eq!(
        decode_refusal(&carry_in.encode()),
        "a carry run is not strictly ascending"
    );
    let mut carry_out = table;
    carry_out.carry_out.expiry_start = vec![(7, 1), (7, 1)];
    assert_eq!(
        decode_refusal(&carry_out.encode()),
        "a carry run is not strictly ascending"
    );
}

/// Every row check of the decoder no test had failed, each refused with its
/// own words naming the day: a closed day holding members by its pool and
/// by its filed count alone, a known day holding none, a closed day holding
/// a listing's evidence and its name count, a known day whose filed count is
/// not its entries', an entry past the last day's members, and a carry out
/// whose running sum is not the last row's.
#[test]
fn every_row_check_refuses_a_row_its_status_or_entries_contradict() {
    let table = five_days();
    let refused = |damage: fn(&mut MonthTable)| {
        let mut damaged = table.clone();
        damage(&mut damaged);
        decode_refusal(&damaged.encode())
    };
    let members = "day row 2 is not known and holds members";
    assert_eq!(refused(|table| table.rows[1].pool = 1), members);
    assert_eq!(refused(|table| table.rows[1].filed = 1), members);
    assert_eq!(
        refused(|table| table.rows[0].pool = 0),
        "day row 1 is known and holds no member"
    );
    let counts = "day row 2 has no listing and holds its counts";
    assert_eq!(
        refused(|table| table.rows[1].evidence = [1; DIGEST_LEN]),
        counts
    );
    assert_eq!(refused(|table| table.rows[1].names = 1), counts);
    assert_eq!(
        refused(|table| table.rows[0].filed = 0),
        "day row 1: filed count differs from its entries"
    );
    assert_eq!(
        refused(|table| table.entries.push(Entry {
            spelling: 0,
            filed: false
        })),
        "entries past the last day's members"
    );
    assert_eq!(
        refused(|table| table.carry_out.refused_in_year += 1),
        "the carry out's running sum is not the last row's"
    );
}

/// A link the host refuses for a reason other than a taken name (the final
/// name's folder is missing) is refused by the final path with the host's
/// own words, and the filled temporary is left to its caller; a name another
/// writer holds is `Exists`, said as nothing written, and its file is kept.
#[test]
fn a_link_the_host_refuses_is_named_by_its_final_path_and_a_taken_name_is_exists() {
    let dir = scratch("fnm-ps1-publish");
    let temporary = dir.join(".ZQ-temporary");
    let nowhere = dir.join("ZQ-no-such-folder").join(file_name(2024, 3, 1));
    let staged = store::staging::create(&temporary).unwrap();
    let refusal = publish(staged.file, b"ZQ bytes", &temporary, &nowhere).unwrap_err();
    let host = fs::hard_link(&temporary, &nowhere).unwrap_err();
    assert_ne!(host.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        refusal,
        MembershipError::Io {
            path: nowhere,
            why: host.to_string(),
        }
    );
    assert_eq!(fs::read(&temporary).unwrap(), b"ZQ bytes");
    let taken = dir.join(file_name(2024, 3, 1));
    fs::write(&taken, b"ZQ another writer").unwrap();
    let again = store::staging::create(&temporary).unwrap();
    assert_eq!(again.cleared, Some(8), "the first temporary, cleared");
    let refusal = publish(again.file, b"ZQ bytes", &temporary, &taken).unwrap_err();
    assert_eq!(
        refusal.to_string(),
        format!("{} already exists; nothing written", taken.display())
    );
    assert_eq!(fs::read(taken).unwrap(), b"ZQ another writer");
}

/// A membership path that is a file, or runs through one, is refused by that
/// path with the host's own words, never as a missing directory: by the
/// lister, by the reader, and by a build, which starts fresh only where
/// nothing is and writes nothing.
#[test]
fn a_membership_path_that_is_not_a_directory_is_refused_with_the_hosts_words() {
    let root = scratch("fnm-ps1-not-a-dir");
    let file = root.join("ZQ-a-file");
    fs::write(&file, b"ZQ").unwrap();
    let host = fs::read_dir(&file).unwrap_err();
    assert_ne!(host.kind(), std::io::ErrorKind::NotFound);
    let refused = MembershipError::Io {
        path: file.clone(),
        why: host.to_string(),
    };
    assert_eq!(generations(&file), Err(refused.clone()));
    assert_eq!(Membership::open(&file).map(|_| ()), Err(refused.clone()));
    let source = Fake::default().list(d(2024, 3, 4), &["ZQALPHA28MAR24500CE"]);
    let on = Some(d(2024, 3, 4));
    assert_eq!(
        build_into(&source, &file, on, d(2024, 3, 4)).map(|_| ()),
        Err(refused)
    );
    let through = file.join(DIR_NAME);
    let looked = fs::symlink_metadata(&through).unwrap_err();
    assert_ne!(looked.kind(), std::io::ErrorKind::NotFound);
    let host = fs::read_dir(&through).unwrap_err();
    assert_eq!(
        build_into(&source, &through, on, d(2024, 3, 4)).map(|_| ()),
        Err(MembershipError::Io {
            path: through,
            why: host.to_string(),
        })
    );
    assert_eq!(files(&root), ["ZQ-a-file"]);
}

/// Two generations of each of 48 months, made in alternating order: every
/// month is served at its highest generation whatever order the directory
/// lists them in. The listing (hash order on ext4 and APFS, make order on
/// tmpfs) lists at least one month's lower generation first, so the arm that
/// replaces a lower generation is read.
#[test]
fn the_highest_generation_of_a_month_wins_whatever_order_the_directory_lists() {
    let dir = scratch("fnm-ps1-generation-order");
    let months: Vec<(u16, u8)> = (0..48_u16)
        .map(|at| (2000 + at / 12, u8::try_from(at % 12).unwrap() + 1))
        .collect();
    for (at, &(year, month)) in months.iter().enumerate() {
        let made = if at % 2 == 0 { [3, 7] } else { [7, 3] };
        for generation in made {
            fs::write(dir.join(file_name(year, month, generation)), b"").unwrap();
        }
    }
    let names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let place = |name: &str| names.iter().position(|one| one == name);
    let lower_first = months
        .iter()
        .filter(|&&(year, month)| {
            place(&file_name(year, month, 3)) < place(&file_name(year, month, 7))
        })
        .count();
    assert!(lower_first > 0, "{names:?}");
    let expected: Generations = months
        .iter()
        .map(|&(year, month)| ((year, month), (7, dir.join(file_name(year, month, 7)))))
        .collect();
    assert_eq!(generations(&dir), Ok((expected, 0)));
}

/// An install whose directory cannot be made (a file holds a name on its
/// path) is refused by the directory's path with the host's own words, and
/// one whose current generation cannot be read (a directory holds its name)
/// by that generation's path; nothing is written either time.
#[test]
fn an_install_that_cannot_make_its_directory_or_read_its_generation_is_refused_by_path() {
    let root = scratch("fnm-ps1-install-io");
    let file = root.join("ZQ-a-file");
    fs::write(&file, b"ZQ").unwrap();
    let table = march_from_monday(1);
    let under_a_file = file.join(DIR_NAME);
    let host = fs::create_dir_all(&under_a_file).unwrap_err();
    assert_eq!(
        install(&under_a_file, &table),
        Err(MembershipError::Io {
            path: under_a_file,
            why: host.to_string(),
        })
    );
    assert_eq!(files(&root), ["ZQ-a-file"]);
    let dir = scratch("fnm-ps1-install-unreadable");
    let generation = dir.join(file_name(2024, 3, 1));
    fs::create_dir(&generation).unwrap();
    let host = fs::read(&generation).unwrap_err();
    assert_eq!(
        install(&dir, &table),
        Err(MembershipError::Io {
            path: generation,
            why: host.to_string(),
        })
    );
    assert_eq!(files(&dir), [file_name(2024, 3, 1)], "nothing beside it");
}

/// A current generation that fails its own checks is refused as corrupt by
/// its path, never extended; one already at the last generation number is
/// refused as exhausted, never wrapped or written over; nothing is written.
#[test]
fn a_current_generation_that_fails_its_checks_or_counts_no_further_is_refused() {
    let table = march_from_monday(2);
    let dir = scratch("fnm-ps1-install-corrupt");
    let garbage = dir.join(file_name(2024, 3, 1));
    fs::write(&garbage, b"ZQ not a month").unwrap();
    assert_eq!(
        install(&dir, &table),
        Err(MembershipError::Corrupt {
            path: garbage,
            why: "shorter than its digest".to_owned(),
        })
    );
    assert_eq!(files(&dir), [file_name(2024, 3, 1)]);
    let full = scratch("fnm-ps1-install-exhausted");
    let last = full.join(file_name(2024, 3, u32::MAX));
    fs::write(&last, march_from_monday(1).encode()).unwrap();
    assert_eq!(
        install(&full, &table),
        Err(MembershipError::Corrupt {
            path: last,
            why: "the generation counter is exhausted".to_owned(),
        })
    );
    assert_eq!(files(&full), [file_name(2024, 3, u32::MAX)]);
}

/// A temporary name the staging door refuses (a directory stands there) is
/// named in the refusal with the door's own words; the directory is left as
/// it is and no month file is written.
#[test]
fn a_temporary_the_staging_door_refuses_is_named_and_nothing_is_written() {
    let dir = scratch("fnm-ps1-staging");
    let planted = dir.join(".ZQ-planted");
    fs::create_dir(&planted).unwrap();
    let door = store::staging::create(&planted).unwrap_err().to_string();
    let table = march_from_monday(1);
    assert_eq!(
        install_with(&dir, &table, &|_: &str| planted.clone()),
        Err(MembershipError::Io {
            path: planted.clone(),
            why: door,
        })
    );
    assert!(planted.is_dir(), "left as it is");
    assert_eq!(
        generations(&dir),
        Ok((Generations::new(), 1)),
        "no month file; the hidden name is counted"
    );
}

/// The reader refuses a highest generation it cannot read (a directory holds
/// its name) by its path with the host's own words, and one whose header
/// names another month than its file name by its path: neither is served.
#[test]
fn open_refuses_a_generation_it_cannot_read_and_one_naming_another_month() {
    let dir = scratch("fnm-ps1-open-unreadable");
    let generation = dir.join(file_name(2024, 3, 1));
    fs::create_dir(&generation).unwrap();
    let host = fs::read(&generation).unwrap_err();
    assert_eq!(
        Membership::open(&dir).map(|_| ()),
        Err(MembershipError::Io {
            path: generation,
            why: host.to_string(),
        })
    );
    let moved = scratch("fnm-ps1-open-moved");
    let april = moved.join(file_name(2024, 4, 1));
    fs::write(&april, march_from_monday(1).encode()).unwrap();
    assert_eq!(
        Membership::open(&moved).map(|_| ()),
        Err(MembershipError::Corrupt {
            path: april,
            why: "the header names another month than the file name".to_owned(),
        })
    );
}

/// A one-month handover is served with its registry in id order, the new
/// spelling in the old one's series; the same month whose first record is
/// not known, whose series count before it is not the chain's, or whose
/// handed-over spelling names the next new series instead of an existing one
/// is refused by the chain, each with its own words.
#[test]
fn every_chain_check_one_month_can_fail_refuses_it_by_name() {
    let table = handed_over();
    let held = Membership::of([table.clone()]).unwrap();
    assert_eq!(held.registry(), table.added());
    let registry: Vec<(&str, u32)> = held
        .registry()
        .iter()
        .map(|one| (one.name.as_str(), one.series_id))
        .collect();
    assert_eq!(registry, [("ZQOLD", 0), ("ZQNEW", 0)]);
    assert_eq!(
        stale_after(&table, |table| table.rows[0].status = DayStatus::NoSource),
        "the span does not start on its first record"
    );
    assert_eq!(
        stale_after(&table, |table| table.series_before = 1),
        "its registry does not continue the month before's"
    );
    assert_eq!(
        stale_after(&table, |table| table.added[1].series_id = 1),
        "a spelling names a series out of order"
    );
}

/// A month registering [`MAX_SPELLINGS`] spellings is served with every one
/// of them in its registry; one spelling more, in series order and under a
/// name of its own, is refused by the chain as past the format bound.
#[test]
fn a_registry_past_the_format_bound_is_refused_by_the_chain() {
    let monday = d(2024, 3, 4);
    let (mut table, _) = month_of(vec![(
        monday,
        listed(monday, &the_format_bound_of_shares()),
    )]);
    let held = Membership::of([table.clone()]).unwrap();
    assert_eq!(held.registry().len(), MAX_SPELLINGS);
    assert_eq!(held.registry(), table.added());
    let mut past = table.added()[0];
    past.name = symbol(&invented(MAX_SPELLINGS));
    past.series_id = u32::try_from(MAX_SPELLINGS).unwrap();
    table.added.push(past);
    assert_eq!(
        Membership::of([table]).unwrap_err(),
        MembershipError::Stale {
            path: PathBuf::new(),
            why: "more spellings than the format bound".to_owned(),
        }
    );
}

/// Every session of August and September 2024 lists ZQBETA's current
/// contract except Fri 30 Aug, August's last session, which has no file.
fn aug_sep_without_the_last_august_session() -> Fake {
    let mut source = Fake::default();
    let mut day = d(2024, 8, 1);
    while day <= d(2024, 9, 30) {
        let ticker = if day <= d(2024, 8, 29) {
            "ZQBETA29AUG24700PE"
        } else if day <= d(2024, 9, 26) {
            "ZQBETA26SEP24700PE"
        } else {
            "ZQBETA31OCT24700PE"
        };
        let session = matches!(
            sourced_kind_of(i64::from(day.days_from_epoch())),
            DayKind::Open(_)
        );
        if session && day != d(2024, 8, 30) {
            source.days.insert(day, vec![ticker.to_owned()]);
        }
        day = day.succ().unwrap();
    }
    source
}

/// A build killed right after August is installed, August's last session
/// (Fri 30 Aug 2024) having no file: the directory serves August to its last
/// known day and answers the sealed session after it as after the span
/// (D-5167); the resume re-reads nothing, because only a known last session
/// is re-read and checked; and the resumed build ends byte for byte where
/// one pass ends.
#[test]
fn a_kill_after_a_month_ending_on_a_session_without_a_file_resumes_without_rereading_it() {
    let source = aug_sep_without_the_last_august_session();
    let window = Window::Typed {
        from: Some(d(2024, 8, 1)),
        to: d(2024, 9, 30),
    };
    let once = scratch("fnm-ps1-kill-once");
    build(&source, &once, window, 2, &mut |_| {}).unwrap();
    let cut = scratch("fnm-ps1-kill-cut");
    let died = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build(&source, &cut, window, 2, &mut |month| {
            assert_eq!(month.table.month(), (2024, 8), "August first");
            panic!("killed between the two installs");
        })
    }));
    assert!(died.is_err(), "the build died after August's install");
    let held = Membership::open(&cut).unwrap();
    assert_eq!(held.known_span(), Some((d(2024, 8, 1), d(2024, 8, 29))));
    assert_eq!(held.sealed_through(), Some(d(2024, 8, 31)));
    assert_eq!(
        held.members(d(2024, 8, 30)).unwrap_err().why,
        UnknownWhy::AfterSpan {
            known_to: d(2024, 8, 29)
        }
    );
    let state = held.resume_state(&source).unwrap();
    assert_eq!(state.last(), Some(d(2024, 8, 31)));
    assert!(state.previous.is_none(), "no last known session to re-read");
    let rest = Window::Typed {
        from: None,
        to: d(2024, 9, 30),
    };
    build(&source, &cut, rest, 2, &mut |_| {}).unwrap();
    assert_eq!(newest(&cut), newest(&once));
}

/// A calendar bound that is not a day (before the epoch, past `u32`, past
/// 9999-12-31) is refused as corrupt with its own words and an empty path;
/// the IST calendar's own bounds are days.
#[test]
fn a_calendar_bound_that_is_not_a_day_is_refused_as_corrupt() {
    assert_eq!(
        (
            calendar_day(SOURCED_FIRST_DAY),
            calendar_day(SOURCED_LAST_DAY)
        ),
        (Ok(d(2018, 1, 1)), Ok(d(2026, 12, 31)))
    );
    let refused = Err(MembershipError::Corrupt {
        path: PathBuf::new(),
        why: "the IST calendar's bounds are not days".to_owned(),
    });
    for epoch in [-1, i64::from(u32::MAX) + 1, i64::from(MAX_DAY_NUMBER) + 1] {
        assert_eq!(calendar_day(epoch), refused, "{epoch}");
    }
}

/// A window that runs backwards, or starts before the IST calendar's first
/// day (2018-01-01), is refused by its days before anything is read or
/// written, while the calendar's first day itself starts one; and a last
/// sealed record on the last day this build can name (9999-12-31) has no day
/// after it to resume at, refused by that day.
#[test]
fn a_window_backwards_before_the_calendar_or_past_the_last_nameable_day_is_refused() {
    let dir = scratch("fnm-ps1-window");
    let source = Fake::default();
    let window = |from, to| build_into(&source, &dir, Some(from), to).map(|_| ());
    let (backwards, early) = (
        (d(2024, 3, 15), d(2024, 3, 4)),
        (d(2017, 12, 31), d(2018, 1, 5)),
    );
    for (from, to) in [backwards, early] {
        assert_eq!(
            window(from, to),
            Err(MembershipError::BadWindow { from, to })
        );
    }
    assert!(files(&dir).is_empty(), "a refused window writes nothing");
    let first = d(2018, 1, 1);
    assert_eq!(
        bounds(
            Window::Typed {
                from: Some(first),
                to: d(2018, 1, 5)
            },
            None
        ),
        Ok((first, d(2018, 1, 5)))
    );
    let last = d(9999, 12, 31);
    assert_eq!(
        bounds(Window::Resume, Some(last)),
        Err(MembershipError::BadWindow {
            from: last,
            to: last
        })
    );
}

/// A session listing only an index option is not known: one before the
/// span's first known day leads the build and one after its last trails it,
/// both reported as listing no share option, never as a missing source.
#[test]
fn a_listing_with_no_share_option_outside_the_span_is_reported_as_no_share_options() {
    let (monday, tuesday, wednesday) = (d(2024, 3, 4), d(2024, 3, 5), d(2024, 3, 6));
    let index = ["NIFTY07MAR2499999.5CE"];
    let source = Fake::default()
        .list(monday, &index)
        .list(tuesday, &["ZQALPHA28MAR24500CE"])
        .list(wednesday, &index);
    let dir = scratch("fnm-ps1-index-only");
    let (built, _) = build_into(&source, &dir, Some(monday), wednesday).unwrap();
    assert_eq!(built.leading, [(monday, DayStatus::NoShareOptions, None)]);
    assert_eq!(
        built.trailing,
        [(wednesday, DayStatus::NoShareOptions, None)]
    );
    assert_eq!(built.folded, 1);
}

/// A known day listing a name the decoder refuses reaches its month's report
/// with its count and the refusal's own words, and the day stays known.
#[test]
fn a_known_days_refused_names_reach_its_months_report_with_count_and_words() {
    let monday = d(2024, 3, 4);
    let Err(why) = decode_ticker("ZQNOTATICKER", monday) else {
        panic!("an invented non-ticker decoded");
    };
    let source = Fake::default().list(monday, &["ZQALPHA28MAR24500CE", "ZQNOTATICKER"]);
    let dir = scratch("fnm-ps1-refusals");
    let (built, months) = build_into(&source, &dir, Some(monday), monday).unwrap();
    assert_eq!(built.folded, 1);
    assert_eq!(
        months[0].refusals,
        [(monday, 1, vec![format!("ZQNOTATICKER: {why}")])]
    );
    assert_eq!(Membership::open(&dir).unwrap().pool_size(monday), Ok(1));
}

/// A month's report names each handover by its two spellings, read from the
/// state the fold left; a state holding neither spelling cannot name it, and
/// the handover is left out of that report while the day still counts.
#[test]
fn a_handover_the_state_cannot_name_is_left_out_of_the_months_report() {
    let (monday, tuesday) = (d(2024, 3, 4), d(2024, 3, 5));
    let mut state = State::default();
    state.fold(monday, listed(monday, &keys("ZQOLD"))).unwrap();
    let record = state
        .fold(tuesday, listed(tuesday, &keys("ZQNEW")))
        .unwrap();
    let [handover] = record.handovers[..] else {
        panic!("{:?}", record.handovers);
    };
    let mut named = MonthWork::default();
    named.note(&record, &state);
    assert_eq!(
        named.handovers,
        [(handover, symbol("ZQOLD"), symbol("ZQNEW"))]
    );
    let mut unnamed = MonthWork::default();
    unnamed.note(&record, &State::default());
    assert!(unnamed.touched, "the day still counts");
    assert!(unnamed.handovers.is_empty(), "{:?}", unnamed.handovers);
}

/// **A ROW WITH NO LISTING WHOSE COUNTS SUM PAST THE FIELD IS REFUSED, NEVER
/// A PANIC** (D-7335): a closed day holding `names = u32::MAX` and
/// `refused = 1` is the same "has no listing and holds its counts" refusal as
/// a count of one. The check used to add the two counts, and the sum
/// overflowed: with overflow checks on in every profile the decoder panicked
/// on the month file's bytes, and the release build aborts on a panic. One
/// extreme at a time and both at once: each count at its maximum, the two
/// together, and a refused count of one beside no name.
#[test]
fn a_row_with_no_listing_whose_counts_pass_the_field_is_refused_never_a_panic() {
    let counts = "day row 2 has no listing and holds its counts";
    for (names, refused) in [(u32::MAX, 1), (0, u32::MAX), (u32::MAX, u32::MAX), (0, 1)] {
        let mut damaged = five_days();
        damaged.rows[1].names = names;
        damaged.rows[1].refused = refused;
        assert_eq!(
            decode_refusal(&damaged.encode()),
            counts,
            "{names} {refused}"
        );
    }
}

/// **A REFUSED PUBLISH WHOSE TEMPORARY WILL NOT GO EITHER SAYS BOTH FAULTS,
/// UNDER THE TEMPORARY'S NAME** (the unlink seam, D-7335): a link refused
/// because the final name's folder is missing, beside an unlink the host
/// refuses, is one `Io` naming the temporary with the link's words and the
/// unlink's; with the unlink taken it is the link's refusal alone; and a
/// publish that landed hands the unlink's own answer back. Extremes at once:
/// both writes refused in one call, and a refused unlink after a link that
/// landed.
#[test]
fn a_refused_publish_whose_temporary_will_not_go_says_both_faults() {
    let dir = scratch("fnm-ps1-unlink");
    let temporary = dir.join(".ZQ-temporary");
    let nowhere = dir.join("ZQ-no-such-folder").join(file_name(2024, 3, 1));
    let refuse = |_: &Path| Err(std::io::Error::other("an invented refusal"));
    let staged = store::staging::create(&temporary).unwrap();
    let MembershipError::Io { path, why } =
        publish_then_unlink(staged.file, b"ZQ bytes", (&temporary, &nowhere), &refuse).unwrap_err()
    else {
        panic!("both faults are one Io");
    };
    assert_eq!(path, temporary);
    assert!(
        why.ends_with("; and its temporary could not be unlinked: an invented refusal"),
        "{why}"
    );
    let staged = store::staging::create(&temporary).unwrap();
    let alone = publish_then_unlink(
        staged.file,
        b"ZQ bytes",
        (&temporary, &nowhere),
        &|left: &Path| fs::remove_file(left),
    )
    .unwrap_err();
    assert!(
        matches!(&alone, MembershipError::Io { path, .. } if *path == nowhere),
        "{alone:?}"
    );
    assert!(!temporary.exists(), "the unlink took it");
    let landed = dir.join(file_name(2024, 3, 1));
    let staged = store::staging::create(&temporary).unwrap();
    let left = publish_then_unlink(staged.file, b"ZQ bytes", (&temporary, &landed), &refuse)
        .unwrap()
        .unwrap_err();
    assert_eq!(left.to_string(), "an invented refusal");
    assert_eq!(fs::read(&landed).unwrap(), b"ZQ bytes");
}
