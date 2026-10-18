//! The pool-run identity, the data tree and the pair identity, proved against
//! FIB-1 (D-6303, D-6316, D-6317). Every fixture is invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::fib::test_support;
use crate::identity::{self, Run};
use brutex_core::instrument::{Exchange, InstrumentKey, Kind};
use vocab::fib_cube::{FIB_CUBE_VERSION, Search};

/// One edit of a run, for the term-by-term test.
type Edit = Box<dyn Fn(&mut FibRun<'_>)>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn pool_bytes() -> Vec<u8> {
    Pool::Development {
        label: "dev-3",
        names: &["ALPHA", "BETA", "GAMMA"],
    }
    .bytes()
    .expect("a sorted pool")
}

fn run(pool: &[u8]) -> FibRun<'_> {
    FibRun {
        mask: ConditionMask::ZERO.with_bit(0).with_bit(81),
        direction: Search::Long,
        pool,
        timeframe: "zerodha:1min,2min,4min",
        params: fib_params(&[7, 11]),
        schema_digest: [0x5C; OUT_LEN],
        generation_root: [0x6E; OUT_LEN],
        data_digest: [0xDA; OUT_LEN],
        vocab_version: FIB_CUBE_VERSION,
        commit: "0123456789abcdef",
        feed: FEED_STAGE_A,
    }
}

