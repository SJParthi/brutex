//! Explicit, bounded runtime research policy. No market data is inspected to
//! choose thresholds, and a malformed selected file never falls back to knobs.

#[cfg(test)]
#[path = "research_policy_tests.rs"]
mod adversarial_tests;

use std::fmt::Write as _;
use std::io::Read as _;
use std::path::Path;

use runner::admission::AdmissionFieldV1;

pub(crate) const SETTING: &str = "BRUTEX_ADMISSION_POLICY_FILE";
const MAX_BYTES: u64 = 16_384;
const FIELDS: usize = AdmissionFieldV1::ALL.len();

/// One immutable file snapshot, resolved before computation or result selection.
#[derive(Clone)]
pub(crate) struct ResearchPolicy {
    values: [Option<String>; FIELDS],
    provenance: String,
}

impl ResearchPolicy {
    /// Only the process owner can select a policy path; HTTP engine knobs cannot.
    pub(crate) fn from_env(max_loss_paisa: u64) -> Result<Option<Self>, String> {
        std::env::var_os(SETTING)
            .map(|path| Self::read(Path::new(&path), max_loss_paisa))
            .transpose()
    }

    pub(crate) fn read(path: &Path, max_loss_paisa: u64) -> Result<Self, String> {
        let file = crate::readonly_file::open(path)
            .map_err(|why| format!("research policy file refused: {why}"))?;
        let before = crate::result_set::file_generation(&file, path)
            .map_err(|why| format!("research policy source refused: {why}"))?;
        let mut bytes = Vec::new();
        (&file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|why| format!("research policy read refused: {why}"))?;
        #[cfg(test)]
        adversarial_tests::after_read(path)?;
        let after = crate::result_set::file_generation(&file, path)
            .map_err(|why| format!("research policy source changed: {why}"))?;
        if before != after {
            return Err("research policy file changed during its bounded read".into());
        }
        Self::parse(&bytes, max_loss_paisa)
    }

    fn parse(bytes: &[u8], max_loss_paisa: u64) -> Result<Self, String> {
        if bytes.len() as u64 > MAX_BYTES {
            return Err("research policy exceeds 16384-byte input bound".into());
        }
        if max_loss_paisa == 0 {
            return Err("research policy requires a positive MAX_POINTS risk amount".into());
        }
        let source =
            std::str::from_utf8(bytes).map_err(|_| "research policy is not UTF-8".to_owned())?;
        let mut values = std::array::from_fn(|_| None);
        let mut version = false;
        for (index, line) in source.lines().enumerate() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let refusal = |why: &str| format!("research policy line {}: {why}", index + 1);
            let (key, raw) = line
                .split_once('=')
                .ok_or_else(|| refusal("expected one key = value"))?;
            let (key, raw) = (key.trim(), raw.trim());
            if key == "policy_version" {
                if version || raw != "1" {
                    return Err(refusal("policy_version must occur once and equal 1"));
                }
                version = true;
                continue;
            }
            let field = AdmissionFieldV1::ALL
                .into_iter()
                .find(|field| field.name() == key)
                .ok_or_else(|| refusal("unknown policy field"))?;
            if stated(field) {
                return Err(refusal(
                    "this field is supplied by the stated MAX_POINTS / 3:1 rules",
                ));
            }
            let slot = values
                .get_mut(field as usize)
                .ok_or_else(|| refusal("field index exceeds the policy schema"))?;
            if slot.is_some() {
                return Err(refusal("duplicate policy field"));
            }
            *slot = Some(resolve_value(field, raw, max_loss_paisa).map_err(|why| refusal(&why))?);
        }
        if !version {
            return Err("research policy is missing policy_version = 1".into());
        }
        let missing: Vec<_> = AdmissionFieldV1::ALL
            .into_iter()
            .filter(|field| {
                !stated(*field) && values.get(*field as usize).is_none_or(Option::is_none)
            })
            .map(AdmissionFieldV1::name)
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "research policy is missing [{}]",
                missing.join(", ")
            ));
        }
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"brutex:research-policy-file:v1\0");
        hasher.update(bytes);
        let mut hex = String::with_capacity(64);
        for byte in hasher.finalize() {
            let _ = write!(hex, "{byte:02x}");
        }
        Ok(Self {
            values,
            provenance: format!(
                "runtime research policy V1; profile BLAKE3(domain + exact file bytes) {hex}; domain=brutex:research-policy-file:v1\\0; risk={max_loss_paisa} paisa; research only"
            ),
        })
    }

    pub(crate) fn value(&self, name: &str) -> Option<&str> {
        AdmissionFieldV1::ALL
            .into_iter()
            .find(|field| field.name() == name)
            .and_then(|field| self.values.get(field as usize).and_then(Option::as_deref))
    }

    pub(crate) fn provenance(&self) -> &str {
        &self.provenance
    }
}

