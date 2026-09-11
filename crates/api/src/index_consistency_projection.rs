//! Exact persisted daily/week observations, with institutional checks retained.
use super::Asked;
use cli::boolean_evidence::Qualification;
use cli::index_consistency::{Evaluation, Policy, Reason, Session, Summary, Week};
use serde_json::{Value, json};

pub(crate) fn policy(value: Policy) -> Value {
    json!({
        "schema_version":1,"policy_digest":hex(value.digest()),
        "instruments":["NSE-NIFTY","NSE-BANKNIFTY"],
        "minimum_winning_day_numerator":value.winning_day_numerator().to_string(),
        "minimum_winning_day_denominator":value.winning_day_denominator().to_string(),
        "minimum_week_winning_days":value.min_weekly_wins().to_string(),
        "maximum_week_losing_days":value.max_weekly_losses().to_string(),
        "maximum_losing_day_streak":value.max_losing_day_streak().to_string(),
        "pnl_basis":"pessimistic_gross_paisa","zero_days_reset_streak":false,"costs_included":false,
        "evaluated_scope":"training_and_later_with_calendar_gap_check",
        "day_rule":value.day_rule().as_str(),"ratio_basis":value.ratio_basis().as_str()
    })
}
pub(super) fn setting(
    reader: &Qualification,
    index: usize,
    institutional: cli::boolean_evidence::AdmissionStatusV1,
) -> Result<Value, String> {
    let pin = reader.completion_digest();
    let qualification = json!({"identity":hex(reader.identity()),"completion":hex(pin)});
    let Some(record) = reader.index_consistency(pin, index)? else {
        return Ok(
            json!({"schema_version":1,"state":"not_assessed","policy_digest":null,"policy":null,"receipt":null,
            "qualification":qualification,"setting_index":index.to_string(),"combined_qualifies":null,
            "evaluation":null,"days_count":"0","weeks_count":"0","evaluated_scope":null,"periods":null}),
        );
    };
    let receipt = reader
        .index_consistency_receipt()
        .ok_or("consistency assessment has no receipt")?;
    saved_record(
        reader.identity(),
        pin,
        index,
        record,
        receipt,
        institutional,
    )
}
/// Pure serializer shared by both independently authenticated native readers.
pub(crate) fn saved_record(
    identity: [u8; 32],
    pin: [u8; 32],
    index: usize,
    record: &cli::index_consistency_store::Record,
    receipt: cli::index_consistency_store::Receipt,
    institutional: cli::boolean_evidence::AdmissionStatusV1,
) -> Result<Value, String> {
    let qualification = json!({"identity":hex(identity),"completion":hex(pin)});
    let evaluation = &record.evaluation;
    let training_days = record.training_session_count;
    let later_days = record
        .sessions
        .len()
        .checked_sub(training_days)
        .ok_or("saved training day prefix exceeds full extent")?;
    Ok(
        json!({"schema_version":1,"state":record.outcome().as_str(),"policy_digest":hex(evaluation.policy.digest()),"policy":policy(evaluation.policy),
            "receipt":{"identity":hex(receipt.identity),"completion":hex(receipt.completion)},"qualification":qualification,
            "setting_index":index.to_string(),"combined_qualifies":record.combined_qualifies(institutional),
            "evaluation":{"instrument":evaluation.family.instrument().to_string(),"first_day":evaluation.first_day.to_string(),"last_day":evaluation.last_day.to_string(),
                "reasons_bits":evaluation.reasons.to_string(),"reasons":reasons(evaluation.reasons),"first_issue_day":evaluation.first_issue_day.map(|day|day.to_string()),
                "calendar_digest":hex(evaluation.calendar_digest),"sessions_digest":hex(evaluation.sessions_digest),"weeks_digest":hex(evaluation.weeks_digest),"summary":summary(evaluation.summary)},
            "days_count":record.sessions.len().to_string(),"weeks_count":evaluation.week_count.to_string(),
            "evaluated_scope":"training_and_later_with_calendar_gap_check",
            "periods":{"training":period(&record.training,training_days),"later":period(&record.later,later_days),"full":period(evaluation,record.sessions.len())}
        }),
    )
}
fn period(evaluation: &Evaluation, days: usize) -> Value {
    json!({"state":evaluation.outcome.as_str(),"first_day":evaluation.first_day.to_string(),"last_day":evaluation.last_day.to_string(),
        "reasons_bits":evaluation.reasons.to_string(),"reasons":reasons(evaluation.reasons),"first_issue_day":evaluation.first_issue_day.map(|day|day.to_string()),
        "days_count":days.to_string(),"weeks_count":evaluation.week_count.to_string(),"summary":summary(evaluation.summary)})
}
fn reasons(bits: u64) -> Vec<&'static str> {
    [
        Reason::WinningDayRatio,
        Reason::WeeklyWins,
        Reason::WeeklyLosses,
        Reason::LosingDayStreak,
        Reason::NoCompleteWeek,
        Reason::NoEligibleSession,
        Reason::MissingSession,
        Reason::UnmeasuredCalendar,
        Reason::InvalidSpan,
        Reason::InvalidOrder,
        Reason::UnexpectedSession,
        Reason::InvalidCountOrReturn,
        Reason::ArithmeticOverflow,
        Reason::AllocationRefused,
    ]
    .into_iter()
    .filter(|reason| bits & reason.mask() != 0)
    .map(Reason::as_str)
    .collect()
}
fn summary(value: Summary) -> Value {
    json!({"calendar_days":value.calendar_days.to_string(),"eligible_days":value.eligible_days.to_string(),
        "observed_days":value.observed_days.to_string(),"winning_days":value.winning_days.to_string(),"losing_days":value.losing_days.to_string(),
        "zero_days":value.zero_days.to_string(),"no_trade_days":value.no_trade_days.to_string(),"missing_days":value.missing_days.to_string(),
        "unmeasured_days":value.unmeasured_days.to_string(),"excluded_days":value.excluded_days.to_string(),"closed_days":value.closed_days.to_string(),
        "weekend_sessions":value.weekend_sessions.to_string(),"trades":value.trades.to_string(),"pessimistic_paisa":value.pessimistic_paisa.to_string(),
        "longest_losing_streak":value.longest_losing_streak.to_string(),"complete_weeks":value.complete_weeks.to_string(),
        "passing_weeks":value.passing_weeks.to_string(),"failing_weeks":value.failing_weeks.to_string(),"short_weeks":value.short_weeks.to_string(),
        "partial_weeks":value.partial_weeks.to_string(),"unmeasured_weeks":value.unmeasured_weeks.to_string()})
}
pub(super) fn page(reader: &Qualification, asked: &Asked) -> Result<Value, String> {
    let setting = asked
        .setting
        .ok_or("index consistency page requires exact setting")?;
    let pin = asked
        .completion
        .ok_or("index consistency page requires exact qualification completion")?;
    let record = reader
        .index_consistency(pin, setting)?
        .ok_or("historical qualification has no index consistency assessment")?;
    let receipt = reader
        .index_consistency_receipt()
        .ok_or("index consistency receipt absent")?;
    let (evaluation, days) = match asked.period.as_str() {
        "full" => (&record.evaluation, record.sessions.as_slice()),
        "training" => (
            &record.training,
            record
                .sessions
                .get(..record.training_session_count)
                .ok_or("index training day extent missing")?,
        ),
        "later" => (
            &record.later,
            record
                .sessions
                .get(record.training_session_count..)
                .ok_or("index later day extent missing")?,
        ),
        _ => return Err("index consistency period unknown".into()),
    };
    let total = if asked.kind == "index-weeks" {
        evaluation.weeks.len()
    } else {
        days.len()
    };
    if asked.offset > total || !(1..=256).contains(&asked.limit) {
        return Err("index consistency page is outside saved extent".into());
    }
    let end = asked.offset.saturating_add(asked.limit).min(total);
    let rows: Vec<_> = if asked.kind == "index-weeks" {
        evaluation
            .weeks
            .get(asked.offset..end)
            .ok_or("index week page outside saved extent")?
            .iter()
            .enumerate()
            .map(|(n, row)| week(asked.offset + n, row))
            .collect()
    } else {
        days.get(asked.offset..end)
            .ok_or("index day page outside saved extent")?
            .iter()
            .enumerate()
            .map(|(n, row)| day(asked.offset + n, *row, evaluation.policy))
            .collect()
    };
    reader.require_current()?;
    Ok(
        json!({"schema_version":1,"status":"saved","model":"qualification","kind":asked.kind,"identity":hex(reader.identity()),"completion":hex(pin),
        "receipt":{"identity":hex(receipt.identity),"completion":hex(receipt.completion)},"period":asked.period,
        "setting":setting.to_string(),"total":total.to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"rows":rows,"refusal":null}),
    )
}
/// One saved day, with its class under the record's OWN day rule -- D-0605.
/// The browser labelled days by V1's sign test because nothing else was sent.
pub(crate) fn day(index: usize, value: Session, policy: Policy) -> Value {
    let class = if value.trades == 0 {
        "no_trade"
    } else {
        match policy.day_rule().classify(value) {
            std::cmp::Ordering::Greater => "winning",
            std::cmp::Ordering::Less => "losing",
            std::cmp::Ordering::Equal => "scratch",
        }
    };
    json!({"index":index.to_string(),"day":value.day.to_string(),"pessimistic_paisa":value.pessimistic_paisa.to_string(),"trades":value.trades.to_string(),"class":class})
}
pub(crate) fn week(index: usize, value: &Week) -> Value {
    json!({"index":index.to_string(),"monday":value.monday.to_string(),"kind":value.kind.as_str(),"state":value.outcome.as_str(),
        "weekday_days":value.weekday_days.to_string(),"eligible_days":value.eligible_days.to_string(),"observed_days":value.observed_days.to_string(),
        "winning_days":value.winning_days.to_string(),"losing_days":value.losing_days.to_string(),"zero_days":value.zero_days.to_string(),
        "no_trade_days":value.no_trade_days.to_string(),"missing_days":value.missing_days.to_string(),"unmeasured_days":value.unmeasured_days.to_string(),
        "excluded_days":value.excluded_days.to_string(),"closed_weekdays":value.closed_weekdays.to_string(),"weekend_sessions":value.weekend_sessions.to_string(),
        "trades":value.trades.to_string(),"pessimistic_paisa":value.pessimistic_paisa.to_string()})
}
fn hex(value: [u8; 32]) -> String {
    crate::server::hex32(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_descriptor_reads_the_canonical_rust_policy_and_exact_scope() {
        // Rendered per POLICY, not from a constant inside the renderer: the same
        // helper serves V1 on the boolean page and V3 on the single-stop page,
        // and it described V3 evidence with V1 numbers until D-0600.
        let expected = Policy::V1;
        let rendered = policy(expected);
        assert_eq!(
            rendered.get("policy_digest"),
            Some(&json!(hex(expected.digest())))
        );
        assert_eq!(
            rendered.get("minimum_winning_day_numerator"),
            Some(&json!(expected.winning_day_numerator().to_string()))
        );
        assert_eq!(
            rendered.get("minimum_winning_day_denominator"),
            Some(&json!(expected.winning_day_denominator().to_string()))
        );
        assert_eq!(
            rendered.get("maximum_losing_day_streak"),
            Some(&json!(expected.max_losing_day_streak().to_string()))
        );
        assert_eq!(
            rendered.get("instruments"),
            Some(&json!(["NSE-NIFTY", "NSE-BANKNIFTY"]))
        );
        assert_eq!(
            rendered.get("evaluated_scope"),
            Some(&json!("training_and_later_with_calendar_gap_check"))
        );
        // D-0605: the day rule and ratio basis travel with the numbers, and the
        // browser checks this exact key set.
        assert_eq!(rendered.get("day_rule"), Some(&json!("pessimistic_sign")));
        assert_eq!(rendered.get("ratio_basis"), Some(&json!("eligible_days")));
        assert_eq!(rendered.as_object().map(serde_json::Map::len), Some(14));
        let v4 = policy(Policy::V4);
        assert_eq!(v4.get("day_rule"), Some(&json!("sign_under_both_readings")));
        assert_eq!(v4.get("ratio_basis"), Some(&json!("decided_days")));
        assert_eq!(
            v4.get("maximum_losing_day_streak"),
            Some(&json!(u64::MAX.to_string()))
        );
    }
    #[test]
    fn each_day_is_classed_under_its_own_record_s_rule() {
        // D-0605: the page cannot class a day, because the rule that decides it
        // belongs to the record's policy version.
        let row = |pessimistic, optimistic, trades| Session {
            day: 20_000,
            pessimistic_paisa: pessimistic,
            optimistic_paisa: optimistic,
            trades,
        };
        for (value, v1, v3, v4) in [
            (row(-5, 900, 2), "losing", "scratch", "scratch"),
            (row(1, 201, 2), "winning", "scratch", "winning"),
            (row(0, 0, 2), "scratch", "scratch", "scratch"),
            (row(-500, -1, 2), "losing", "losing", "losing"),
            (row(0, 0, 0), "no_trade", "no_trade", "no_trade"),
        ] {
            for (version, want) in [(Policy::V1, v1), (Policy::V3, v3), (Policy::V4, v4)] {
                assert_eq!(
                    day(0, value, version).get("class"),
                    Some(&json!(want)),
                    "{version:?}"
                );
            }
        }
    }
    #[test]
    fn integer_projection_keeps_large_counts_signed_money_and_distinct_zeros_exact() {
        let value = summary(Summary {
            trades: u64::MAX,
            pessimistic_paisa: i64::MIN,
            no_trade_days: 3,
            zero_days: 4,
            ..Summary::default()
        });
        assert_eq!(value.get("trades"), Some(&json!(u64::MAX.to_string())));
        assert_eq!(
            value.get("pessimistic_paisa"),
            Some(&json!(i64::MIN.to_string()))
        );
        assert_eq!(value.get("no_trade_days"), Some(&json!("3")));
        assert_eq!(value.get("zero_days"), Some(&json!("4")));
        let value = day(
            5,
            Session {
                day: 20_000,
                pessimistic_paisa: i64::MAX,
                optimistic_paisa: i64::MAX,
                trades: u64::MAX,
            },
            Policy::V1,
        );
        assert_eq!(
            value.get("pessimistic_paisa"),
            Some(&json!(i64::MAX.to_string()))
        );
        assert_eq!(value.get("index"), Some(&json!("5")));
        assert_eq!(
            reasons(Reason::MissingSession.mask() | Reason::LosingDayStreak.mask()),
            vec![
                Reason::LosingDayStreak.as_str(),
                Reason::MissingSession.as_str()
            ]
        );
    }
}
