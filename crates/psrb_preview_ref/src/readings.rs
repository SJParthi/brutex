//! F12 `readings::Bank::step` (with F13..F18, the seven states it holds) and F19
//! `readings::Taint` (PREVIEW-SPEC sections 3.1 "Folding" and 3.4 "Hole").
//!
//! One candle folds into all seven states or into none: a candle the candle check
//! refuses, or whose close is above 2^32 paisa, is folded into none and is a hole.
//! After a hole each reading is tainted until it has folded its own period of clean
//! candles.

use crate::indicators::bollinger::{Bollinger, MAX_CLOSE};
use crate::indicators::cci::Cci;
use crate::indicators::ema::Ema;
use crate::indicators::rsi::Rsi;
use crate::indicators::supertrend::SuperTrend;
use crate::indicators::willr::WilliamsR;
use crate::indicators::{Corrupt, check_evaluable};
use crate::market::Bar;

/// The six readings a rule set can gate on, in table order (both Supertrends are one).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Supertrend 10/2 and 10/3 (ATR 10).
    Supertrend,
    /// Bollinger 20.
    Bollinger,
    /// RSI 14.
    Rsi,
    /// Williams %R 14.
    WilliamsR,
    /// CCI 20.
    Cci,
    /// EMA 200.
    Ema,
}

/// Every reading kind, in table order.
pub const KINDS: [Kind; 6] = [
    Kind::Supertrend,
    Kind::Bollinger,
    Kind::Rsi,
    Kind::WilliamsR,
    Kind::Cci,
    Kind::Ema,
];

impl Kind {
    /// The clean candles a reading needs after a hole: 10, 20, 14, 14, 20, 200.
    #[must_use]
    pub const fn taint_period(self) -> u32 {
        match self {
            Self::Supertrend => 10,
            Self::Bollinger | Self::Cci => 20,
            Self::Rsi | Self::WilliamsR => 14,
            Self::Ema => 200,
        }
    }

    /// Its place in [`KINDS`].
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Supertrend => 0,
            Self::Bollinger => 1,
            Self::Rsi => 2,
            Self::WilliamsR => 3,
            Self::Cci => 4,
            Self::Ema => 5,
        }
    }

    /// A short label for reports.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Supertrend => "ST",
            Self::Bollinger => "BB",
            Self::Rsi => "RSI",
            Self::WilliamsR => "WR",
            Self::Cci => "CCI",
            Self::Ema => "EMA",
        }
    }
}

/// Why a candle was folded into none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The candle's own check.
    Corrupt(Corrupt),
    /// A close above 2^32 paisa (the Bollinger window's ceiling).
    CloseAboveCeiling,
}

/// The bank's pre-check: the one test every state would apply.
///
/// # Errors
/// The first refusal.
pub fn check(bar: &Bar) -> Result<(), Refusal> {
    check_evaluable(bar).map_err(Refusal::Corrupt)?;
    if bar.close > MAX_CLOSE {
        return Err(Refusal::CloseAboveCeiling);
    }
    Ok(())
}

/// F19: clean candles since the last hole, per reading; `None` before any hole.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Taint {
    clean: [Option<u32>; 6],
}

impl Taint {
    /// A hole: every reading restarts its count.
    pub const fn hole(&mut self) {
        self.clean = [Some(0); 6];
    }

    /// A clean candle: every count rises by one.
    pub fn clean(&mut self) {
        for c in self.clean.iter_mut().flatten() {
            *c = c.saturating_add(1);
        }
    }

    /// The reading has folded its own period of clean candles since the last hole.
    #[must_use]
    pub fn untainted(&self, kind: Kind) -> bool {
        self.clean
            .get(kind.index())
            .copied()
            .flatten()
            .is_none_or(|c| c >= kind.taint_period())
    }
}

/// F12: the seven states of one stock and timeframe, and their taint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bank {
    /// F13: Supertrend 10 / 2.000.
    pub st2: SuperTrend,
    /// F13: Supertrend 10 / 3.000.
    pub st3: SuperTrend,
    /// F14: Bollinger 20 (read at widths 2.000 and 3.000).
    pub bb: Bollinger,
    /// F15: RSI 14.
    pub rsi: Rsi,
    /// F16: Williams %R 14.
    pub wr: WilliamsR,
    /// F17: CCI 20.
    pub cci: Cci,
    /// F18: EMA 200.
    pub ema: Ema,
    /// F19: the taint counts.
    pub taint: Taint,
    /// Candles folded.
    pub folded: u64,
}

/// What one step did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// A clean candle, folded: a decision may be read on it.
    Clean,
    /// A hole that held a bar: folded into every state, no decision, taints.
    FoldedHole,
    /// A hole with no bar, or a refused candle: folded into none, taints.
    Hole(Option<Refusal>),
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

impl Bank {
    /// Fresh states at the recipe's fixed parameters.
    #[must_use]
    pub fn new() -> Self {
        Self {
            st2: SuperTrend::new(10, 2_000),
            st3: SuperTrend::new(10, 3_000),
            bb: Bollinger::new(20),
            rsi: Rsi::new(14),
            wr: WilliamsR::new(14),
            cci: Cci::new(20),
            ema: Ema::new(200),
            taint: Taint::default(),
            folded: 0,
        }
    }

    /// One scheduled bucket: its bar if it holds one, and whether it is a hole.
    pub fn step(&mut self, bar: Option<&Bar>, hole: bool) -> Step {
        let Some(bar) = bar else {
            self.taint.hole();
            return Step::Hole(None);
        };
        if let Err(why) = check(bar) {
            self.taint.hole();
            return Step::Hole(Some(why));
        }
        self.st2.fold(bar);
        self.st3.fold(bar);
        self.bb.fold(bar);
        self.rsi.fold(bar);
        self.wr.fold(bar);
        self.cci.fold(bar);
        self.ema.fold(bar);
        self.folded += 1;
        if hole {
            self.taint.hole();
            Step::FoldedHole
        } else {
            self.taint.clean();
            Step::Clean
        }
    }

    /// The reading `kind` has folded enough candles to read.
    #[must_use]
    pub fn warm(&self, kind: Kind) -> bool {
        match kind {
            Kind::Supertrend => self.st2.reading().is_some(),
            Kind::Bollinger => self.bb.warm(),
            Kind::Rsi => self.rsi.warm(),
            Kind::WilliamsR => self.wr.warm(),
            Kind::Cci => self.cci.warm(),
            Kind::Ema => self.ema.warm(),
        }
    }
}

#[cfg(test)]
#[path = "readings_tests.rs"]
mod tests;