fn stated(field: AdmissionFieldV1) -> bool {
    matches!(
        field,
        AdmissionFieldV1::MaxWorstTradeLossPaisa | AdmissionFieldV1::MinWorstRewardRiskPpm
    )
}

fn signed(field: AdmissionFieldV1) -> bool {
    matches!(
        field,
        AdmissionFieldV1::MinWeakestPeriodReturnPaisa
            | AdmissionFieldV1::MinPessimisticProfitPaisa
            | AdmissionFieldV1::MinOosPessimisticReturnPaisa
    )
}

fn resolve_value(field: AdmissionFieldV1, raw: &str, risk: u64) -> Result<String, String> {
    if matches!(
        field,
        AdmissionFieldV1::RequireWhiteRealityRejection
            | AdmissionFieldV1::RequireRomanoWolfRejection
    ) {
        return match raw {
            "true" | "false" => Ok(raw.to_owned()),
            _ => Err(format!("{} requires true or false", field.name())),
        };
    }
    if let Some(multiplier) = raw
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .and_then(|text| text.strip_prefix("risk*"))
    {
        if !field.name().ends_with("_paisa") {
            return Err("risk multiples are only valid for paisa fields".into());
        }
        let multiplier = multiplier
            .parse::<i64>()
            .map_err(|_| "risk multiplier is not an i64 integer".to_owned())?;
        let result = i128::from(risk) * i128::from(multiplier);
        return if signed(field) {
            i64::try_from(result)
                .map(|value| value.to_string())
                .map_err(|_| "signed risk multiplication overflow".into())
        } else {
            u64::try_from(result)
                .map(|value| value.to_string())
                .map_err(|_| "unsigned risk multiplication overflow or negative value".into())
        };
    }
    if signed(field) {
        raw.parse::<i64>()
            .map(|value| value.to_string())
            .map_err(|_| format!("{} requires an i64 integer", field.name()))
    } else {
        raw.parse::<u64>()
            .map(|value| value.to_string())
            .map_err(|_| format!("{} requires a u64 integer", field.name()))
    }
}

pub(crate) fn command(path: &str, points: &str, out: &mut String) -> u8 {
    match explain(Path::new(path), points) {
        Ok(report) => {
            out.push_str(&report);
            crate::OK
        }
        Err(why) => crate::refuse(out, &why),
    }
}

pub(crate) fn explain(path: &Path, points: &str) -> Result<String, String> {
    let max_points = points
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "MAX_POINTS must be a positive u64 integer".to_owned())?;
    let risk = max_points
        .checked_mul(100)
        .ok_or_else(|| "MAX_POINTS overflows paisa".to_owned())?;
    let profile = ResearchPolicy::read(path, risk)?;
    let provenance = profile.provenance().to_owned();
    let request = crate::ledger_all::LedgerAllRequest {
        vendor: "",
        from: (0, 0),
        to: (0, 0),
        support_ppm: 0,
        max_points,
        root: Path::new(""),
    };
    let (_, gates) =
        crate::ledger_all::resolve_policy(&request, "policy-check", Some(profile), false)?;
    let mut out = format!(
        "RESEARCH POLICY — 37 file settings + 2 stated rules = 39 checks\n{provenance}\n\nThis validates configuration only. No historical strategy has passed these checks here.\nR = {max_points} price points = {risk} paisa per price unit; this is not an account capital allocation.\nIntraday and the 15:10 IST deadline remain separate mandatory execution rules.\nPublished statistical methods do not certify these particular thresholds or future returns.\n\n| Check | Applied limit | What it checks |\n|---|---|---|\n"
    );
    for gate in gates {
        let field = AdmissionFieldV1::ALL
            .into_iter()
            .find(|field| field.name() == gate.name)
            .ok_or_else(|| "admission field has no explanation".to_owned())?;
        let (label, meaning) = explanation(field);
        let limit = display_limit(field, &gate.value)?;
        let _ = writeln!(
            out,
            "| {label} (`{}`) | {limit} | {meaning} |",
            field.name()
        );
    }
    Ok(out)
}