/// THE FRAMING IS `runner::identity`'S, BYTE FOR BYTE. That module's `term`
/// is private, so it is re-implemented here and proved two ways: the framed
/// bytes of one term hash to what `term` streams, and the nine terms of a
/// single-instrument run, streamed through this `term` with no domain tag,
/// give exactly `runner::identity::identity` of that run.
#[test]
fn the_term_framing_is_runner_identity_s() {
    let mut streamed = Hasher::new();
    term(&mut streamed, 5, b"abc");
    let framed = [5, 3, 0, 0, 0, b'a', b'b', b'c'];
    assert_eq!(streamed.finalize(), hash(&framed));
    assert_eq!(
        hex(&streamed.finalize()),
        TERM_GOLDEN,
        "the term framing changed"
    );

    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("a swept index");
    let mask = ConditionMask::ZERO.with_bit(3).with_bit(200);
    let params = fib_params(&[4, 2]);
    let digest = [0x42; OUT_LEN];
    let legacy = Run {
        mask,
        direction: Direction::Short,
        instrument: &key,
        timeframe: "1min",
        params,
        data_digest: digest,
        commit: "feedface",
        feed: "zerodha",
    };
    let mut ours = Hasher::new();
    let mut mask_bytes = Vec::new();
    for word in mask.words() {
        mask_bytes.extend_from_slice(&word.to_le_bytes());
    }
    term(&mut ours, Term::Mask.tag(), &mask_bytes);
    term(&mut ours, Term::Direction.tag(), &[Direction::Short.byte()]);
    let mut instrument = Vec::new();
    for part in [
        key.exchange.as_str(),
        key.segment.as_str(),
        key.underlying.as_str(),
    ] {
        frame(&mut instrument, part.as_bytes());
    }
    assert_eq!(key.kind, Kind::Index);
    frame(&mut instrument, &[0]);
    term(&mut ours, Term::Instrument.tag(), &instrument);
    term(&mut ours, Term::Timeframe.tag(), b"1min");
    let mut p = Vec::new();
    for value in [
        params.min_hits,
        params.ceiling,
        params.pair_budget,
        params.policy,
    ] {
        p.extend_from_slice(&value.to_le_bytes());
    }
    term(&mut ours, Term::Params.tag(), &p);
    term(&mut ours, Term::DataDigest.tag(), &digest);
    term(
        &mut ours,
        Term::VocabVersion.tag(),
        &vocab::VOCAB_VERSION.to_le_bytes(),
    );
    term(&mut ours, Term::Commit.tag(), b"feedface");
    term(&mut ours, Term::Feed.tag(), b"zerodha");
    assert_eq!(ours.finalize(), identity::identity(&legacy).bytes());
    // The tags are the nine positions of CLAUDE.md section 3 rule 3.
    let tags: Vec<u8> = [
        Term::Mask,
        Term::Direction,
        Term::Instrument,
        Term::Timeframe,
        Term::Params,
        Term::DataDigest,
        Term::VocabVersion,
        Term::Commit,
        Term::Feed,
    ]
    .iter()
    .map(|t| t.tag())
    .collect();
    assert_eq!(tags, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

const TERM_GOLDEN: &str = "7d3c4e5e0ead755ea695341fa91c351c587ac8c0b9b9285f0b34ced67a3f9724";

#[test]
fn fib_identity_is_stable() {
    let pool = pool_bytes();
    let id = fib_identity(&run(&pool));
    assert_eq!(id.hex(), IDENTITY_GOLDEN, "every FIB run identity re-keyed");
    assert_eq!(id, fib_identity(&run(&pool)), "idempotent");
}

const IDENTITY_GOLDEN: &str = "13cdef8590f9bb0890c0e803bcc9e81dcfcaab4796adfea1c9a2d307a9594863";

#[test]
fn fib_identity_changes_when_any_one_term_changes() {
    let pool = pool_bytes();
    let other_pool = Pool::Development {
        label: "dev-3",
        names: &["ALPHA", "BETA", "OMEGA"],
    }
    .bytes()
    .unwrap();
    let base = fib_identity(&run(&pool));
    let edits: Vec<Edit> = vec![
        Box::new(|r| r.mask = r.mask.with_bit(2)),
        Box::new(|r| r.direction = Search::Short),
        Box::new(|r| r.timeframe = "zerodha:1min,2min"),
        Box::new(|r| r.params = fib_params(&[7, 12])),
        Box::new(|r| r.schema_digest[0] ^= 1),
        Box::new(|r| r.generation_root[31] ^= 1),
        Box::new(|r| r.data_digest[7] ^= 1),
        Box::new(|r| r.vocab_version += 1),
        Box::new(|r| r.commit = "0123456789abcdee"),
        Box::new(|r| r.feed = FEED_DEEP),
    ];
    let mut seen = vec![base];
    for edit in &edits {
        let mut r = run(&pool);
        edit(&mut r);
        let id = fib_identity(&r);
        assert!(
            !seen.contains(&id),
            "an edit did not re-key, or two collided"
        );
        seen.push(id);
    }
    let mut r = run(&pool);
    r.pool = &other_pool;
    assert_ne!(fib_identity(&r), base, "the pool is a term");
}

/// A FIB identity never equals a single-instrument identity: the stream starts
/// with `BTXFIBP1`, where `runner::identity` starts with the mask's tag.
#[test]
fn fib_identity_differs_from_runner_identity_for_any_single_instrument_run() {
    let key = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap();
    for (direction, search) in [
        (Direction::Long, Search::Long),
        (Direction::Short, Search::Short),
    ] {
        for feed in ["zerodha", "zerodha+gdfl"] {
            let mask = ConditionMask::ZERO.with_bit(9);
            let params = fib_params(&[]);
            let legacy = Run {
                mask,
                direction,
                instrument: &key,
                timeframe: "1min",
                params,
                data_digest: [1; OUT_LEN],
                commit: "c",
                feed,
            };
            let pool = Pool::Development {
                label: "single",
                names: &["BANKNIFTY"],
            }
            .bytes()
            .unwrap();
            let fib = FibRun {
                mask,
                direction: search,
                pool: &pool,
                timeframe: "1min",
                params,
                schema_digest: [0; OUT_LEN],
                generation_root: [0; OUT_LEN],
                data_digest: [1; OUT_LEN],
                vocab_version: vocab::VOCAB_VERSION,
                commit: "c",
                feed,
            };
            assert_ne!(fib_identity(&fib), identity::identity(&legacy));
        }
    }
    assert_eq!(direction_of(Search::Long), Direction::Long);
    assert_eq!(direction_of(Search::Short), Direction::Short);
}

#[test]
fn pair_identity_changes_with_either_side_and_differs_from_both() {
    let pool = pool_bytes();
    let long = fib_identity(&run(&pool));
    let mut short_run = run(&pool);
    short_run.direction = Search::Short;
    let short = fib_identity(&short_run);
    let numbers = fixed_numbers_digest();
    let pair = pair_identity(&long, &short, 1, &numbers);
    assert_ne!(pair, long);
    assert_ne!(pair, short);
    assert_eq!(pair, pair_identity(&long, &short, 1, &numbers));
    assert_ne!(
        pair,
        pair_identity(&short, &long, 1, &numbers),
        "sides are named"
    );
    let mut other = run(&pool);
    other.commit = "other";
    let other_long = fib_identity(&other);
    assert_ne!(pair, pair_identity(&other_long, &short, 1, &numbers));
    other.direction = Search::Short;
    assert_ne!(
        pair,
        pair_identity(&long, &fib_identity(&other), 1, &numbers)
    );
    assert_ne!(pair, pair_identity(&long, &short, 2, &numbers));
    let mut changed = numbers;
    changed[0] ^= 1;
    assert_ne!(pair, pair_identity(&long, &short, 1, &changed));
    assert_eq!(numbers, hash(&fixed_numbers_bytes()));
    assert_eq!(pair.hex(), PAIR_GOLDEN);
}

/// Re-pinned by FIB-2 (D-6323, D-6325): the fixed-number ledger gained the
/// radix digit width and the checkpoint varint group width, so its digest and
/// every pair identity re-keyed.
const PAIR_GOLDEN: &str = "1956186185fa61075de4abe4ae5a5bce20ee8b941138c8be0974092f6324283f";

fn leaf(n: u8) -> [u8; OUT_LEN] {
    [n; OUT_LEN]
}

#[test]
fn merkle_odd_leaf_golden() {
    assert_eq!(hex(&merkle(&[])), EMPTY_GOLDEN, "the empty root");
    assert_eq!(merkle(&[leaf(1)]), node(Node::Leaf, &[&leaf(1)]));
    let three = merkle(&[leaf(1), leaf(2), leaf(3)]);
    assert_eq!(
        hex(&three),
        ODD_GOLDEN,
        "three leaves: a peak of two and a peak of one"
    );
    let l = |n| node(Node::Leaf, &[&leaf(n)]);
    let two = node(Node::Parent, &[&l(1), &l(2)]);
    assert_eq!(
        three,
        node(Node::Bag, &[&two, &l(3)]),
        "the odd leaf stays a peak"
    );
    assert_ne!(
        three,
        merkle(&[leaf(2), leaf(1), leaf(3)]),
        "order-dependent"
    );
    assert_ne!(
        three,
        merkle(&[leaf(1), leaf(2), leaf(3), leaf(3)]),
        "no duplication"
    );
}

const EMPTY_GOLDEN: &str = "cd2f51de576b8cdb0f43c1ec54f3118833340b751df08b3c7810e3928e546d65";
const ODD_GOLDEN: &str = "383b167240fec050f0299af67faaf5e699e219c1725d10c4e44f0260a55be0db";

#[test]
fn mmr_append_equals_rebuild() {
    let mut grown = Mmr::new();
    assert_eq!(grown, Mmr::default());
    let mut leaves = Vec::new();
    for n in 0..40_u8 {
        assert_eq!(grown.root(), merkle(&leaves), "after {n} leaves");
        assert_eq!(grown.peaks().len(), (n as usize).count_ones() as usize);
        assert_eq!(grown.leaves(), u64::from(n));
        assert_eq!(grown.merges(), u64::from(n) - u64::from(n.count_ones()));
        leaves.push(leaf(n));
        grown.append(node(Node::Leaf, &[&leaf(n)]));
    }
    // One append does at most log2(n) + 1 merges: the 32nd leaf does five.
    let mut m = Mmr::new();
    for n in 0..31_u8 {
        m.append(leaf(n));
    }
    let before = m.merges();
    m.append(leaf(31));
    assert_eq!(m.merges() - before, 5);
    assert_eq!(m.peaks().len(), 1);
}

fn key(year_month: u32, kind: LeafKind, series: &str) -> LeafKey {
    LeafKey {
        year_month,
        kind,
        series: series.to_owned(),
    }
}

#[test]
fn identity_independent_of_listing_and_worker_order() {
    let listed = vec![
        (key(202_503, LeafKind::Minutes, "ALPHA"), leaf(1)),
        (key(202_503, LeafKind::Minutes, "BETA"), leaf(2)),
        (key(202_503, LeafKind::Authority, "calendar"), leaf(3)),
        (key(202_504, LeafKind::Minutes, "ALPHA"), leaf(4)),
        (key(202_504, LeafKind::Deep, "ALPHA"), leaf(5)),
    ];
    let sorted = DataTree::of_unsorted(listed.clone()).unwrap();
    let mut reversed = listed.clone();
    reversed.reverse();
    let mut interleaved = listed.clone();
    interleaved.swap(0, 3);
    interleaved.swap(1, 4);
    for order in [reversed, interleaved] {
        assert_eq!(DataTree::of_unsorted(order).unwrap().root(), sorted.root());
    }
    // Appending in key order is the same tree.
    let mut appended = DataTree::default();
    let mut in_order = listed.clone();
    in_order.sort();
    for (k, d) in &in_order {
        appended.append(k.clone(), d).unwrap();
    }
    assert_eq!(appended, sorted);
    assert_eq!(appended.peaks(), sorted.peaks());
    // Five leaves are a peak of four and a peak of one, the run manifest's peaks.
    assert_eq!(sorted.peaks(), sorted.mmr().peaks());
    assert_eq!(sorted.peaks().len(), 2);
    assert_eq!(appended.mmr().leaves(), 5);
    // One digest moves the root; so does one key.
    let mut moved = listed.clone();
    moved[2].1 = leaf(9);
    assert_ne!(DataTree::of_unsorted(moved).unwrap().root(), sorted.root());
    let mut renamed = listed.clone();
    renamed[1].0.series = "BETB".to_owned();
    assert_ne!(
        DataTree::of_unsorted(renamed).unwrap().root(),
        sorted.root()
    );
    // A key listed twice is refused by name.
    let mut twice = listed.clone();
    twice.push(listed[0].clone());
    let refused = DataTree::of_unsorted(twice).unwrap_err();
    assert_eq!(refused.name(), "leaf_out_of_order");
}

/// MONTH-MAJOR: a new month only appends; an earlier month after a later one
/// is refused rather than rebuilding the tree.
#[test]
fn a_new_month_appends_and_an_older_one_is_refused() {
    let mut tree = DataTree::new();
    tree.append(key(202_503, LeafKind::Deep, "ZETA"), &leaf(1))
        .unwrap();
    tree.append(key(202_504, LeafKind::Minutes, "ALPHA"), &leaf(2))
        .unwrap();
    let late = key(202_503, LeafKind::Minutes, "ALPHA");
    assert_eq!(
        tree.append(late.clone(), &leaf(3)),
        Err(FibIdentityRefusal::LeafOutOfOrder { key: late })
    );
    assert!(key(202_503, LeafKind::Authority, "a") < key(202_504, LeafKind::Minutes, "a"));
    assert!(key(202_503, LeafKind::Minutes, "z") < key(202_503, LeafKind::Deep, "a"));
    assert_eq!(LeafKind::Minutes.name(), "minutes");
    assert_eq!(LeafKind::Deep.name(), "deep");
    assert_eq!(LeafKind::Authority.name(), "authority");
}

#[test]
fn a_pool_has_one_spelling() {
    let pit = Pool::PointInTime {
        membership_digest: [3; OUT_LEN],
        surface_rule: "fno_cash_equity",
    }
    .bytes()
    .unwrap();
    assert!(pit.starts_with(&[3, 0, 0, 0, b'p', b'i', b't']));
    assert_ne!(pit, pool_bytes());
    assert_eq!(
        Pool::Development {
            label: "x",
            names: &[]
        }
        .bytes(),
        Err(FibIdentityRefusal::PoolEmpty)
    );
    assert_eq!(
        Pool::Development {
            label: "x",
            names: &["B", "A"]
        }
        .bytes(),
        Err(FibIdentityRefusal::PoolNamesNotSorted { at: 1 })
    );
    let repeated = Pool::Development {
        label: "x",
        names: &["A", "B", "B"],
    }
    .bytes()
    .unwrap_err();
    assert_eq!(repeated, FibIdentityRefusal::PoolNamesNotSorted { at: 2 });
    assert_eq!(repeated.name(), "pool_names_not_sorted");
    assert_eq!(FibIdentityRefusal::PoolEmpty.name(), "pool_empty");
    assert!(
        FibIdentityRefusal::PoolEmpty
            .to_string()
            .starts_with("fib identity refused (pool_empty)")
    );
    let boxed: Box<dyn std::error::Error> = Box::new(FibIdentityRefusal::PoolEmpty);
    assert!(boxed.to_string().contains("pool_empty"));
}

#[test]
fn the_timeframe_word_names_every_width_tried_in_every_version() {
    let chain = test_support::chain();
    assert_eq!(timeframe_word(&chain), "zerodha:1min,2min,3min,4min");
    let one =
        vocab::fib_cube::GenerationChain::new(test_support::generation(Search::Long, 1, 6, &[118]));
    assert_eq!(timeframe_word(&one), "zerodha:118min");
    assert_eq!(stage_a_feed(None), "zerodha");
    assert_eq!(stage_a_feed(Some("of1")), "zerodha+of1");
}

#[test]
fn fib_params_fold_the_profile_and_walk_no_ladder() {
    let p = fib_params(&[1]);
    assert_eq!((p.min_hits, p.ceiling, p.pair_budget), (0, 0, 0));
    assert_ne!(p, fib_params(&[1, 0]), "a trailing zero still re-keys");
    assert_ne!(p, fib_params(&[]));
    assert_eq!(p, fib_params(&[1]));
}

#[test]
fn the_generation_root_hashes_the_ordered_chain_of_versions() {
    let chain = test_support::chain();
    let root = generation_root(&chain);
    let digests: Vec<[u8; OUT_LEN]> = chain
        .versions()
        .map(|v| hash(&v.run_descriptor()))
        .collect();
    assert_eq!(root, merkle(&digests));
    let mut reversed = digests.clone();
    reversed.reverse();
    assert_ne!(root, merkle(&reversed), "order-dependent");
    let first_only = vocab::fib_cube::GenerationChain::new(test_support::generation(
        Search::Long,
        1,
        6,
        &[1, 2, 4],
    ));
    assert_ne!(root, generation_root(&first_only));
}
