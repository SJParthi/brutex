//! Immutable typed preparation shared by text catalogs and campaign batches.
use super::*;
use rayon::prelude::*;
use runner::admission::AdmissionPolicyV1;
use runner::exit_grid_policy::ExitGridPolicyV1;
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(crate) struct Input<'a> {
    pub vendor: &'a str,
    pub symbols: &'a str,
    pub from: (u16, u8),
    pub to: (u16, u8),
    pub horizon: runner::outcome::Horizon,
    pub max_points: u64,
    pub output: &'a Path,
}

pub(crate) struct Prepared {
    #[cfg(test)]
    pub(crate) generated_fixture: bool,
    pub(crate) scope: ResearchScopeV1,
    pub(crate) source: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) vendor: brutex_core::vendor::Vendor,
    pub(crate) from: (u16, u8),
    pub(crate) to: (u16, u8),
    pub(crate) horizon: runner::outcome::Horizon,
    pub(crate) strict: StrictConfig,
    shared: StrictConfig,
    long: ExitGridPolicyV1,
    short: ExitGridPolicyV1,
    policy: AdmissionPolicyV1,
    plan: StatisticsPlan,
    widths: indicators::evaluator::Widths,
    thresholds: indicators::pattern::Thresholds,
}

#[derive(Clone, Copy)]
pub(crate) enum Stage<'a> {
    Families(&'a [Result<candidate::CommittedBooleanFamilyV1, String>]),
    Statistics(&'a statistics::CommittedBooleanStatisticsV1),
    Admission(&'a statistics::admission::CommittedBooleanAdmissionV1),
}

impl Prepared {
    pub(crate) const fn policy(&self) -> AdmissionPolicyV1 {
        self.policy
    }
    pub(crate) const fn policy_ref(&self) -> &AdmissionPolicyV1 {
        &self.policy
    }
    /// The exact resolved exits held by this preparation, without re-reading settings.
    pub(crate) fn exit_policy_digests(&self) -> [[u8; 32]; 2] {
        [self.long.digest(), self.short.digest()]
    }
    pub(crate) const fn procedure(
        &self,
    ) -> crate::population_statistics_v2::PopulationStatisticsProcedureV2 {
        self.plan.procedure
    }
    #[cfg(test)]
    pub(crate) fn generated_fixture(&mut self) -> Result<(), String> {
        self.generated_fixture = true;
        self.long = candidate::tests::policy(runner::excursion::Side::Long)?;
        self.short = candidate::tests::policy(runner::excursion::Side::Short)?;
        Ok(())
    }
    pub(crate) fn new(input: Input<'_>, out: &mut String) -> Result<Self, String> {
        let vendor = crate::parse_vendor(input.vendor)?;
        crate::audited_range_command::validate_boolean_runtime().map_err(|why| {
            format!(
                "Boolean catalog rejects unsupported or invalid legacy audit settings: {}",
                why.invalid.join(", ")
            )
        })?;
        let request = LedgerAllRequest {
            vendor: input.vendor,
            from: input.from,
            to: input.to,
            support_ppm: 0,
            max_points: input.max_points,
            root: input.output,
        };
        let (policy, active) = admission_policy(&request, VERB)?;
        let strict = StrictConfig::from_env().map_err(|why| why.to_string())?;
        let scope = scope(input.symbols)?;
        let plan = StatisticsPlan::new(scope.families().len(), &strict)?;
        if input.from > input.to || input.horizon.as_bars() == 0 {
            return Err("ordered month span and positive horizon required".into());
        }
        for (year, month) in [input.from, input.to] {
            pull::session::Day::new(year, month, 1).map_err(|why| why.to_string())?;
        }
        let families = scope.families().len() as u64;
        let shared = StrictConfig::from_values(
            Some(strict.receipt_root().as_os_str().to_owned()),
            Some((strict.max_bytes() / families).to_string().into()),
            Some((strict.max_records() / families).to_string().into()),
        )
        .map_err(|why| format!("shared family input budget: {why}"))?;
        out.push_str(crate::STORED_PROVENANCE);
        out.push_str("\nEXPLICIT BOOLEAN CATALOG RESEARCH. This is the complete supplied catalog, not exhaustive Boolean grammar search or Selection V6. Intraday only;15:10IST deadline.\n");
        let _ = writeln!(
            out,
            "Saved evidence root: {}. Dashboard visibility requires the observing server's BRUTEX_STORE to name this output root; another server root is not searched.",
            input.output.display()
        );
        render_gate_census(out, &active);
        Ok(Self {
            #[cfg(test)]
            generated_fixture: false,
            scope,
            source: crate::store_root()?,
            output: input.output.to_path_buf(),
            vendor,
            from: input.from,
            to: input.to,
            horizon: input.horizon,
            strict,
            shared,
            long: exit_policy(runner::excursion::Side::Long)?,
            short: exit_policy(runner::excursion::Side::Short)?,
            policy,
            plan,
            widths: indicators::evaluator::Widths::pinned()
                .map_err(|why| format!("widths: {why:?}"))?,
            thresholds: indicators::pattern::Thresholds::CLASSICAL,
        })
    }

    pub(crate) fn policy_digest(&self) -> [u8; 32] {
        let mut hash = brutex_core::blake3::Hasher::new();
        hash.update(b"brutex-boolean-campaign-prepared-v1\0");
        hash.update(&self.policy.digest());
        hash.update(&self.scope.digest());
        for value in [
            self.plan.procedure.draws(),
            self.plan.procedure.seed(),
            self.plan.procedure.block_length(),
            self.strict.max_bytes(),
            self.strict.max_records(),
        ] {
            hash.update(&value.to_le_bytes());
        }
        hash.finalize()
    }

    pub(crate) fn request<'a>(
        &'a self,
        rung: &'a str,
        programs: &'a [Expression],
        underlying: &'a str,
    ) -> candidate::Request<'a> {
        candidate::Request {
            store: &self.source,
            output: &self.output,
            vendor: self.vendor,
            underlying,
            rung,
            from: self.from,
            to: self.to,
            horizon: self.horizon,
            programs,
            long: &self.long,
            short: &self.short,
            inputs: &self.shared,
            bounds: candidate::Bounds {
                programs: programs.len() as u64,
                coordinates: self.shared.max_records(),
                trades: self.shared.max_records(),
                bytes: self.shared.max_bytes(),
            },
            widths: self.widths,
            thresholds: self.thresholds,
        }
    }

    pub(crate) fn run(
        &self,
        rung: &str,
        programs: &[Expression],
        out: &mut String,
        mut record: impl FnMut(Stage<'_>) -> Result<(), String>,
    ) -> Result<statistics::admission::CommittedBooleanAdmissionV1, String> {
        if !crate::ledger_all::LEDGER_RUNGS.contains(&rung) {
            return Err("intraday rung required".into());
        }
        let lanes = std::thread::available_parallelism()
            .map_err(|why| why.to_string())?
            .get()
            .min(self.scope.families().len());
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(lanes)
            .build()
            .map_err(|why| why.to_string())?;
        let _ = writeln!(
            out,
            "\n{} families × {} programs × both directions × all resolved exit cells; {} parallel family workers. Capture ceilings: {} bytes and {} records per family; strict source-file ceilings use these same values. These are physical admission limits, not a total process RAM guarantee.\nProfile minimum trades: {}. Individual family completion is not whole-cohort completion.",
            self.scope.families().len(),
            programs.len(),
            lanes,
            self.shared.max_bytes(),
            self.shared.max_records(),
            self.policy.values().min_trades
        );
        let results: Vec<_> = pool.install(|| {
            self.scope
                .families()
                .par_iter()
                .map(|family| {
                    let key = family.instrument();
                    self.produce(self.request(rung, programs, key.underlying.as_str()))
                })
                .collect()
        });
        render_family_receipts(&results, out);
        for family in results.iter().filter_map(|value| value.as_ref().ok()) {
            family.require_current()?;
        }
        record(Stage::Families(&results))?;
        let families = collect_families(&self.scope, results)?;
        super::finish_statistics(
            &self.output,
            families,
            &self.policy,
            &self.plan,
            out,
            &mut record,
        )
    }

    fn produce(
        &self,
        request: candidate::Request<'_>,
    ) -> Result<candidate::CommittedBooleanFamilyV1, String> {
        if request.vendor != self.vendor || request.store != self.source {
            return Err("candidate request differs from prepared source/feed".into());
        }
        #[cfg(test)]
        if self.generated_fixture {
            return candidate::produce_campaign_fixture(request);
        }
        candidate::produce(request)
    }
}
