//! The bench rows of the pool-run identity and the data tree (FIB-1; D-6303,
//! D-6316). Both are BOUNDED by design and declared so: an identity hashes its
//! terms once per run, and an append to the Merkle Mountain Range merges at
//! most log2(n) times.

use std::fmt::Write as _;

use runner::fib::identity::{FEED_STAGE_A, FibRun, Mmr, Pool, fib_identity, fib_params};
use vocab::ConditionMask;
use vocab::fib_cube::Search;

use crate::harness::{Axis, Class, Host, Row, run};

/// The identity's state: one pool's bytes and the term bytes hashed so far.
pub(crate) struct Identity {
    /// The pool's canonical bytes.
    pool: Vec<u8>,
    /// Term bytes hashed, in all: the structural work counter.
    hashed: u64,
}

fn identity_state(names: u64) -> Identity {
    let owned: Vec<String> = (0..names).map(|n| format!("SYN{n:06}")).collect();
    let names: Vec<&str> = owned.iter().map(String::as_str).collect();
    let pool = Pool::Development {
        label: "synthetic",
        names: &names,
    }
    .bytes()
    .expect("sorted synthetic names");
    Identity { pool, hashed: 0 }
}

fn identity_op(state: &mut Identity, i: u64) -> u64 {
    let run = FibRun {
        mask: ConditionMask::ZERO.with_bit(1),
        direction: Search::Long,
        pool: &state.pool,
        timeframe: "zerodha:1min,2min,4min",
        params: fib_params(&[i]),
        schema_digest: [1; 32],
        generation_root: [2; 32],
        data_digest: [3; 32],
        vocab_version: 1,
        commit: "0123456789abcdef0123456789abcdef01234567",
        feed: FEED_STAGE_A,
    };
    let id = fib_identity(&run);
    state.hashed += state.pool.len() as u64;
    u64::from(id.bytes()[0])
}

fn mmr_state(leaves: u64) -> Mmr {
    let mut mmr = Mmr::new();
    for n in 0..leaves {
        mmr.append(leaf(n));
    }
    mmr
}

fn leaf(n: u64) -> [u8; 32] {
    let mut digest = [0; 32];
    digest[..8].copy_from_slice(&n.to_le_bytes());
    digest
}

/// The rows, run and reported. False when any row failed.
pub(crate) fn report(host: &Host, out: &mut String) -> bool {
    let identity = Row {
        id: "C-F-05",
        name: "fib_identity of one pooled run",
        class: Some(Class::Bounded {
            bound: "O(term bytes), once per run",
        }),
        axis: Some(Axis::FixedState {
            why: "the run's own terms, hashed once per run",
        }),
        scales: [1, 10, 100],
        setup: identity_state,
        op: identity_op,
        work: |state| state.hashed,
        bytes: |state| state.pool.len() as u64,
    };
    let mmr = Row {
        id: "C-F-06",
        name: "one Merkle Mountain Range append",
        class: Some(Class::Bounded {
            bound: "O(log n) merges per append, once per stock-month leaf",
        }),
        axis: Some(Axis::FixedState {
            why: "the state is the peaks: O(log n) digests",
        }),
        scales: [1_000, 10_000, 100_000],
        setup: mmr_state,
        op: |mmr, i| {
            mmr.append(leaf(i));
            mmr.leaves()
        },
        work: Mmr::merges,
        bytes: |mmr| mmr.peaks().len() as u64 * 32,
    };
    let mut ok = true;
    for verdict in [run(&identity, host, out), run(&mmr, host, out)] {
        ok &= !verdict.fails();
    }
    let _ = writeln!(out);
    ok
}
