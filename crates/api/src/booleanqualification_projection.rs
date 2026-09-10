//! Qualification uses the same saved admission fields, with explicit later evidence.
use super::{Asked, admission_projection, base, float, put, statistics_row, summary};
use cli::boolean_evidence::{AdmissionRow, Qualification, QualificationRow};
use serde_json::{Value, json};
#[cfg(test)]
#[path = "booleanqualification_projection_tests.rs"]
mod tests;

pub(super) fn project(reader: &Qualification, asked: &Asked) -> Result<Value, String> {
    reader.require_current()?;
    let pin = reader.completion_digest();
    if asked.completion.is_some_and(|expected| expected != pin) {
        return Err("qualification completion pin differs; no replacement page returned".into());
    }
    if matches!(asked.kind.as_str(), "index-weeks" | "index-days") {
        return super::index_consistency_projection::page(reader, asked);
    }
    let rows = reader.rows(pin, asked.offset, asked.limit)?;
    let stats = reader.original().statistics();
    let observations = stats.rows(stats.completion_digest(), asked.offset, asked.limit)?;
    if rows.len() != observations.len() {
        return Err("qualification page differs from original statistics extent".into());
    }
    let mut rendered = Vec::new();
    for (n, (row, original)) in rows.iter().zip(&observations).enumerate() {
        let index = asked.offset + n;
        if row.original != original.identity
            || row.family != original.source
            || row.coordinate != original.coordinate
        {
            return Err("qualification row differs from original coordinate".into());
        }
        let mut value = admission_projection::row(
            index,
            &AdmissionRow {
                identity: row.original,
                source_index: index,
                values: row.values,
                verdict: row.verdict,
            },
        )?;
        put(
            &mut value,
            "statistics",
            statistics_row(stats, index, original)?,
        )?;
        put(&mut value, "qualification", row_detail(reader, row)?)?;
        put(
            &mut value,
            "index_consistency",
            super::index_consistency_projection::setting(reader, index, row.verdict.status())?,
        )?;
        rendered.push(value);
    }
    let mut body = base(asked, pin, reader.row_count(), rendered)?;
    put(&mut body, "summary", summary(stats.summary())?)?;
    put(
        &mut body,
        "statistics_identity",
        json!(crate::server::hex32(stats.identity())),
    )?;
    put(
        &mut body,
        "statistics_completion",
        json!(crate::server::hex32(stats.completion_digest())),
    )?;
    put(
        &mut body,
        "policy",
        admission_projection::policy(&reader.policy()),
    )?;
    put(
        &mut body,
        "admitted_bytes",
        json!(reader.admitted_bytes().to_string()),
    )?;
    let allocation = reader.allocation();
    let threshold = allocation.threshold();
    let minimum = allocation.minimum_draws();
    let [draws, seed, block_length] = reader.procedure();
    put(
        &mut body,
        "qualification",
        json!({
            "scope":crate::server::hex32(reader.scope()),"unit":crate::server::hex32(reader.unit()),
            "original":{"identity":crate::server::hex32(reader.original().identity()),"completion":crate::server::hex32(reader.original().completion_digest())},
            "fold_count":reader.fold_count().to_string(),"procedure":{"draws":draws.to_string(),"seed":seed.to_string(),"block_length":block_length.to_string()},
            "allocation":{"digest":crate::server::hex32(allocation.digest()),"batch":allocation.batch().to_string(),"batches":allocation.batches().to_string(),"rung":allocation.rung().to_string(),"rungs":allocation.rungs().to_string(),"alpha_ppm":allocation.alpha_ppm().to_string(),"threshold":{"numerator":threshold.numerator().to_string(),"denominator":threshold.denominator().to_string()},"minimum_draws":minimum.map(|n|n.to_string()),"draw_resolution_met":minimum.is_some_and(|n|u128::from(draws)>=n)},
            "later":reader.later().iter().enumerate().map(|(index,later)|json!({"index":index.to_string(),"identity":crate::server::hex32(later.identity()),"completion":crate::server::hex32(later.completion_digest()),"coordinates":later.coordinate_count().to_string(),"sessions":later.sessions().len().to_string(),"instrument":later.family().instrument().to_string()})).collect::<Vec<_>>()
        }),
    )?;
    put(
        &mut body,
        "scope",
        json!(
            "Saved fixed-training later qualification for every declared coordinate. Training PBO and later execution/profitability are different evidence. The finite multiplicity allocation and bootstrap draw resolution are explicit. Returns exclude costs; this view grants no Selection V6, live-trading or future-profitability approval and does not reread current raw-market files."
        ),
    )?;
    reader.require_current()?;
    Ok(body)
}

