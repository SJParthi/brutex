//! The Fib Flag Breakout engine's runner half (FIB-1 scaffold; D-6300).
//!
//! # The funnel (D-6300)
//!
//! STAGE A finds every candidate on Zerodha minutes and walks it on Zerodha
//! one-minute bars; each lands in one full cell of `vocab::fib_cube`. A BUC
//! iceberg cube grows one-direction mixes by trade count (D-6302), and gate A —
//! the Winner Gate on NET trades — judges every visited mix on every
//! walk-forward learning window. STAGE DEEP re-walks only gate-A survivors on
//! GDFL one-second bars at the worst price and walk-forwards them (gate B). The
//! newest months are a sealed exam, opened once, only after the operator's yes.
//! Equities stay labelled research: gross of every charge until FIB-11 wires
//! the charges, and never Selection V6 or execution authority (`CLAUDE.md` §1).
//!
//! # Who owns what
//!
//! | Module | Owner | What |
//! |---|---|---|
//! | [`identity`] | FIB-1 | the pool-run identity, the data tree, the pair identity |
//! | [`types`] | FIB-1, then FIB-5 adds fields | shared value types with no behaviour |
//! | [`rank`] | FIB-2 | the mover rank |
//! | [`walk`], [`codec`] | FIB-5 | the walker and its records |
//! | [`lattice`], [`score`] | FIB-6 | the BUC lattice and its scoring |
//! | [`wf`], [`luck`], [`pair`] | FIB-7 | walk-forward, the luck check, pairing |
//! | [`charges`] | FIB-11 | the precise charges; [`charges::is_wired`] is false until then |
//! | [`oos`], [`refine`] | FIB-13 | the unseen-month check and refinement |
//!
//! The application-wide Winner Gate is [`crate::winner_gate`] (FIB-7),
//! deliberately outside this module: the operator applies it to every strategy.
//!
//! # The schema digests
//!
//! [`KNOWN_SCHEMAS`] pins the BLAKE3 of every COMPILED schema version's
//! `vocab::fib_cube::descriptor`, so a schema edit without a version bump fails
//! the build. A run's PER-RUN descriptor (the compiled schema, its generation
//! and its layout; FM-F006) is hashed by [`run_schema_digest`] into the
//! identity's params and recorded in the append-only [`SchemaLedger`].
//!
//! This module does no I/O and its loops call nothing (gates 17 and 22).

pub mod charges;
pub mod codec;
pub mod identity;
pub mod lattice;
pub mod luck;
pub mod oos;
pub mod pair;
pub mod rank;
pub mod refine;
pub mod score;
pub mod types;
pub mod walk;
pub mod wf;

#[cfg(test)]
mod test_support;

use brutex_core::blake3::{OUT_LEN, hash};
use vocab::fib_cube::{Generation, descriptor};

/// Every compiled schema version with the BLAKE3 of its descriptor, oldest
/// first. APPEND-ONLY: a row is never edited, and a new schema version adds
/// one.
pub const KNOWN_SCHEMAS: &[(u32, [u8; OUT_LEN])] = &[(
    1,
    *b"\xe9\xdc\xd4\x3d\x21\xb8\xbe\xf8\x34\x83\xaa\x1f\xac\x15\x23\x5f\x86\x76\x00\x16\x1b\xd7\x0d\xe8\x50\xcb\x9e\xa2\x1b\x51\xa1\x51",
)];

/// The BLAKE3 of the compiled schema's descriptor.
#[must_use]
pub fn schema_digest() -> [u8; OUT_LEN] {
    hash(&descriptor())
}

/// The BLAKE3 of one generation version's per-run descriptor (FM-F006): the
/// schema digest every FIB file header and the run identity carry.
#[must_use]
pub fn run_schema_digest(generation: &Generation) -> [u8; OUT_LEN] {
    hash(&generation.run_descriptor())
}

/// The append-only ledger of run-time schema digests (FM-F006). A digest is
/// recorded once; recording it again answers its first position. Nothing is
/// ever removed or reordered, so a position names one schema forever.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaLedger {
    /// The digests, in the order they were first recorded.
    entries: Vec<[u8; OUT_LEN]>,
}

impl SchemaLedger {
    /// An empty ledger.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The position of `digest`, appending it when it is new.
    pub fn record(&mut self, digest: [u8; OUT_LEN]) -> usize {
        if let Some(at) = self.position(&digest) {
            return at;
        }
        self.entries.push(digest);
        self.entries.len() - 1
    }

    /// The position of `digest`, or `None` when it was never recorded.
    #[must_use]
    pub fn position(&self, digest: &[u8; OUT_LEN]) -> Option<usize> {
        self.entries.iter().position(|entry| entry == digest)
    }

    /// Every digest, in recording order.
    #[must_use]
    pub fn entries(&self) -> &[[u8; OUT_LEN]] {
        &self.entries
    }
}

