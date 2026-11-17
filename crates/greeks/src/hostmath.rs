//! The two transcendental functions of this crate, `exp` and `ln`, and the one place a host may replace them.
//!
//! fix:determinism (16 Nov 2026, the spike engine's det copy of this crate; MAIN-RULINGS-ENGINE-v22 Addendum 30.3 follow-up).
//!
//! # Why
//!
//! Every `d1`, both discount factors, the density and both branches of the distribution go through `exp` or `ln`, and the platform's
//! libm (Apple's on a Mac, glibc's on Linux) rounds them differently in the last bits: measured on 1,470 index-option rows, Apple's libm
//! and the pure-Rust `libm` 0.2.16 disagree on 9.3% of the `exp` results, and Apple's arm64 and x86_64 libms print different Greeks
//! on 18 of the rows (`v34/determinism/FIND-DETERMINISM.md`, E2 / E3). The crate's own words below, "reproducible within one target,
//! and not across two", are that fact.
//!
//! # What this module does
//!
//! - **Default (nothing installed): the platform's functions,** `f64::exp` / `f64::ln` from std, exactly as the crate always called
//!   them: the same calls on the same arguments, so the same bits on every machine as before (the host's P0).
//! - **A host may install ONE implementation for the whole process** ([`set_global`]): from then on every `exp` / `ln` of this crate
//!   is that implementation's. The spike engine installs the pinned pure-Rust `libm` 0.2.16 (FreeBSD msun algorithms, integer and
//!   IEEE-754 binary64 arithmetic only), so the same inputs give the same bits on every 64-bit platform. The price: the results then
//!   differ in the last bits from the platform libm this crate uses elsewhere (for example behind `/greeks.json`).
//! - **This crate still depends on nothing** (CI gate 9b reads `[dependencies]` and `[dev-dependencies]`, both still empty): the host
//!   links the implementation and hands over two plain function pointers. No type from any workspace enters the public surface.
//! - Tests of a host whose unit tests run in parallel threads may route ONE thread instead (`set_thread`, feature `det-thread`, never in
//!   a release build).
//!
//! The cost when nothing is installed is one atomic load (Acquire) and a predictable branch per call.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// A host-provided implementation of this crate's two transcendental functions.
#[derive(Clone, Copy, Debug)]
pub struct HostMath {
    /// `e^x`.
    pub exp: fn(f64) -> f64,
    /// The natural logarithm.
    pub ln: fn(f64) -> f64,
    /// What it is, for provenance (printed by the host).
    pub name: &'static str,
}

/// The implementation installed for the process (at most one per process: a second, different one is refused).
static GLOBAL: OnceLock<HostMath> = OnceLock::new();
/// Whether the installed implementation is in force (Release on store, Acquire on load: a thread that sees `true` sees `GLOBAL` set).
static ON: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "det-thread")]
thread_local! {
    /// Test hook (feature `det-thread`): this thread's own routing, ahead of the process-wide one.
    static LOCAL: std::cell::Cell<Option<HostMath>> = const { std::cell::Cell::new(None) };
}

/// Process-wide: `Some(m)` routes every later `exp` / `ln` of this crate, on every thread, through `m`; `None` returns to the
/// platform's functions.
///
/// # Errors
///
/// A second implementation whose name differs from the one already installed: one implementation per process, so two runs of one
/// process can never mix two sets of bits.
pub fn set_global(m: Option<HostMath>) -> Result<(), String> {
    match m {
        None => {
            ON.store(false, Ordering::Release);
            Ok(())
        }
        Some(m) => {
            let g = GLOBAL.get_or_init(|| m);
            if g.name != m.name {
                return Err(format!("greeks::hostmath: '{}' is installed; '{}' refused (one implementation per process)", g.name, m.name));
            }
            ON.store(true, Ordering::Release);
            Ok(())
        }
    }
}

/// Test hook (feature `det-thread`): this thread only; `Some(m)` routes this thread's calls through `m` whatever the process-wide
/// setting, `None` follows the process-wide setting again.
#[cfg(feature = "det-thread")]
pub fn set_thread(m: Option<HostMath>) {
    LOCAL.with(|c| c.set(m));
}

/// The implementation in force on this thread: `Some(host)`, or `None` for the platform's functions.
#[inline]
#[must_use]
pub fn active() -> Option<HostMath> {
    #[cfg(feature = "det-thread")]
    {
        if let Some(m) = LOCAL.with(std::cell::Cell::get) {
            return Some(m);
        }
    }
    if ON.load(Ordering::Acquire) { GLOBAL.get().copied() } else { None }
}

/// `e^x`: the installed implementation's when one is in force, else the platform's (`f64::exp`).
#[inline]
#[must_use]
pub fn exp(x: f64) -> f64 {
    match active() {
        Some(m) => (m.exp)(x),
        None => x.exp(),
    }
}

/// `ln x`: the installed implementation's when one is in force, else the platform's (`f64::ln`).
#[inline]
#[must_use]
pub fn ln(x: f64) -> f64 {
    match active() {
        Some(m) => (m.ln)(x),
        None => x.ln(),
    }
}