fn row_detail(reader: &Qualification, row: &QualificationRow) -> Result<Value, String> {
    let later = reader
        .later()
        .get(row.family)
        .ok_or("qualification later family missing")?;
    let folds = if let Some(folds) = &row.folds {
        Some(json!({
            "plan":crate::server::hex32(folds.plan_digest()),"selected":crate::server::hex32(folds.selected_digest()),"anchor":crate::server::hex32(folds.anchor_digest()),
            "later":crate::server::hex32(folds.later_digest()),"training_run":crate::server::hex32(folds.training_run_id()),"later_run":crate::server::hex32(folds.later_run_id()),"resolution":crate::server::hex32(folds.resolution_digest()),
            "ordinal":folds.ordinal().to_string(),"training_last_day":folds.training_last_day().to_string(),"first_day":folds.requested().first_day().to_string(),"last_day":folds.requested().last_day().to_string(),"execution_refusal_bits":folds.execution_refusal_bits().bits().to_string(),
            "decided":folds.decided_folds().to_string(),"profitable":folds.profitable_oos_folds().to_string(),"return_paisa":folds.aggregate_oos_paisa()?.to_string(),
            "rows":folds.folds().iter().enumerate().map(|(index,fold)|json!({"index":index.to_string(),"first_day":fold.window.first_day().to_string(),"last_day":fold.window.last_day().to_string(),"sessions":fold.sessions.to_string(),"trades":fold.trades.to_string(),"wins":fold.wins.to_string(),"return_paisa":fold.return_paisa.to_string()})).collect::<Vec<_>>()
        }))
    } else {
        None
    };
    Ok(
        json!({"identity":crate::server::hex32(row.later),"family":row.family.to_string(),"coordinate":row.coordinate.to_string(),"source":{"identity":crate::server::hex32(later.identity()),"completion":crate::server::hex32(later.completion_digest())},"romano":romano(row.romano)?,"folds":folds}),
    )
}

fn romano(
    [
        class,
        numerator,
        denominator,
        present,
        strategy,
        rank,
        statistic,
        exceedances,
        initial_n,
        initial_d,
        adjusted_n,
        adjusted_d,
    ]: [u64; 12],
) -> Result<Value, String> {
    let classification = match class {
        1 => "measured-positive",
        2 => "conservative-zero",
        3 => "conservative-nonpositive",
        _ => return Err("qualification Romano-Wolf classification unknown".into()),
    };
    let shared = match present {
        0 => None,
        1 => Some(
            json!({"strategy":strategy.to_string(),"stepdown_rank":rank.to_string(),"statistic":float(statistic),"strict_exceedances":exceedances.to_string(),"initial":{"numerator":initial_n.to_string(),"denominator":initial_d.to_string()},"adjusted":{"numerator":adjusted_n.to_string(),"denominator":adjusted_d.to_string()}}),
        ),
        _ => return Err("qualification Romano-Wolf presence unknown".into()),
    };
    Ok(
        json!({"classification":classification,"probability":{"numerator":numerator.to_string(),"denominator":denominator.to_string()},"shared":shared}),
    )
}