fn display_limit(field: AdmissionFieldV1, raw: &str) -> Result<String, String> {
    use AdmissionFieldV1::{
        MaxLosingTrades, MinConsecutiveWinningStreak, RequireRomanoWolfRejection,
        RequireWhiteRealityRejection,
    };
    if matches!(
        field,
        RequireWhiteRealityRejection | RequireRomanoWolfRejection
    ) {
        return Ok(if raw == "true" {
            "Test must explicitly reject its null"
        } else {
            "Rejection not required; measured evidence still required"
        }
        .to_owned());
    }
    if field == MaxLosingTrades && raw == u64::MAX.to_string() {
        return Ok("No additional absolute count cap; losing rate still applies".into());
    }
    if field == MinConsecutiveWinningStreak && raw == "0" {
        return Ok("0; no winning-streak requirement".into());
    }
    let comparison = if field.name().starts_with("min_") {
        "at least"
    } else {
        "at most"
    };
    let value = raw
        .parse::<i128>()
        .map_err(|_| "invalid explained numeric value".to_owned())?;
    let formatted = if field.name().ends_with("_ppm") {
        let ratio = matches!(
            field,
            AdmissionFieldV1::MinWorstRewardRiskPpm
                | AdmissionFieldV1::MinReturnDrawdownPpm
                | AdmissionFieldV1::MinProfitFactorPpm
        );
        let (scale, unit) = if ratio {
            (1_000_000, "×")
        } else {
            (10_000, "%")
        };
        // Exact integer formatting; no price floats and no loss of ppm precision.
        let places = if ratio { 6 } else { 4 };
        let fraction = format!("{:0width$}", value % scale, width = places);
        let fraction = fraction.trim_end_matches('0');
        let decimal = if fraction.is_empty() {
            String::new()
        } else {
            format!(".{fraction}")
        };
        format!("{}{decimal}{unit}", value / scale)
    } else if field.name().ends_with("_paisa") {
        format!("{raw} paisa ({} price points)", fixed_points(value))
    } else {
        raw.to_owned()
    };
    Ok(format!("{comparison} {formatted}"))
}

fn fixed_points(paisa: i128) -> String {
    let sign = if paisa < 0 { "-" } else { "" };
    let magnitude = paisa.unsigned_abs();
    format!("{sign}{}.{:02}", magnitude / 100, magnitude % 100)
}

