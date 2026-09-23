//! Explicit priced expression research using the common screen and trade codec.
use crate::candidate_trades::{self, Capture, Evaluated, Key, Model, Tier, TradeReader};
use indicators::{Candle, column::Column};
use runner::{expression::Expression, grid, outcome::Horizon, trade::SliceFacts};
use std::path::Path;

#[derive(Clone, Copy)]
pub(crate) struct Plan {
    pub horizon: Horizon,
    pub rungs: usize,
    pub step: i64,
    pub rules: crate::Rules,
}
impl Plan {
    pub(crate) fn parse(args: &[&str]) -> Result<Self, String> {
        let [hold, rungs, step] = args else {
            return Err("priced search requires HORIZON GRID_RUNGS GRID_STEP_PPM".to_owned());
        };
        let horizon = hold
            .parse::<u32>()
            .ok()
            .and_then(Horizon::bars)
            .ok_or("HORIZON must be positive execution minutes")?;
        let rungs = rungs
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("GRID_RUNGS must be positive")?;
        let step = step
            .parse::<i64>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("GRID_STEP_PPM must be positive")?;
        if i64::try_from(rungs)
            .ok()
            .and_then(|n| n.checked_mul(step))
            .is_none()
            || grid::variants(rungs.saturating_add(1), rungs, rungs) > 100_000
        {
            return Err("explicit expression grid exceeds its 100,000-cell or integer admission; no pricing occurred".to_owned());
        }
        validate_overrides()?;
        Ok(Self {
            horizon,
            rungs,
            step,
            rules: crate::Rules::operator(),
        })
    }
    pub(crate) fn words(self) -> [u64; 16] {
        let r = self.rules;
        [
            u64::from_le_bytes(*b"EXPRICE1"),
            u64::from(self.horizon.as_bars()),
            self.rungs as u64,
            self.step.cast_unsigned(),
            1,
            r.max_mae_ppm.cast_unsigned(),
            r.min_rr_bp.cast_unsigned(),
            r.min_win_rate_bp.cast_unsigned(),
            r.min_trades,
            r.min_assurance_bp.cast_unsigned(),
            r.min_weakest_bp.cast_unsigned(),
            r.min_ret_over_dd_bp.cast_unsigned(),
            u64::from(r.require_protective_exits),
            r.min_fill_headroom_bp.cast_unsigned(),
            r.min_avg_rr_bp.cast_unsigned(),
            r.top as u64,
        ]
    }
    fn tier(self) -> Tier {
        Tier {
            index: 0,
            eligible: 1,
            evaluated: 1,
            horizon: u64::from(self.horizon.as_bars()),
            rungs: self.rungs as u64,
            step_ppm: Some(self.step),
            forced_ppm: (self.rules.max_mae_ppm > 0).then_some(self.rules.max_mae_ppm),
            ratios: true,
            rules: self.rules,
            stops_ppm: Vec::new(),
        }
    }
}

fn validate_overrides() -> Result<(), String> {
    for name in [
        "BRUTEX_MAX_MAE_PPM",
        "BRUTEX_MIN_RR_BP",
        "BRUTEX_MIN_WIN_RATE_BP",
        "BRUTEX_MIN_TRADES",
        "BRUTEX_MIN_WEAKEST_BP",
        "BRUTEX_MIN_RET_OVER_DD_BP",
        "BRUTEX_MIN_FILL_HEADROOM_BP",
        "BRUTEX_MIN_AVG_RR_BP",
        "BRUTEX_TOP",
    ] {
        if let Some(value) = crate::knobs::var(name) {
            let valid = value
                .parse::<i64>()
                .ok()
                .is_some_and(|n| n >= 0 && (name != "BRUTEX_MIN_WIN_RATE_BP" || n <= 10_000));
            if !valid {
                return Err(format!(
                    "{name} is malformed or outside its nonnegative policy range; no default was substituted"
                ));
            }
        }
    }
    if crate::knobs::var("BRUTEX_PROTECTED_EXITS").is_some_and(|s| !matches!(s.as_str(), "0" | "1"))
    {
        return Err("BRUTEX_PROTECTED_EXITS must be exactly 0 or 1".to_owned());
    }
    Ok(())
}

pub(crate) struct Prepared {
    pub bars: Vec<Candle>,
    pub column: Column,
    pub plan: Plan,
    pub execution_note: String,
    facts: SliceFacts,
}
impl Prepared {
    pub(crate) fn new(
        bars: Vec<Candle>,
        column: Column,
        plan: Plan,
        execution_note: String,
    ) -> Self {
        let facts = SliceFacts::of(&bars, &column);
        Self {
            bars,
            column,
            plan,
            execution_note,
            facts,
        }
    }
    pub(crate) fn capture(
        &self,
        root: &Path,
        attempt: &crate::sweep_evidence::Attempt,
        expression: &Expression,
    ) -> Result<(), String> {
        let capture =
            Capture::begin_expression(root, attempt, &self.bars, &self.column, expression)?;
        let tier = capture.tier(self.plan.tier())?;
        let mask = expression.referenced();
        for direction in [costs::fill::Direction::Long, costs::fill::Direction::Short] {
            let grid = grid::evaluate_expression_over(
                &self.bars,
                &self.column,
                expression,
                self.plan.horizon,
                crate::side_of_direction(direction),
                grid::Levels {
                    rungs: self.plan.rungs,
                    step_ppm: Some(self.plan.step),
                    forced: tier.forced_ppm,
                    ratios: true,
                    stops_ppm: &[],
                },
                &self.facts,
            )?;
            capture.record(
                &tier,
                &Evaluated {
                    rank: 1,
                    mask: &mask,
                    direction,
                    grid: &grid,
                    selected: crate::shown_cell(&grid, self.plan.rules),
                },
            )?;
        }
        capture.finish()?;
        capture.confirm()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Verify {
    pub plan: Plan,
    pub execution_digest: [u8; 32],
}
impl Verify {
    pub(crate) fn candidate(
        self,
        root: &Path,
        identity: [u8; 32],
        attempt: u64,
        expression: &Expression,
    ) -> Result<(), String> {
        let summary = candidate_trades::read_model(
            root,
            identity,
            attempt,
            Model::Expression,
            candidate_trades::DEFAULT_MAX_BYTES,
        )?
        .ok_or("priced expression has no complete exact trade catalog")?;
        if summary.expression() != Some(expression)
            || summary.execution_digest != self.execution_digest
            || summary.tiers != 1
            || summary.candidates != 2
        {
            return Err(
                "priced expression catalog does not match its actual predicate/execution scope"
                    .to_owned(),
            );
        }
        if candidate_trades::tier(root, &summary, 0, candidate_trades::DEFAULT_MAX_BYTES)?
            != self.plan.tier()
        {
            return Err("priced expression tier differs from the identified policy".to_owned());
        }
        for direction in [costs::fill::Direction::Long, costs::fill::Direction::Short] {
            TradeReader::open(
                root,
                &summary,
                Key {
                    tier: 0,
                    rank: 1,
                    direction,
                },
                candidate_trades::DEFAULT_MAX_BYTES,
            )?;
        }
        Ok(())
    }
}