impl Default for SchemaLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use vocab::fib_cube::{FIB_CUBE_VERSION, Search, ZONE_RUNGS, untyped_numbers};

    /// A SCHEMA EDIT WITHOUT A VERSION BUMP FAILS THE BUILD.
    #[test]
    fn known_schemas_pin_the_compiled_descriptor() {
        let (version, digest) = KNOWN_SCHEMAS.last().copied().expect("one schema");
        assert_eq!(version, FIB_CUBE_VERSION);
        assert_eq!(
            digest,
            schema_digest(),
            "vocab::fib_cube::descriptor changed without a FIB_CUBE_VERSION bump: \
             add a schema version and a KNOWN_SCHEMAS row"
        );
        assert!(
            KNOWN_SCHEMAS.windows(2).all(|w| w[0].0 < w[1].0),
            "versions strictly increase"
        );
        assert_eq!(KNOWN_SCHEMAS[0].0, 1);
    }

    #[test]
    fn the_run_schema_digest_moves_with_the_generation_and_the_ledger_appends() {
        let long = test_support::generation(Search::Long, 1, 6, &[1, 2, 4]);
        let short = test_support::generation(Search::Short, 1, 6, &[1, 2, 4]);
        let a = run_schema_digest(&long);
        let b = run_schema_digest(&short);
        assert_ne!(a, b);
        assert_ne!(a, schema_digest());
        assert_eq!(a, hash(&long.run_descriptor()));
        let mut ledger = SchemaLedger::default();
        assert_eq!(ledger, SchemaLedger::new());
        assert_eq!(ledger.record(a), 0);
        assert_eq!(ledger.record(b), 1);
        assert_eq!(ledger.record(a), 0, "recorded once");
        assert_eq!(ledger.entries(), &[a, b]);
        assert_eq!(ledger.position(&b), Some(1));
        assert_eq!(ledger.position(&[0; OUT_LEN]), None);
    }

    /// FM-F006: a run-time schema (here, one exit set against another) is
    /// recorded in the schema ledger and moves the run identity's params.
    #[test]
    fn runtime_schema_recorded_in_identity() {
        let four = test_support::generation(Search::Long, 1, 6, &[1, 2, 4]);
        let mut parts = four.parts().clone();
        parts.exit_set.count = 7;
        let seven = Generation::new(parts).expect("seven exit variants");
        let (a, b) = (run_schema_digest(&four), run_schema_digest(&seven));
        assert_ne!(a, b);
        let mut ledger = SchemaLedger::new();
        assert_eq!((ledger.record(a), ledger.record(b)), (0, 1));
        let pool = identity::Pool::Development {
            label: "dev",
            names: &["ALPHA"],
        }
        .bytes()
        .expect("a pool");
        let run = |schema_digest| identity::FibRun {
            mask: four.layout().offered_mask().expect("maskable"),
            direction: Search::Long,
            pool: &pool,
            timeframe: "zerodha:1min,2min,4min",
            params: identity::fib_params(&[]),
            schema_digest,
            generation_root: [0; OUT_LEN],
            data_digest: [0; OUT_LEN],
            vocab_version: FIB_CUBE_VERSION,
            commit: "c",
            feed: identity::FEED_STAGE_A,
        };
        assert_ne!(
            identity::fib_identity(&run(a)),
            identity::fib_identity(&run(b)),
            "the per-run descriptor is an identity term"
        );
    }

    /// The `fib_zone` rungs `vocab` retypes (it can name no other crate) are
    /// the first seven of `indicators::fib::RUNGS`.
    #[test]
    fn fib_zone_rungs_are_the_first_seven_of_indicators_fib_rungs() {
        assert_eq!(ZONE_RUNGS.len(), 7);
        for (zone, rung) in ZONE_RUNGS.iter().zip(indicators::fib::RUNGS) {
            assert_eq!(*zone, u64::try_from(rung).unwrap());
        }
    }

    /// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): every FIB-1 file of this
    /// module holds no numeric literal but 0 and 1 in its shipping part. The
    /// test-only fixture module is not shipped and is not read.
    #[test]
    fn every_fib1_file_of_runner_types_no_number() {
        for (name, source) in [
            ("mod.rs", include_str!("mod.rs")),
            ("identity.rs", include_str!("identity.rs")),
            ("types.rs", include_str!("types.rs")),
            ("charges.rs", include_str!("charges.rs")),
            ("codec.rs", include_str!("codec.rs")),
            ("lattice.rs", include_str!("lattice.rs")),
            ("luck.rs", include_str!("luck.rs")),
            ("oos.rs", include_str!("oos.rs")),
            ("pair.rs", include_str!("pair.rs")),
            ("rank.rs", include_str!("rank.rs")),
            ("refine.rs", include_str!("refine.rs")),
            ("score.rs", include_str!("score.rs")),
            ("walk.rs", include_str!("walk.rs")),
            ("wf.rs", include_str!("wf.rs")),
            ("../winner_gate.rs", include_str!("../winner_gate.rs")),
        ] {
            let typed = untyped_numbers(source);
            assert!(typed.is_empty(), "{name} types {typed:?}");
        }
    }
}