#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive match keeps every fixed admission field and its explanation together"
)]
pub(crate) const fn explanation(field: AdmissionFieldV1) -> (&'static str, &'static str) {
    use AdmissionFieldV1 as F;
    match field {
        F::MinSupportHits => (
            "Enough signals",
            "How often the condition matched; signals are not automatically trades.",
        ),
        F::MinIndependentSessions => (
            "Enough trading days",
            "Distinct qualifying sessions carrying support; a day count does not prove statistical independence.",
        ),
        F::MinTrades => (
            "Enough completed trades",
            "Rejects a result based on too few fully closed trades.",
        ),
        F::MaxMaePaisa => (
            "Worst movement against a trade",
            "Limits the measured adverse price excursion, even if the trade later recovered.",
        ),
        F::MinWorstRewardRiskPpm => (
            "Smallest win versus largest loss",
            "The stated 3:1 rule; separate from an average win/loss ratio.",
        ),
        F::MinWinRatePpm => (
            "Winning trade share",
            "Minimum share of pessimistically evaluated trades that won.",
        ),
        F::MinWilsonWinRatePpm => (
            "Cautious win-rate estimate",
            "Minimum Wilson lower confidence bound; it accounts for sample size under the method's assumptions.",
        ),
        F::MinReturnDrawdownPpm => (
            "Return compared with drawdown",
            "Compares pessimistic return with the measured peak-to-trough loss.",
        ),
        F::MinWeakestPeriodReturnPaisa => (
            "Worst required period",
            "Stops a strong total from concealing a weak required evaluation period.",
        ),
        F::MaxPboPpm => (
            "Backtest overfitting estimate",
            "Limits the reported PBO estimate from valid ranking splits; not the probability of future trading success.",
        ),
        F::MaxFwerPValuePpm => (
            "Search-wide adjusted evidence",
            "Limits the family-wise adjusted p-value; the complete declared search population matters.",
        ),
        F::MaxSpaPValuePpm => (
            "Superior predictive ability test",
            "Limits the SPA p-value against the recorded benchmark; not a win probability.",
        ),
        F::MinDecidedFolds => (
            "Enough walk-forward decisions",
            "Requires enough historical train/test windows with a usable decision.",
        ),
        F::MaxAmbiguousFillRatePpm => (
            "Ambiguous candle outcomes",
            "Caps trades whose candle data cannot determine the price-event order.",
        ),
        F::MaxGapAffectedRatePpm => (
            "Gap-affected outcomes",
            "Caps trades marked as affected by missing or discontinuous price evidence.",
        ),
        F::MaxSessionConcentrationPpm => (
            "Dependence on one day",
            "Limits the share of all trades concentrated in a single session.",
        ),
        F::MaxLargestTradeProfitSharePpm => (
            "Dependence on one lucky trade",
            "Limits how much of the recorded profit share comes from the largest trade.",
        ),
        F::MaxDrawdownPaisa => (
            "Largest accumulated setback",
            "Caps the measured peak-to-trough drawdown in price units, not percent of account capital.",
        ),
        F::MaxWorstTradeLossPaisa => (
            "Worst single loss",
            "The stated MAX_POINTS bound converted exactly into paisa.",
        ),
        F::MaxLosingTradeRatePpm => (
            "Losing trade share",
            "Caps the proportion of completed trades that lost.",
        ),
        F::MaxLosingTrades => (
            "Total losing trades",
            "An absolute count ceiling; a rate limit is more comparable across different study lengths.",
        ),
        F::MinPessimisticProfitPaisa => (
            "Conservative total result",
            "Requires the pessimistic modeled result to meet this floor; excluded costs remain excluded.",
        ),
        F::MinWinningTrades => (
            "Enough winning trades",
            "Requires more than a few successes even when the win percentage looks good.",
        ),
        F::MinAverageWinPaisa => (
            "Average winning amount",
            "Minimum average price gain across winning trades.",
        ),
        F::MaxAverageLossPaisa => (
            "Average losing amount",
            "Maximum average loss magnitude across losing trades.",
        ),
        F::MinProfitFactorPpm => (
            "Total wins compared with losses",
            "Gross winning amounts divided by gross losing amounts; different from win rate.",
        ),
        F::MaxConsecutiveLosingStreak => (
            "Longest losing streak",
            "Caps the number of consecutive losses in the recorded order.",
        ),
        F::MinConsecutiveWinningStreak => (
            "Winning-streak requirement",
            "Optional minimum streak; zero avoids treating consecutive lucky wins as required evidence.",
        ),
        F::MinBootstrapDraws => (
            "Enough statistical resamples",
            "Minimum resampling work behind every required family test; this does not add new market observations.",
        ),
        F::MinBootstrapStrategies => (
            "Enough compared strategies",
            "Minimum strategy count in each family test; it never permits omitting other searched candidates.",
        ),
        F::MinBootstrapPeriods => (
            "Enough aligned periods",
            "Minimum common observation periods available to the resampling tests.",
        ),
        F::MinPboContributingFolds => (
            "Enough usable overfitting splits",
            "Minimum ranking splits that actually contributed to the PBO estimate.",
        ),
        F::MaxPboUnrankableFolds => (
            "Unusable overfitting splits",
            "Caps splits that could not rank the compared strategies.",
        ),
        F::MinProfitableOosFolds => (
            "Profitable later test windows",
            "Requires enough walk-forward test windows to have positive results.",
        ),
        F::MinOosPessimisticReturnPaisa => (
            "Conservative later-period result",
            "Minimum aggregate pessimistic out-of-sample result; preserve an untouched later holdout too.",
        ),
        F::MaxWhiteRealityPValuePpm => (
            "White Reality Check evidence",
            "Limits the family Reality Check p-value accounting for the recorded search.",
        ),
        F::RequireWhiteRealityRejection => (
            "White test reached a decision",
            "When enabled, requires an explicit rejection of the benchmark null; measured evidence is still required when disabled.",
        ),
        F::MaxRomanoWolfPValuePpm => (
            "Candidate-specific adjusted evidence",
            "Limits the candidate's Romano-Wolf p-value after accounting for multiple comparisons.",
        ),
        F::RequireRomanoWolfRejection => (
            "Candidate test reached a decision",
            "When enabled, requires an explicit candidate rejection decision from Romano-Wolf; measured evidence is still required when disabled.",
        ),
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test assertions name exact expected parsing outcomes"
)]
mod tests {
    use super::*;
    const PROFILE: &str = include_str!("../../../config/intraday-research-v1.toml");

