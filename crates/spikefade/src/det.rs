//! fix:determinism (16 Nov 2026; `v34/determinism/FIX-DETERMINISM.md`, after find:determinism's E1-E3): the engine's platform-dependent
//! computations made PORTABLE (the same bits on every 64-bit platform), each behind a switch that is OFF by default (off = the old kit
//! byte for byte, P0).
//!
//! - **DET-1 (tools; `kit::common_args`, `slice::cmd_cube_slice`):** every subcommand takes `--portable`, `--portable-math` and
//!   `--tool-<t>`; `cube-slice --transport` implies PORTABLE. The boxes' C2 stopped at `cube-slice --transport`, whose zstd was the Mac
//!   path `/opt/homebrew/bin/zstd` whatever PATH, `SF_ZSTD` or `--tool-zstd` said.
//! - **DET-2 (transcendental functions; this module, `kit::PMATH`):** the platform libm (Apple's, glibc's) rounds `exp`, `ln` and
//!   `ln_1p` differently in the last bits (MEASURED: Apple's libm and libm 0.2.16 differ on 9.3% of 169,320 `exp` results of 1,470
//!   index-option rows; Apple's arm64 and x86_64 libms print different Greeks on 18 of those rows). Under `--portable-math` (implied by
//!   `--shard on`) every such call of the engine goes through ONE pure-Rust implementation, `libm` 0.2.16 pinned under
//!   `pinned/libm-0.2.16` (FreeBSD msun algorithms on integer and IEEE-754 binary64 arithmetic; default features off, so no
//!   arch-specific path): the Greeks crate's `exp` / `ln` (its `hostmath`, installed below) and `gk.rs`'s `ln_1p` (below).
//!   `sqrt` stays std's: IEEE-754 rounds it correctly on every platform.

use std::borrow::Cow;

/// The ONE pure-Rust implementation (provenance words: printed in RUN.log and the RESULTS Greeks lines under `--portable-math`).
pub const LIBM_NAME: &str = "libm 0.2.16 (pure Rust; pinned/libm-0.2.16 = the local cargo registry's copy, .crate sha256 b6d2cec3eae94f9f509c767b45932f1ada8350c4bdb85af2fcab4a3c14807981; default features off)";

/// The kit word of PMATH.
pub const PMATH_WORDS: &str = "PORTABLE MATH (DET-2: exp / ln / ln_1p from the pinned pure-Rust libm 0.2.16: the same bits on every 64-bit platform)";

/// The Greeks crate's host implementation: the pinned libm.
pub const LIBM: greeks::hostmath::HostMath = greeks::hostmath::HostMath { exp: libm::exp, ln: libm::log, name: LIBM_NAME };

/// Routes the Greeks crate (process-wide; for this test thread only in a test build, where every kit switch is per thread).
pub fn portable_math(on: bool) {
    #[cfg(not(test))]
    {
        if let Err(e) = greeks::hostmath::set_global(on.then_some(LIBM)) {
            eprintln!("REFUSED: {e}");
            std::process::exit(2);
        }
    }
    #[cfg(test)]
    {
        greeks::hostmath::set_thread(on.then_some(LIBM));
    }
}

#[cfg(test)]
thread_local! {
    /// Test hook: the `ln_1p` calls this thread routed through the pinned libm.
    pub static LN1P_PORTABLE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// `ln(1 + x)`: the pinned libm's under PMATH, else the platform's (`f64::ln_1p`, the old kit).
#[must_use]
pub fn ln_1p(x: f64) -> f64 {
    if crate::kit::on(crate::kit::PMATH) {
        #[cfg(test)]
        LN1P_PORTABLE.with(|c| c.set(c.get() + 1));
        return libm::log1p(x);
    }
    // MUTANT m_det_off_ln1p_libm: the switch-off path takes the pinned libm too (off would no longer be the old kit)
    if cfg!(feature = "m_det_off_ln1p_libm") { libm::log1p(x) } else { x.ln_1p() }
}

/// The words of the report lines that name the Greeks code: under PMATH they name the pinned libm (the 8b proof's bit-for-bit equality
/// with the live app holds with the platform libm only); off: the text unchanged (v3.3's words byte for byte).
#[must_use]
pub fn greeks_label(s: &str) -> Cow<'_, str> {
    // MUTANT m_det_greeks_words_v33: v3.3's words kept under PMATH
    if !crate::kit::on(crate::kit::PMATH) || cfg!(feature = "m_det_greeks_words_v33") {
        return Cow::Borrowed(s);
    }
    let mut t = s.to_owned();
    for (a, b) in LABELS {
        t = t.replace(a, b);
    }
    Cow::Owned(t)
}

/// (v3.3's words, the words under PMATH).
pub const LABELS: [(&str, &str); 5] = [
    (
        "the app's own Greeks code on the 1-minute inputs, proven equal to the live app",
        "the app's own Greeks formulas on the 1-minute inputs with exp / ln / ln_1p from the pinned pure-Rust libm 0.2.16 (--portable-math: the same bits on every 64-bit platform; not bit for bit the live app's)",
    ),
    (
        "the app's OWN crates/greeks (path dependency, unmodified)",
        "the app's OWN crates/greeks formulas (a copy whose exp / ln / ln_1p come from the pinned pure-Rust libm 0.2.16 under --portable-math: the same bits on every 64-bit platform; they differ from the platform libm's in the last bits, MEASURED on 1,470 index-option rows: IV within 1.4e-12, vega within 1.2e-11 relative)",
    ),
    (
        "(proven equal to the live app's /greeks.json, which serves",
        "(--portable-math: exp / ln / ln_1p from the pinned pure-Rust libm 0.2.16, so equal to the live app's /greeks.json in all but the last bits, not bit for bit; /greeks.json serves",
    ),
    (
        "crates/greeks linked unmodified; 8b proof: v32/PROOF-GREEKS-8B-v32.txt",
        "crates/greeks formulas with exp / ln / ln_1p from the pinned libm 0.2.16 (--portable-math); the 8b proof v32/PROOF-GREEKS-8B-v32.txt is bit for bit with the platform libm only",
    ),
    (
        "crates/greeks linked unmodified; 8b proof on v3.3:",
        "crates/greeks formulas with exp / ln / ln_1p from the pinned pure-Rust libm 0.2.16 (--portable-math: this run's Greeks are not bit for bit the live app's); the 8b proofs, made with the platform libm, on v3.3:",
    ),
];