    #[test]
    fn shipped_profile_resolves_all_37_without_requiring_manual_values() {
        let policy = ResearchPolicy::parse(PROFILE.as_bytes(), 5000).expect("complete profile");
        assert_eq!(policy.values.iter().flatten().count(), 37);
        assert_eq!(policy.value("max_mae_paisa"), Some("5000"));
        // THREE risk units, not ten. The operator's defining criterion, and the
        // number equals min_worst_reward_risk_ppm so one minimum win repays the
        // worst admitted drawdown. Both figures below scale with the risk unit.
        assert_eq!(policy.value("max_drawdown_paisa"), Some("15000"));
        assert_eq!(
            policy.value("min_weakest_period_return_paisa"),
            Some("-5000")
        );
        assert_eq!(policy.value("min_pessimistic_profit_paisa"), Some("25000"));
        assert_eq!(policy.value("require_romano_wolf_rejection"), Some("true"));
        assert_eq!(
            policy.value("max_losing_trades"),
            Some("18446744073709551615")
        );
        assert_eq!(policy.value("min_worst_reward_risk_ppm"), None);
        assert_eq!(policy.value("unknown"), None);
        let other = ResearchPolicy::parse(PROFILE.as_bytes(), 10_000).expect("different risk");
        assert_eq!(other.value("max_drawdown_paisa"), Some("30000"));
        assert_ne!(policy.provenance(), other.provenance());
    }

    #[test]
    fn every_missing_duplicate_unknown_and_stated_field_refuses() {
        for field in AdmissionFieldV1::ALL
            .into_iter()
            .filter(|field| !stated(*field))
        {
            let line = PROFILE
                .lines()
                .find(|line| line.starts_with(&format!("{} =", field.name())))
                .expect("shipped field");
            let missing = PROFILE.replace(line, "");
            assert!(
                ResearchPolicy::parse(missing.as_bytes(), 100)
                    .err()
                    .unwrap()
                    .contains(field.name())
            );
            let duplicate = format!("{PROFILE}\n{line}\n");
            assert!(
                ResearchPolicy::parse(duplicate.as_bytes(), 100)
                    .err()
                    .unwrap()
                    .contains("duplicate")
            );
        }
        for tail in [
            "unknown = 1",
            "max_worst_trade_loss_paisa = 1",
            "min_worst_reward_risk_ppm = 1",
            "[table]",
            "policy_version = 1",
        ] {
            assert!(
                ResearchPolicy::parse(format!("{PROFILE}\n{tail}").as_bytes(), 100).is_err(),
                "{tail}"
            );
        }
        for source in [
            PROFILE.replace("policy_version = 1", "policy_version = 2"),
            PROFILE.replace("policy_version = 1", ""),
        ] {
            assert!(ResearchPolicy::parse(source.as_bytes(), 100).is_err());
        }
    }

    #[test]
    fn exact_domains_overflow_and_malformed_values_refuse() {
        use AdmissionFieldV1::{
            MaxMaePaisa, MinTrades, MinWeakestPeriodReturnPaisa, RequireWhiteRealityRejection,
        };
        for raw in [
            "-1",
            "1.0",
            "NaN",
            "1 = 2",
            "\"risk*-1\"",
            "18446744073709551616",
            "\"risk*9223372036854775808\"",
        ] {
            assert!(resolve_value(MaxMaePaisa, raw, 100).is_err(), "{raw}");
        }
        assert!(resolve_value(MaxMaePaisa, "\"risk*2\"", u64::MAX).is_err());
        assert!(resolve_value(MinWeakestPeriodReturnPaisa, "\"risk*1\"", u64::MAX).is_err());
        assert!(resolve_value(MinWeakestPeriodReturnPaisa, "9223372036854775808", 1).is_err());
        assert!(resolve_value(MinTrades, "\"risk*1\"", 1).is_err());
        assert!(resolve_value(RequireWhiteRealityRejection, "1", 1).is_err());
        assert_eq!(
            resolve_value(RequireWhiteRealityRejection, "false", 1).unwrap(),
            "false"
        );
        assert!(ResearchPolicy::parse(&[0xff], 1).is_err());
        assert!(ResearchPolicy::parse(PROFILE.as_bytes(), 0).is_err());
        assert!(ResearchPolicy::parse(&vec![b' '; 16_385], 1).is_err());
        assert!(ResearchPolicy::parse(PROFILE.as_bytes(), u64::MAX).is_err());
    }

    #[test]
    fn file_bytes_and_risk_are_audited_even_if_numeric_values_match() {
        let first = ResearchPolicy::parse(PROFILE.as_bytes(), 100).unwrap();
        let identical = ResearchPolicy::parse(PROFILE.as_bytes(), 100).unwrap();
        let commented = ResearchPolicy::parse(
            format!("{PROFILE}\n# explanation changed\n").as_bytes(),
            100,
        )
        .unwrap();
        assert_eq!(first.provenance(), identical.provenance());
        assert_eq!(first.values, commented.values);
        assert_ne!(first.provenance(), commented.provenance());
    }

    #[test]
    fn human_limits_preserve_exact_rates_ratios_prices_and_special_rules() {
        use AdmissionFieldV1 as F;
        assert_eq!(
            display_limit(F::MaxFwerPValuePpm, "50001").unwrap(),
            "at most 5.0001%"
        );
        assert_eq!(
            display_limit(F::MinProfitFactorPpm, "1500001").unwrap(),
            "at least 1.500001×"
        );
        assert_eq!(
            display_limit(F::MinWeakestPeriodReturnPaisa, "-1").unwrap(),
            "at least -1 paisa (-0.01 price points)"
        );
        assert_eq!(fixed_points(i128::from(i64::MIN)), "-92233720368547758.08");
        assert!(display_limit(F::MinTrades, "invalid").is_err());
        for field in F::ALL {
            let (label, meaning) = explanation(field);
            assert!(!label.is_empty() && !meaning.is_empty());
        }
    }

    #[test]
    fn executable_policy_check_validates_without_reading_markets_or_overrides()
    -> Result<(), Box<dyn std::error::Error>> {
        let _serial = crate::knobs::serially();
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let path = scratch.0.join("policy.toml");
        std::fs::write(&path, PROFILE)?;
        crate::knobs::clear_all();
        crate::knobs::set("BRUTEX_ADMIT_MIN_TRADES", "broken");
        let mut report = String::new();
        let args = [
            "policy-check",
            path.to_str().ok_or("test path UTF-8")?,
            "50",
        ]
        .map(str::to_owned);
        let code = crate::run(&args, &mut report);
        crate::knobs::clear_all();
        assert_eq!(code, 0, "{report}");
        assert_eq!(
            report.lines().filter(|line| line.starts_with("| ")).count(),
            40
        );
        assert!(
            report.contains("at least 40")
                && report.contains("profile BLAKE3(domain + exact file bytes)")
        );
        assert!(!crate::is_sweep_command("policy-check"));
        for raw in ["0", "-1", "invalid", "18446744073709551615"] {
            assert!(explain(&path, raw).is_err());
        }
        std::fs::write(
            &path,
            PROFILE.replace("min_win_rate_ppm = 250000", "min_win_rate_ppm = 1000001"),
        )?;
        assert!(
            explain(&path, "50")
                .err()
                .ok_or("invalid policy accepted")?
                .contains("admission policy refused")
        );
        Ok(())
    }

    #[test]
    fn disk_reader_refuses_missing_directory_symlink_and_oversized_input()
    -> Result<(), Box<dyn std::error::Error>> {
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let path = scratch.0.join("policy.toml");
        assert!(ResearchPolicy::read(&path, 100).is_err());
        assert!(ResearchPolicy::read(&scratch.0, 100).is_err());
        std::fs::write(&path, PROFILE)?;
        assert_eq!(
            ResearchPolicy::read(&path, 100)?
                .values
                .iter()
                .flatten()
                .count(),
            37
        );
        #[cfg(unix)]
        {
            let link = scratch.0.join("link.toml");
            std::os::unix::fs::symlink(&path, &link)?;
            assert!(ResearchPolicy::read(&link, 100).is_err());
        }
        std::fs::write(&path, vec![b' '; 16_385])?;
        assert!(ResearchPolicy::read(&path, 100).is_err());
        Ok(())
    }

    #[test]
    fn selected_runtime_profile_reaches_v6_preflight_and_preserves_override_refusals()
    -> Result<(), Box<dyn std::error::Error>> {
        const CHILD: &str = "BRUTEX_RESEARCH_POLICY_TEST_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let _serial = crate::knobs::serially();
            crate::knobs::clear_all();
            let path = std::env::var_os(SETTING).ok_or("selected test path missing")?;
            let path = Path::new(&path);
            let root = path.with_file_name("no-created-authority");
            let request = crate::ledger_all::LedgerAllRequest {
                vendor: "dhan",
                from: (2025, 4),
                to: (2025, 5),
                support_ppm: 1000,
                max_points: 50,
                root: &root,
            };
            let report = crate::ledger_v6::ledger_v6(&request);
            assert!(report.contains("all 39 configured"), "{report}");
            assert!(
                report.contains("strict input configuration is unavailable"),
                "{report}"
            );
            assert!(
                !root.exists(),
                "physical preflight cannot create authority roots"
            );
            crate::knobs::set("BRUTEX_ADMIT_MIN_TRADES", "300");
            let (policy, active) = crate::ledger_all::admission_policy(&request, "policy-check")?;
            assert_eq!(policy.values().min_trades, 300);
            let mut census = String::new();
            crate::ledger_all::render_gate_census(&mut census, &active);
            assert!(census.contains("set by its BRUTEX_ADMIT_ knob"));
            assert!(census.contains("profile BLAKE3(domain + exact file bytes)"));
            crate::knobs::set("BRUTEX_ADMIT_MIN_TRADES", "broken");
            let error = crate::ledger_all::admission_policy(&request, "policy-check")
                .err()
                .ok_or("bad override accepted")?;
            assert!(error.contains("BRUTEX_ADMIT_MIN_TRADES="));
            crate::knobs::set("BRUTEX_ADMIT_MIN_TRADES", "300");
            std::fs::write(path, "policy_version = 2\n")?;
            assert!(
                crate::ledger_all::admission_policy(&request, "policy-check")
                    .err()
                    .ok_or("bad file ignored")?
                    .contains("policy_version")
            );
            crate::knobs::clear_all();
            return Ok(());
        }
        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
        let path = scratch.0.join("profile.toml");
        std::fs::write(&path, PROFILE)?;
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", "research_policy::tests::selected_runtime_profile_reaches_v6_preflight_and_preserves_override_refusals", "--nocapture"])
            .env(CHILD, "1").env(SETTING, &path)
            .env_remove("BRUTEX_CHECKSUM_RECEIPTS")
            .env_remove("BRUTEX_CHECKSUM_MAX_BYTES")
            .env_remove("BRUTEX_CHECKSUM_MAX_RECORDS")
            .output()?;
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }
}
