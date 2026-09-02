//! Fixed-layout institutional admission, separate from ranking.
//!
//! Ranking answers which admitted candidates are strongest relative to one
//! another.  Admission answers whether one fully measured candidate is allowed
//! into that population at all.  Keeping those operations separate prevents a
//! spectacular but incomplete row from winning merely because the ranker can
//! order the measurements it happened to receive.
//!
//! # No invented defaults
//!
//! [`AdmissionPolicyV1::new`] accepts an [`AdmissionPolicyDraftV1`] whose every
//! threshold is optional.  Every field is required and invalid domains are
//! refused with the exact field and cause.  There is deliberately no `Default`
//! implementation on the draft, policy, evidence or verdict.
//!
//! # Every check runs
//!
//! [`AdmissionPolicyV1::evaluate`] performs the same fixed V1 check set on
//! every call.  It does not stop at the first failure.  Measured failures,
//! unmeasured inputs and upstream refusals are kept in three disjoint
//! [`ReasonBits`] values, while their union is the authoritative reason set.
//! A candidate is admitted if and only if that union is empty.
//!
//! # Cost, representation, and the persistence boundary
//!
//! All values are integers, all layouts are fixed, and evaluation allocates
//! nothing.  Its time and space are O(1) because policy V1 has a fixed set of
//! append-only checks.  Adding a future check requires a new stable reason bit
//! and a new policy version; an existing bit is never renumbered or reused.
//!
//! Ppm fields are deterministic comparison projections only.  They do not
//! replace the full-precision statistical values, return/risk ratios, exact
//! counts and denominators that a durable population record must store under
//! repository law.
//! [`AdmissionEvidenceValuesV1::full_precision_statistics_complete`] makes that
//! source-record requirement an admission check rather than a comment.
//!
//! Policy, evidence, verdict and their recomputable union each have an explicit
//! V1 canonical byte representation and a domain-separated BLAKE3 digest.
//! Those encodings are independent of Rust layout, endianness and enum payload
//! layout: integers are little-endian, variants have explicit one-byte tags,
//! and payload-less states carry an all-zero payload.  `repr(C)` and `size_of`
//! remain in-memory bounds only and must never be persisted as a wire format.
//!
//! PBO median placement remains part of the required full source record and
//! report, but V1 does not gate on it: no operator-supplied median threshold was
//! provided, and inventing one here would violate the no-default rule.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

/// Comparison-projection denominator for bounded rates and probabilities.
///
/// This denominator is not the durable representation of a statistical value.
/// Full raw values and their exact denominators remain required upstream.
pub const PPM: u64 = 1_000_000;

/// Stable version carried by every canonical admission record and digest input.
pub const ADMISSION_VERSION_V1: u16 = 1;

/// Exact byte count of one canonical V1 policy record.
pub const ADMISSION_POLICY_CANONICAL_LEN_V1: usize = 310;
/// Exact byte count of one canonical V1 evidence record.
pub const ADMISSION_EVIDENCE_CANONICAL_LEN_V1: usize = 352;
/// Exact byte count of one canonical V1 verdict record.
pub const ADMISSION_VERDICT_CANONICAL_LEN_V1: usize = 45;
/// Exact byte count of one canonical V1 policy/evidence/verdict seal.
pub const ADMISSION_DECISION_CANONICAL_LEN_V1: usize = 719;

/// Successor version that joins exact Statistics V2 fields to an independent
/// anchored walk-forward authority without reinterpreting any V1 byte.
pub const ADMISSION_VERSION_V2: u16 = 2;
/// Exact byte count of one canonical Admission Evidence V2 record.
pub const ADMISSION_EVIDENCE_CANONICAL_LEN_V2: usize = 1_088;
/// Exact byte count of one canonical Admission Decision V2 record.
pub const ADMISSION_DECISION_CANONICAL_LEN_V2: usize = 1_455;

/// Successor version that removes identities detached Statistics cannot prove.
///
/// V3 retains the exact Candidate/Statistics identities and measurements plus
/// Runner's independently derived anchored walk authority.  It does not carry
/// Population Search, ranking, source-policy, or Finalization identities;
/// those terms belong to later typed CLI joins rather than Runner arithmetic.
pub const ADMISSION_VERSION_V3: u16 = 3;
/// Exact byte count of one canonical Admission Evidence V3 record.
pub const ADMISSION_EVIDENCE_CANONICAL_LEN_V3: usize = 960;
/// Exact byte count of one canonical Admission Decision V3 record.
pub const ADMISSION_DECISION_CANONICAL_LEN_V3: usize = 1_327;

const CANONICAL_HEADER_LEN: usize = 12;
const POLICY_PAYLOAD_LEN_V1: u32 = 298;
const EVIDENCE_PAYLOAD_LEN_V1: u32 = 340;
const VERDICT_PAYLOAD_LEN_V1: u32 = 33;
const DECISION_PAYLOAD_LEN_V1: u32 = 707;
const EVIDENCE_PAYLOAD_LEN_V2: u32 = 1_076;
const DECISION_PAYLOAD_LEN_V2: u32 = 1_443;
const EVIDENCE_PAYLOAD_LEN_V3: u32 = 948;
const DECISION_PAYLOAD_LEN_V3: u32 = 1_315;

const POLICY_DOMAIN_V1: u8 = 1;
const EVIDENCE_DOMAIN_V1: u8 = 2;
const VERDICT_DOMAIN_V1: u8 = 3;
const DECISION_DOMAIN_V1: u8 = 4;

/// Canonical admission record named by a decode refusal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionCanonicalRecordV1 {
    /// Validated policy record.
    Policy = POLICY_DOMAIN_V1,
    /// Validated evidence record.
    Evidence = EVIDENCE_DOMAIN_V1,
    /// Reconciled verdict record.
    Verdict = VERDICT_DOMAIN_V1,
    /// Policy/evidence/verdict decision seal.
    Decision = DECISION_DOMAIN_V1,
}

/// Verdict reason partition named by a canonical decode refusal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionVerdictPartitionV1 {
    /// Union of every reason partition.
    Reasons = 0,
    /// Measured policy failures.
    Failed = 1,
    /// Inputs that were not measured.
    Unmeasured = 2,
    /// Inputs refused by an upstream authority.
    Refused = 3,
}

/// Exact reason canonical admission bytes were refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionCanonicalRefusalV1 {
    /// The complete record length was not its fixed V1 length.
    Length {
        /// Record whose length was wrong.
        record: AdmissionCanonicalRecordV1,
        /// Fixed byte length required by V1.
        expected: usize,
        /// Byte length actually supplied.
        actual: usize,
    },
    /// The record did not start with the admission magic bytes.
    Magic {
        /// Record whose magic was wrong.
        record: AdmissionCanonicalRecordV1,
    },
    /// The domain byte did not name the record being decoded.
    Domain {
        /// Record decoder that received the bytes.
        record: AdmissionCanonicalRecordV1,
        /// Domain byte actually supplied.
        actual: u8,
    },
    /// A byte reserved as zero was nonzero.
    Reserved {
        /// Record containing the nonzero reserved byte or payload.
        record: AdmissionCanonicalRecordV1,
        /// Absolute byte offset of the first refused byte.
        offset: usize,
    },
    /// The canonical record version was not V1.
    Version {
        /// Record whose version was wrong.
        record: AdmissionCanonicalRecordV1,
        /// Version actually supplied.
        actual: u16,
    },
    /// The header's declared payload length was not the fixed V1 length.
    PayloadLength {
        /// Record whose payload declaration was wrong.
        record: AdmissionCanonicalRecordV1,
        /// Payload length actually declared.
        actual: u32,
    },
    /// A boolean or enum tag was outside its fixed V1 domain.
    Tag {
        /// Record containing the unknown tag.
        record: AdmissionCanonicalRecordV1,
        /// Absolute byte offset of the tag.
        offset: usize,
        /// Tag byte actually supplied.
        actual: u8,
    },
    /// Decoded policy values violated the authoritative policy constructor.
    Policy(AdmissionPolicyRefusalV1),
    /// Decoded evidence values violated the authoritative evidence constructor.
    Evidence(AdmissionEvidenceRefusalV1),
    /// A verdict partition contained a bit not assigned by V1.
    UnknownReasonBits {
        /// Partition containing the future or corrupt bit.
        partition: AdmissionVerdictPartitionV1,
    },
    /// Verdict partitions, their union, or their status contradicted one another.
    VerdictInconsistent,
    /// A well-formed supplied verdict was not the verdict recomputed from policy and evidence.
    VerdictMismatch,
}

/// A runtime policy field, used by exact construction refusals.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionFieldV1 {
    /// Minimum condition-mask support hits.
    MinSupportHits = 0,
    /// Minimum distinct, causally independent sessions carrying support.
    MinIndependentSessions = 1,
    /// Minimum fully closed trades.
    MinTrades = 2,
    /// Maximum adverse excursion in paisa.
    MaxMaePaisa = 3,
    /// Minimum pessimistic reward-to-risk ratio in ppm.
    MinWorstRewardRiskPpm = 4,
    /// Minimum pessimistic win rate in ppm.
    MinWinRatePpm = 5,
    /// Minimum Wilson lower win-rate bound in ppm.
    MinWilsonWinRatePpm = 6,
    /// Minimum pessimistic return-to-drawdown ratio in ppm.
    MinReturnDrawdownPpm = 7,
    /// Minimum return of the weakest required period, in paisa.
    MinWeakestPeriodReturnPaisa = 8,
    /// Maximum probability of backtest overfitting in ppm.
    MaxPboPpm = 9,
    /// Maximum family-wise adjusted p-value in ppm.
    MaxFwerPValuePpm = 10,
    /// Maximum Superior Predictive Ability p-value in ppm.
    MaxSpaPValuePpm = 11,
    /// Minimum walk-forward folds that made a decision.
    MinDecidedFolds = 12,
    /// Maximum ambiguous-fill rate in ppm.
    MaxAmbiguousFillRatePpm = 13,
    /// Maximum gap-affected trade rate in ppm.
    MaxGapAffectedRatePpm = 14,
    /// Maximum share of trades concentrated in one session in ppm.
    MaxSessionConcentrationPpm = 15,
    /// Maximum share of profit contributed by one trade in ppm.
    MaxLargestTradeProfitSharePpm = 16,
    /// Maximum peak-to-trough drawdown magnitude in paisa.
    MaxDrawdownPaisa = 17,
    /// Maximum worst-trade loss magnitude in paisa.
    MaxWorstTradeLossPaisa = 18,
    /// Maximum losing-trade rate in ppm.
    MaxLosingTradeRatePpm = 19,
    /// Maximum number of losing trades.
    MaxLosingTrades = 20,
    /// Minimum pessimistic total profit in paisa.
    MinPessimisticProfitPaisa = 21,
    /// Minimum number of winning trades.
    MinWinningTrades = 22,
    /// Minimum average winning-trade magnitude in paisa.
    MinAverageWinPaisa = 23,
    /// Maximum average losing-trade magnitude in paisa.
    MaxAverageLossPaisa = 24,
    /// Minimum gross-profit to gross-loss ratio in ppm.
    MinProfitFactorPpm = 25,
    /// Maximum consecutive losing-trade streak.
    MaxConsecutiveLosingStreak = 26,
    /// Minimum consecutive winning-trade streak.
    MinConsecutiveWinningStreak = 27,
    /// Minimum bootstrap draws used by every required family test.
    MinBootstrapDraws = 28,
    /// Minimum strategies compared by every required family test.
    MinBootstrapStrategies = 29,
    /// Minimum aligned periods used by every required family test.
    MinBootstrapPeriods = 30,
    /// Minimum PBO folds that contributed a valid placement.
    MinPboContributingFolds = 31,
    /// Maximum PBO folds that were unrankable.
    MaxPboUnrankableFolds = 32,
    /// Minimum profitable out-of-sample folds.
    MinProfitableOosFolds = 33,
    /// Minimum aggregate pessimistic out-of-sample return in paisa.
    MinOosPessimisticReturnPaisa = 34,
    /// Maximum White Reality Check p-value comparison projection in ppm.
    MaxWhiteRealityPValuePpm = 35,
    /// Whether White's null must be explicitly rejected.
    RequireWhiteRealityRejection = 36,
    /// Maximum candidate-specific Romano–Wolf p-value projection in ppm.
    MaxRomanoWolfPValuePpm = 37,
    /// Whether Romano–Wolf must explicitly reject the candidate's null.
    RequireRomanoWolfRejection = 38,
}

impl AdmissionFieldV1 {
    /// All V1 policy fields in stable numeric order.
    pub const ALL: [Self; 39] = [
        Self::MinSupportHits,
        Self::MinIndependentSessions,
        Self::MinTrades,
        Self::MaxMaePaisa,
        Self::MinWorstRewardRiskPpm,
        Self::MinWinRatePpm,
        Self::MinWilsonWinRatePpm,
        Self::MinReturnDrawdownPpm,
        Self::MinWeakestPeriodReturnPaisa,
        Self::MaxPboPpm,
        Self::MaxFwerPValuePpm,
        Self::MaxSpaPValuePpm,
        Self::MinDecidedFolds,
        Self::MaxAmbiguousFillRatePpm,
        Self::MaxGapAffectedRatePpm,
        Self::MaxSessionConcentrationPpm,
        Self::MaxLargestTradeProfitSharePpm,
        Self::MaxDrawdownPaisa,
        Self::MaxWorstTradeLossPaisa,
        Self::MaxLosingTradeRatePpm,
        Self::MaxLosingTrades,
        Self::MinPessimisticProfitPaisa,
        Self::MinWinningTrades,
        Self::MinAverageWinPaisa,
        Self::MaxAverageLossPaisa,
        Self::MinProfitFactorPpm,
        Self::MaxConsecutiveLosingStreak,
        Self::MinConsecutiveWinningStreak,
        Self::MinBootstrapDraws,
        Self::MinBootstrapStrategies,
        Self::MinBootstrapPeriods,
        Self::MinPboContributingFolds,
        Self::MaxPboUnrankableFolds,
        Self::MinProfitableOosFolds,
        Self::MinOosPessimisticReturnPaisa,
        Self::MaxWhiteRealityPValuePpm,
        Self::RequireWhiteRealityRejection,
        Self::MaxRomanoWolfPValuePpm,
        Self::RequireRomanoWolfRejection,
    ];

    /// Stable machine-readable name for audit and dashboard decoding.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MinSupportHits => "min_support_hits",
            Self::MinIndependentSessions => "min_independent_sessions",
            Self::MinTrades => "min_trades",
            Self::MaxMaePaisa => "max_mae_paisa",
            Self::MinWorstRewardRiskPpm => "min_worst_reward_risk_ppm",
            Self::MinWinRatePpm => "min_win_rate_ppm",
            Self::MinWilsonWinRatePpm => "min_wilson_win_rate_ppm",
            Self::MinReturnDrawdownPpm => "min_return_drawdown_ppm",
            Self::MinWeakestPeriodReturnPaisa => "min_weakest_period_return_paisa",
            Self::MaxPboPpm => "max_pbo_ppm",
            Self::MaxFwerPValuePpm => "max_fwer_p_value_ppm",
            Self::MaxSpaPValuePpm => "max_spa_p_value_ppm",
            Self::MinDecidedFolds => "min_decided_folds",
            Self::MaxAmbiguousFillRatePpm => "max_ambiguous_fill_rate_ppm",
            Self::MaxGapAffectedRatePpm => "max_gap_affected_rate_ppm",
            Self::MaxSessionConcentrationPpm => "max_session_concentration_ppm",
            Self::MaxLargestTradeProfitSharePpm => "max_largest_trade_profit_share_ppm",
            Self::MaxDrawdownPaisa => "max_drawdown_paisa",
            Self::MaxWorstTradeLossPaisa => "max_worst_trade_loss_paisa",
            Self::MaxLosingTradeRatePpm => "max_losing_trade_rate_ppm",
            Self::MaxLosingTrades => "max_losing_trades",
            Self::MinPessimisticProfitPaisa => "min_pessimistic_profit_paisa",
            Self::MinWinningTrades => "min_winning_trades",
            Self::MinAverageWinPaisa => "min_average_win_paisa",
            Self::MaxAverageLossPaisa => "max_average_loss_paisa",
            Self::MinProfitFactorPpm => "min_profit_factor_ppm",
            Self::MaxConsecutiveLosingStreak => "max_consecutive_losing_streak",
            Self::MinConsecutiveWinningStreak => "min_consecutive_winning_streak",
            Self::MinBootstrapDraws => "min_bootstrap_draws",
            Self::MinBootstrapStrategies => "min_bootstrap_strategies",
            Self::MinBootstrapPeriods => "min_bootstrap_periods",
            Self::MinPboContributingFolds => "min_pbo_contributing_folds",
            Self::MaxPboUnrankableFolds => "max_pbo_unrankable_folds",
            Self::MinProfitableOosFolds => "min_profitable_oos_folds",
            Self::MinOosPessimisticReturnPaisa => "min_oos_pessimistic_return_paisa",
            Self::MaxWhiteRealityPValuePpm => "max_white_reality_p_value_ppm",
            Self::RequireWhiteRealityRejection => "require_white_reality_rejection",
            Self::MaxRomanoWolfPValuePpm => "max_romano_wolf_p_value_ppm",
            Self::RequireRomanoWolfRejection => "require_romano_wolf_rejection",
        }
    }
}

/// Why a policy draft could not become an authoritative policy.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionPolicyRefusalKindV1 {
    /// A required runtime threshold was absent.
    Absent = 0,
    /// A structural sample-count floor was zero and proved no sample at all.
    MustBePositive = 1,
    /// A rate or probability exceeded the complete ppm domain.
    AboveOneMillion = 2,
}

/// Exact construction refusal for a policy draft.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionPolicyRefusalV1 {
    /// Field that was refused.
    pub field: AdmissionFieldV1,
    /// Domain rule that field violated.
    pub kind: AdmissionPolicyRefusalKindV1,
}

/// Every runtime choice needed to construct policy V1.
///
/// `Option` is intentional: omission must remain distinguishable from a
/// numeric zero.  Callers must fill every field; no fallback is supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionPolicyDraftV1 {
    /// See [`AdmissionFieldV1::MinSupportHits`].
    pub min_support_hits: Option<u64>,
    /// See [`AdmissionFieldV1::MinIndependentSessions`].
    pub min_independent_sessions: Option<u64>,
    /// See [`AdmissionFieldV1::MinTrades`].
    pub min_trades: Option<u64>,
    /// See [`AdmissionFieldV1::MaxMaePaisa`].
    pub max_mae_paisa: Option<u64>,
    /// See [`AdmissionFieldV1::MinWorstRewardRiskPpm`].
    pub min_worst_reward_risk_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MinWinRatePpm`].
    pub min_win_rate_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MinWilsonWinRatePpm`].
    pub min_wilson_win_rate_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MinReturnDrawdownPpm`].
    pub min_return_drawdown_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MinWeakestPeriodReturnPaisa`].
    pub min_weakest_period_return_paisa: Option<i64>,
    /// See [`AdmissionFieldV1::MaxPboPpm`].
    pub max_pbo_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxFwerPValuePpm`].
    pub max_fwer_p_value_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxSpaPValuePpm`].
    pub max_spa_p_value_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MinDecidedFolds`].
    pub min_decided_folds: Option<u64>,
    /// See [`AdmissionFieldV1::MaxAmbiguousFillRatePpm`].
    pub max_ambiguous_fill_rate_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxGapAffectedRatePpm`].
    pub max_gap_affected_rate_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxSessionConcentrationPpm`].
    pub max_session_concentration_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxLargestTradeProfitSharePpm`].
    pub max_largest_trade_profit_share_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxDrawdownPaisa`].
    pub max_drawdown_paisa: Option<u64>,
    /// See [`AdmissionFieldV1::MaxWorstTradeLossPaisa`].
    pub max_worst_trade_loss_paisa: Option<u64>,
    /// See [`AdmissionFieldV1::MaxLosingTradeRatePpm`].
    pub max_losing_trade_rate_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxLosingTrades`].
    pub max_losing_trades: Option<u64>,
    /// See [`AdmissionFieldV1::MinPessimisticProfitPaisa`].
    pub min_pessimistic_profit_paisa: Option<i64>,
    /// See [`AdmissionFieldV1::MinWinningTrades`].
    pub min_winning_trades: Option<u64>,
    /// See [`AdmissionFieldV1::MinAverageWinPaisa`].
    pub min_average_win_paisa: Option<u64>,
    /// See [`AdmissionFieldV1::MaxAverageLossPaisa`].
    pub max_average_loss_paisa: Option<u64>,
    /// See [`AdmissionFieldV1::MinProfitFactorPpm`].
    pub min_profit_factor_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::MaxConsecutiveLosingStreak`].
    pub max_consecutive_losing_streak: Option<u64>,
    /// See [`AdmissionFieldV1::MinConsecutiveWinningStreak`].
    pub min_consecutive_winning_streak: Option<u64>,
    /// See [`AdmissionFieldV1::MinBootstrapDraws`].
    pub min_bootstrap_draws: Option<u64>,
    /// See [`AdmissionFieldV1::MinBootstrapStrategies`].
    pub min_bootstrap_strategies: Option<u64>,
    /// See [`AdmissionFieldV1::MinBootstrapPeriods`].
    pub min_bootstrap_periods: Option<u64>,
    /// See [`AdmissionFieldV1::MinPboContributingFolds`].
    pub min_pbo_contributing_folds: Option<u64>,
    /// See [`AdmissionFieldV1::MaxPboUnrankableFolds`].
    pub max_pbo_unrankable_folds: Option<u64>,
    /// See [`AdmissionFieldV1::MinProfitableOosFolds`].
    pub min_profitable_oos_folds: Option<u64>,
    /// See [`AdmissionFieldV1::MinOosPessimisticReturnPaisa`].
    pub min_oos_pessimistic_return_paisa: Option<i64>,
    /// See [`AdmissionFieldV1::MaxWhiteRealityPValuePpm`].
    pub max_white_reality_p_value_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::RequireWhiteRealityRejection`].
    pub require_white_reality_rejection: Option<bool>,
    /// See [`AdmissionFieldV1::MaxRomanoWolfPValuePpm`].
    pub max_romano_wolf_p_value_ppm: Option<u64>,
    /// See [`AdmissionFieldV1::RequireRomanoWolfRejection`].
    pub require_romano_wolf_rejection: Option<bool>,
}

/// Validated in-memory comparison policy values.
///
/// This value is not a wire record; see the module persistence boundary.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionPolicyValuesV1 {
    /// Minimum support hits.
    pub min_support_hits: u64,
    /// Minimum independent sessions.
    pub min_independent_sessions: u64,
    /// Minimum closed trades.
    pub min_trades: u64,
    /// Maximum MAE in paisa.
    pub max_mae_paisa: u64,
    /// Minimum pessimistic reward-to-risk ratio in ppm.
    pub min_worst_reward_risk_ppm: u64,
    /// Minimum pessimistic win rate in ppm.
    pub min_win_rate_ppm: u64,
    /// Minimum Wilson lower bound in ppm.
    pub min_wilson_win_rate_ppm: u64,
    /// Minimum pessimistic return-to-drawdown ratio in ppm.
    pub min_return_drawdown_ppm: u64,
    /// Minimum weakest-period return in paisa.
    pub min_weakest_period_return_paisa: i64,
    /// Maximum PBO in ppm.
    pub max_pbo_ppm: u64,
    /// Maximum family-wise adjusted p-value in ppm.
    pub max_fwer_p_value_ppm: u64,
    /// Maximum SPA p-value in ppm.
    pub max_spa_p_value_ppm: u64,
    /// Minimum decided folds.
    pub min_decided_folds: u64,
    /// Maximum ambiguous-fill rate in ppm.
    pub max_ambiguous_fill_rate_ppm: u64,
    /// Maximum gap-affected rate in ppm.
    pub max_gap_affected_rate_ppm: u64,
    /// Maximum single-session trade concentration in ppm.
    pub max_session_concentration_ppm: u64,
    /// Maximum single-trade profit share in ppm.
    pub max_largest_trade_profit_share_ppm: u64,
    /// Maximum peak-to-trough drawdown magnitude in paisa.
    pub max_drawdown_paisa: u64,
    /// Maximum worst-trade loss magnitude in paisa.
    pub max_worst_trade_loss_paisa: u64,
    /// Maximum losing-trade rate in ppm.
    pub max_losing_trade_rate_ppm: u64,
    /// Maximum losing-trade count.
    pub max_losing_trades: u64,
    /// Minimum pessimistic profit in paisa.
    pub min_pessimistic_profit_paisa: i64,
    /// Minimum winning-trade count.
    pub min_winning_trades: u64,
    /// Minimum average winning-trade magnitude in paisa.
    pub min_average_win_paisa: u64,
    /// Maximum average losing-trade magnitude in paisa.
    pub max_average_loss_paisa: u64,
    /// Minimum gross-profit to gross-loss ratio in ppm.
    pub min_profit_factor_ppm: u64,
    /// Maximum consecutive losing-trade streak.
    pub max_consecutive_losing_streak: u64,
    /// Minimum consecutive winning-trade streak.
    pub min_consecutive_winning_streak: u64,
    /// Minimum bootstrap draws.
    pub min_bootstrap_draws: u64,
    /// Minimum strategies compared by each family test.
    pub min_bootstrap_strategies: u64,
    /// Minimum aligned bootstrap periods.
    pub min_bootstrap_periods: u64,
    /// Minimum PBO folds contributing valid placements.
    pub min_pbo_contributing_folds: u64,
    /// Maximum unrankable PBO folds.
    pub max_pbo_unrankable_folds: u64,
    /// Minimum profitable out-of-sample folds.
    pub min_profitable_oos_folds: u64,
    /// Minimum aggregate pessimistic out-of-sample return in paisa.
    pub min_oos_pessimistic_return_paisa: i64,
    /// Maximum White Reality Check p-value projection in ppm.
    pub max_white_reality_p_value_ppm: u64,
    /// Maximum candidate-specific Romano–Wolf p-value projection in ppm.
    pub max_romano_wolf_p_value_ppm: u64,
    /// Whether White's null must be explicitly rejected.
    pub require_white_reality_rejection: bool,
    /// Whether Romano–Wolf must explicitly reject the candidate's null.
    pub require_romano_wolf_rejection: bool,
}

/// Validated, immutable admission policy V1.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionPolicyV1 {
    values: AdmissionPolicyValuesV1,
}

impl AdmissionPolicyV1 {
    /// Validates every required runtime threshold without supplying defaults.
    ///
    /// # Errors
    ///
    /// Returns the first field in stable policy order that is absent or outside
    /// its declared domain.  Count floors must be positive.  Rates and
    /// probabilities must be at most [`PPM`].  Ratios may exceed one because a
    /// reward-to-risk or return-to-drawdown ratio is not a percentage.
    #[expect(
        clippy::too_many_lines,
        reason = "the explicit no-default V1 constructor keeps every public draft field beside its domain validator and validated comparison value"
    )]
    pub fn new(draft: AdmissionPolicyDraftV1) -> Result<Self, AdmissionPolicyRefusalV1> {
        let min_support_hits =
            required_positive(draft.min_support_hits, AdmissionFieldV1::MinSupportHits)?;
        let min_independent_sessions = required_positive(
            draft.min_independent_sessions,
            AdmissionFieldV1::MinIndependentSessions,
        )?;
        let min_trades = required_positive(draft.min_trades, AdmissionFieldV1::MinTrades)?;
        let max_mae_paisa = required(draft.max_mae_paisa, AdmissionFieldV1::MaxMaePaisa)?;
        let min_worst_reward_risk_ppm = required(
            draft.min_worst_reward_risk_ppm,
            AdmissionFieldV1::MinWorstRewardRiskPpm,
        )?;
        let min_win_rate_ppm =
            required_ppm(draft.min_win_rate_ppm, AdmissionFieldV1::MinWinRatePpm)?;
        let min_wilson_win_rate_ppm = required_ppm(
            draft.min_wilson_win_rate_ppm,
            AdmissionFieldV1::MinWilsonWinRatePpm,
        )?;
        let min_return_drawdown_ppm = required(
            draft.min_return_drawdown_ppm,
            AdmissionFieldV1::MinReturnDrawdownPpm,
        )?;
        let min_weakest_period_return_paisa =
            draft
                .min_weakest_period_return_paisa
                .ok_or(AdmissionPolicyRefusalV1 {
                    field: AdmissionFieldV1::MinWeakestPeriodReturnPaisa,
                    kind: AdmissionPolicyRefusalKindV1::Absent,
                })?;
        let max_pbo_ppm = required_ppm(draft.max_pbo_ppm, AdmissionFieldV1::MaxPboPpm)?;
        let max_fwer_p_value_ppm = required_ppm(
            draft.max_fwer_p_value_ppm,
            AdmissionFieldV1::MaxFwerPValuePpm,
        )?;
        let max_spa_p_value_ppm =
            required_ppm(draft.max_spa_p_value_ppm, AdmissionFieldV1::MaxSpaPValuePpm)?;
        let min_decided_folds =
            required_positive(draft.min_decided_folds, AdmissionFieldV1::MinDecidedFolds)?;
        let max_ambiguous_fill_rate_ppm = required_ppm(
            draft.max_ambiguous_fill_rate_ppm,
            AdmissionFieldV1::MaxAmbiguousFillRatePpm,
        )?;
        let max_gap_affected_rate_ppm = required_ppm(
            draft.max_gap_affected_rate_ppm,
            AdmissionFieldV1::MaxGapAffectedRatePpm,
        )?;
        let max_session_concentration_ppm = required_ppm(
            draft.max_session_concentration_ppm,
            AdmissionFieldV1::MaxSessionConcentrationPpm,
        )?;
        let max_largest_trade_profit_share_ppm = required_ppm(
            draft.max_largest_trade_profit_share_ppm,
            AdmissionFieldV1::MaxLargestTradeProfitSharePpm,
        )?;
        let max_drawdown_paisa =
            required(draft.max_drawdown_paisa, AdmissionFieldV1::MaxDrawdownPaisa)?;
        let max_worst_trade_loss_paisa = required(
            draft.max_worst_trade_loss_paisa,
            AdmissionFieldV1::MaxWorstTradeLossPaisa,
        )?;
        let max_losing_trade_rate_ppm = required_ppm(
            draft.max_losing_trade_rate_ppm,
            AdmissionFieldV1::MaxLosingTradeRatePpm,
        )?;
        let max_losing_trades =
            required(draft.max_losing_trades, AdmissionFieldV1::MaxLosingTrades)?;
        let min_pessimistic_profit_paisa =
            draft
                .min_pessimistic_profit_paisa
                .ok_or(AdmissionPolicyRefusalV1 {
                    field: AdmissionFieldV1::MinPessimisticProfitPaisa,
                    kind: AdmissionPolicyRefusalKindV1::Absent,
                })?;
        let min_winning_trades =
            required(draft.min_winning_trades, AdmissionFieldV1::MinWinningTrades)?;
        let min_average_win_paisa = required(
            draft.min_average_win_paisa,
            AdmissionFieldV1::MinAverageWinPaisa,
        )?;
        let max_average_loss_paisa = required(
            draft.max_average_loss_paisa,
            AdmissionFieldV1::MaxAverageLossPaisa,
        )?;
        let min_profit_factor_ppm = required(
            draft.min_profit_factor_ppm,
            AdmissionFieldV1::MinProfitFactorPpm,
        )?;
        let max_consecutive_losing_streak = required(
            draft.max_consecutive_losing_streak,
            AdmissionFieldV1::MaxConsecutiveLosingStreak,
        )?;
        let min_consecutive_winning_streak = required(
            draft.min_consecutive_winning_streak,
            AdmissionFieldV1::MinConsecutiveWinningStreak,
        )?;
        let min_bootstrap_draws = required_positive(
            draft.min_bootstrap_draws,
            AdmissionFieldV1::MinBootstrapDraws,
        )?;
        let min_bootstrap_strategies = required_positive(
            draft.min_bootstrap_strategies,
            AdmissionFieldV1::MinBootstrapStrategies,
        )?;
        let min_bootstrap_periods = required_positive(
            draft.min_bootstrap_periods,
            AdmissionFieldV1::MinBootstrapPeriods,
        )?;
        let min_pbo_contributing_folds = required_positive(
            draft.min_pbo_contributing_folds,
            AdmissionFieldV1::MinPboContributingFolds,
        )?;
        let max_pbo_unrankable_folds = required(
            draft.max_pbo_unrankable_folds,
            AdmissionFieldV1::MaxPboUnrankableFolds,
        )?;
        let min_profitable_oos_folds = required_positive(
            draft.min_profitable_oos_folds,
            AdmissionFieldV1::MinProfitableOosFolds,
        )?;
        let min_oos_pessimistic_return_paisa =
            draft
                .min_oos_pessimistic_return_paisa
                .ok_or(AdmissionPolicyRefusalV1 {
                    field: AdmissionFieldV1::MinOosPessimisticReturnPaisa,
                    kind: AdmissionPolicyRefusalKindV1::Absent,
                })?;
        let max_white_reality_p_value_ppm = required_ppm(
            draft.max_white_reality_p_value_ppm,
            AdmissionFieldV1::MaxWhiteRealityPValuePpm,
        )?;
        let require_white_reality_rejection = required_bool(
            draft.require_white_reality_rejection,
            AdmissionFieldV1::RequireWhiteRealityRejection,
        )?;
        let max_romano_wolf_p_value_ppm = required_ppm(
            draft.max_romano_wolf_p_value_ppm,
            AdmissionFieldV1::MaxRomanoWolfPValuePpm,
        )?;
        let require_romano_wolf_rejection = required_bool(
            draft.require_romano_wolf_rejection,
            AdmissionFieldV1::RequireRomanoWolfRejection,
        )?;

        Ok(Self {
            values: AdmissionPolicyValuesV1 {
                min_support_hits,
                min_independent_sessions,
                min_trades,
                max_mae_paisa,
                min_worst_reward_risk_ppm,
                min_win_rate_ppm,
                min_wilson_win_rate_ppm,
                min_return_drawdown_ppm,
                min_weakest_period_return_paisa,
                max_pbo_ppm,
                max_fwer_p_value_ppm,
                max_spa_p_value_ppm,
                min_decided_folds,
                max_ambiguous_fill_rate_ppm,
                max_gap_affected_rate_ppm,
                max_session_concentration_ppm,
                max_largest_trade_profit_share_ppm,
                max_drawdown_paisa,
                max_worst_trade_loss_paisa,
                max_losing_trade_rate_ppm,
                max_losing_trades,
                min_pessimistic_profit_paisa,
                min_winning_trades,
                min_average_win_paisa,
                max_average_loss_paisa,
                min_profit_factor_ppm,
                max_consecutive_losing_streak,
                min_consecutive_winning_streak,
                min_bootstrap_draws,
                min_bootstrap_strategies,
                min_bootstrap_periods,
                min_pbo_contributing_folds,
                max_pbo_unrankable_folds,
                min_profitable_oos_folds,
                min_oos_pessimistic_return_paisa,
                max_white_reality_p_value_ppm,
                max_romano_wolf_p_value_ppm,
                require_white_reality_rejection,
                require_romano_wolf_rejection,
            },
        })
    }

    /// Exact validated in-memory comparison values.
    #[must_use]
    pub const fn values(self) -> AdmissionPolicyValuesV1 {
        self.values
    }

    /// Canonical, fixed-size V1 policy bytes.
    ///
    /// The returned bytes contain their own domain, version and payload length;
    /// they do not expose or depend on Rust's in-memory representation.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; ADMISSION_POLICY_CANONICAL_LEN_V1] {
        canonical_policy_bytes(&self.values)
    }

    /// Decodes and revalidates one exact canonical V1 policy record.
    ///
    /// # Errors
    ///
    /// Refuses every wrong length, header field, reserved byte, boolean tag or
    /// value rejected by [`Self::new`].  No missing value is defaulted.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV1> {
        decode_policy(bytes)
    }

    /// Domain-separated BLAKE3 of [`Self::canonical_bytes`].
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }

    /// Evaluates and seals the exact policy, evidence and resulting verdict.
    #[must_use]
    pub fn evaluate_sealed(self, evidence: &AdmissionEvidenceV1) -> AdmissionDecisionSealV1 {
        AdmissionDecisionSealV1::new(&self, evidence)
    }

    /// Evaluates all V1 checks and records every applicable reason.
    ///
    /// The evidence type is an opaque validated boundary: its public
    /// constructor and canonical decoder both refuse measured values in the
    /// three PBO-named V1 slots. Consequently, a publicly reachable V1 policy
    /// evaluation can report those checks only as `Unmeasured` or `Refused`;
    /// it cannot authorize a measured legacy anchored-fold diagnostic as
    /// genuine CSCV/PBO. A genuine measured authority requires a successor
    /// evidence version.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "the fixed V1 audit table keeps every stable reason beside its exact evidence and threshold; splitting or iterating erased that reviewable mapping"
    )]
    pub fn evaluate(self, evidence: &AdmissionEvidenceV1) -> AdmissionVerdictV1 {
        let e = evidence.values;
        let p = self.values;
        let mut failed = ReasonBits::EMPTY;
        let mut unmeasured = ReasonBits::EMPTY;
        let mut refused = ReasonBits::EMPTY;

        check_min_u64(
            e.support_hits,
            p.min_support_hits,
            AdmissionReasonV1::Support,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.independent_sessions,
            p.min_independent_sessions,
            AdmissionReasonV1::SessionIndependence,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.trades,
            p.min_trades,
            AdmissionReasonV1::Trades,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.max_mae_paisa,
            p.max_mae_paisa,
            AdmissionReasonV1::MaxMae,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.worst_reward_risk_ppm,
            p.min_worst_reward_risk_ppm,
            AdmissionReasonV1::WorstRewardRisk,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.win_rate_ppm,
            p.min_win_rate_ppm,
            AdmissionReasonV1::WinRate,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.wilson_win_rate_ppm,
            p.min_wilson_win_rate_ppm,
            AdmissionReasonV1::WilsonWinRate,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.return_drawdown_ppm,
            p.min_return_drawdown_ppm,
            AdmissionReasonV1::ReturnDrawdown,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_i64(
            e.weakest_period_return_paisa,
            p.min_weakest_period_return_paisa,
            AdmissionReasonV1::WeakestPeriod,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.pbo_ppm,
            p.max_pbo_ppm,
            AdmissionReasonV1::Pbo,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.fwer_p_value_ppm,
            p.max_fwer_p_value_ppm,
            AdmissionReasonV1::Fwer,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.spa_p_value_ppm,
            p.max_spa_p_value_ppm,
            AdmissionReasonV1::Spa,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.decided_folds,
            p.min_decided_folds,
            AdmissionReasonV1::DecidedFolds,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.ambiguous_fill_rate_ppm,
            p.max_ambiguous_fill_rate_ppm,
            AdmissionReasonV1::AmbiguousFills,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.gap_affected_rate_ppm,
            p.max_gap_affected_rate_ppm,
            AdmissionReasonV1::GapAffected,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.session_concentration_ppm,
            p.max_session_concentration_ppm,
            AdmissionReasonV1::SessionConcentration,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.largest_trade_profit_share_ppm,
            p.max_largest_trade_profit_share_ppm,
            AdmissionReasonV1::LargestTradeProfitShare,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_completeness(
            e.execution_complete,
            AdmissionReasonV1::ExecutionCompleteness,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_completeness(
            e.data_complete,
            AdmissionReasonV1::DataCompleteness,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_completeness(
            e.calendar_complete,
            AdmissionReasonV1::CalendarCompleteness,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_completeness(
            e.population_complete,
            AdmissionReasonV1::PopulationCompleteness,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.drawdown_paisa,
            p.max_drawdown_paisa,
            AdmissionReasonV1::Drawdown,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.worst_trade_loss_paisa,
            p.max_worst_trade_loss_paisa,
            AdmissionReasonV1::WorstTradeLoss,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.losing_trade_rate_ppm,
            p.max_losing_trade_rate_ppm,
            AdmissionReasonV1::LosingTradeRate,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.losing_trades,
            p.max_losing_trades,
            AdmissionReasonV1::LosingTrades,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_i64(
            e.pessimistic_profit_paisa,
            p.min_pessimistic_profit_paisa,
            AdmissionReasonV1::PessimisticProfit,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.winning_trades,
            p.min_winning_trades,
            AdmissionReasonV1::WinningTrades,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.average_win_paisa,
            p.min_average_win_paisa,
            AdmissionReasonV1::AverageWin,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.average_loss_paisa,
            p.max_average_loss_paisa,
            AdmissionReasonV1::AverageLoss,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.profit_factor_ppm,
            p.min_profit_factor_ppm,
            AdmissionReasonV1::ProfitFactor,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.consecutive_losing_streak,
            p.max_consecutive_losing_streak,
            AdmissionReasonV1::ConsecutiveLosingStreak,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.consecutive_winning_streak,
            p.min_consecutive_winning_streak,
            AdmissionReasonV1::ConsecutiveWinningStreak,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.bootstrap_draws,
            p.min_bootstrap_draws,
            AdmissionReasonV1::BootstrapDraws,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.bootstrap_strategies,
            p.min_bootstrap_strategies,
            AdmissionReasonV1::BootstrapStrategies,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.bootstrap_periods,
            p.min_bootstrap_periods,
            AdmissionReasonV1::BootstrapPeriods,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.pbo_contributing_folds,
            p.min_pbo_contributing_folds,
            AdmissionReasonV1::PboContributingFolds,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.pbo_unrankable_folds,
            p.max_pbo_unrankable_folds,
            AdmissionReasonV1::PboUnrankableFolds,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_u64(
            e.profitable_oos_folds,
            p.min_profitable_oos_folds,
            AdmissionReasonV1::ProfitableOosFolds,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_min_i64(
            e.oos_pessimistic_return_paisa,
            p.min_oos_pessimistic_return_paisa,
            AdmissionReasonV1::OosPessimisticReturn,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.white_reality_p_value_ppm,
            p.max_white_reality_p_value_ppm,
            AdmissionReasonV1::WhiteRealityPValue,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_hypothesis_decision(
            e.white_reality_decision,
            p.require_white_reality_rejection,
            AdmissionReasonV1::WhiteRealityDecision,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_max_u64(
            e.romano_wolf_p_value_ppm,
            p.max_romano_wolf_p_value_ppm,
            AdmissionReasonV1::RomanoWolfPValue,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_hypothesis_decision(
            e.romano_wolf_decision,
            p.require_romano_wolf_rejection,
            AdmissionReasonV1::RomanoWolfDecision,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );
        check_completeness(
            e.full_precision_statistics_complete,
            AdmissionReasonV1::FullPrecisionStatisticsCompleteness,
            &mut failed,
            &mut unmeasured,
            &mut refused,
        );

        AdmissionVerdictV1::from_parts(failed, unmeasured, refused)
    }
}

fn required(value: Option<u64>, field: AdmissionFieldV1) -> Result<u64, AdmissionPolicyRefusalV1> {
    value.ok_or(AdmissionPolicyRefusalV1 {
        field,
        kind: AdmissionPolicyRefusalKindV1::Absent,
    })
}

fn required_bool(
    value: Option<bool>,
    field: AdmissionFieldV1,
) -> Result<bool, AdmissionPolicyRefusalV1> {
    value.ok_or(AdmissionPolicyRefusalV1 {
        field,
        kind: AdmissionPolicyRefusalKindV1::Absent,
    })
}

fn required_positive(
    value: Option<u64>,
    field: AdmissionFieldV1,
) -> Result<u64, AdmissionPolicyRefusalV1> {
    let value = required(value, field)?;
    if value == 0 {
        return Err(AdmissionPolicyRefusalV1 {
            field,
            kind: AdmissionPolicyRefusalKindV1::MustBePositive,
        });
    }
    Ok(value)
}

fn required_ppm(
    value: Option<u64>,
    field: AdmissionFieldV1,
) -> Result<u64, AdmissionPolicyRefusalV1> {
    let value = required(value, field)?;
    if value > PPM {
        return Err(AdmissionPolicyRefusalV1 {
            field,
            kind: AdmissionPolicyRefusalKindV1::AboveOneMillion,
        });
    }
    Ok(value)
}

/// One measured quantity, an explicit absence, or an upstream refusal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedU64V1 {
    /// Quantity was measured.
    Measured(u64),
    /// Quantity could not be measured from the admitted input shape.
    Unmeasured,
    /// Upstream validation refused the quantity as invalid.
    Refused,
}

/// Signed counterpart used by the weakest-period return.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedI64V1 {
    /// Quantity was measured.
    Measured(i64),
    /// Quantity could not be measured from the admitted input shape.
    Unmeasured,
    /// Upstream validation refused the quantity as invalid.
    Refused,
}

/// Completeness evidence for one required upstream surface.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletenessV1 {
    /// Surface reconciled completely.
    Complete = 0,
    /// Surface ran and proved itself incomplete.
    Incomplete = 1,
    /// Surface did not produce a completeness measurement.
    Unmeasured = 2,
    /// Surface explicitly refused its input or result.
    Refused = 3,
}

/// Explicit upstream hypothesis-test decision for a required family test.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HypothesisDecisionV1 {
    /// The test rejected its null for the required family or candidate.
    RejectedNull = 0,
    /// The test ran but did not reject its null.
    DidNotReject = 1,
    /// No decision could be measured.
    Unmeasured = 2,
    /// Upstream validation refused the test or its inputs.
    Refused = 3,
}

/// Fully typed evidence offered to [`AdmissionEvidenceV1::new`].
///
/// Rate, probability and ratio ppm values are comparison projections, not the
/// durable source statistics.  The future population record must store the
/// full raw values and exact denominators separately.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionEvidenceValuesV1 {
    /// Condition-mask support hits.
    pub support_hits: ObservedU64V1,
    /// Distinct causally independent sessions carrying support.
    pub independent_sessions: ObservedU64V1,
    /// Fully closed trades.
    pub trades: ObservedU64V1,
    /// Maximum adverse excursion in paisa.
    pub max_mae_paisa: ObservedU64V1,
    /// Pessimistic reward-to-risk ratio in ppm.
    pub worst_reward_risk_ppm: ObservedU64V1,
    /// Pessimistic win rate in ppm.
    pub win_rate_ppm: ObservedU64V1,
    /// Wilson lower win-rate bound in ppm.
    pub wilson_win_rate_ppm: ObservedU64V1,
    /// Pessimistic return-to-drawdown ratio in ppm.
    pub return_drawdown_ppm: ObservedU64V1,
    /// Weakest required period return in paisa.
    pub weakest_period_return_paisa: ObservedI64V1,
    /// Reserved V1 probability-of-backtest-overfitting projection in ppm.
    ///
    /// Genuine CSCV/PBO authority did not exist when this fixed layout shipped.
    /// [`AdmissionEvidenceV1::new`] therefore refuses a measured value in this
    /// slot; only `Unmeasured` or `Refused` is valid V1 evidence.
    pub pbo_ppm: ObservedU64V1,
    /// Family-wise adjusted p-value in ppm.
    pub fwer_p_value_ppm: ObservedU64V1,
    /// Superior Predictive Ability p-value in ppm.
    pub spa_p_value_ppm: ObservedU64V1,
    /// Walk-forward folds that made a decision.
    pub decided_folds: ObservedU64V1,
    /// Ambiguous-fill rate in ppm.
    pub ambiguous_fill_rate_ppm: ObservedU64V1,
    /// Gap-affected trade rate in ppm.
    pub gap_affected_rate_ppm: ObservedU64V1,
    /// Share of trades concentrated in one session in ppm.
    pub session_concentration_ppm: ObservedU64V1,
    /// Share of total profit supplied by the largest winning trade in ppm.
    pub largest_trade_profit_share_ppm: ObservedU64V1,
    /// Exact one-minute execution evidence completeness.
    pub execution_complete: CompletenessV1,
    /// Stored input data completeness and reconciliation.
    pub data_complete: CompletenessV1,
    /// IST trading-calendar completeness and reconciliation.
    pub calendar_complete: CompletenessV1,
    /// Full candidate-population enumeration and pricing completeness.
    pub population_complete: CompletenessV1,
    /// Peak-to-trough drawdown magnitude in paisa.
    pub drawdown_paisa: ObservedU64V1,
    /// Worst-trade loss magnitude in paisa.
    pub worst_trade_loss_paisa: ObservedU64V1,
    /// Losing-trade rate in ppm.
    pub losing_trade_rate_ppm: ObservedU64V1,
    /// Number of losing trades.
    pub losing_trades: ObservedU64V1,
    /// Pessimistic total profit in paisa.
    pub pessimistic_profit_paisa: ObservedI64V1,
    /// Number of winning trades.
    pub winning_trades: ObservedU64V1,
    /// Average winning-trade magnitude in paisa.
    pub average_win_paisa: ObservedU64V1,
    /// Average losing-trade magnitude in paisa.
    pub average_loss_paisa: ObservedU64V1,
    /// Gross-profit to gross-loss ratio in ppm.
    pub profit_factor_ppm: ObservedU64V1,
    /// Longest consecutive losing-trade streak.
    pub consecutive_losing_streak: ObservedU64V1,
    /// Longest consecutive winning-trade streak.
    pub consecutive_winning_streak: ObservedU64V1,
    /// Smallest bootstrap draw count across required White, SPA and RW tests.
    pub bootstrap_draws: ObservedU64V1,
    /// Smallest compared-strategy count across required family tests.
    pub bootstrap_strategies: ObservedU64V1,
    /// Smallest aligned-period count across required family tests.
    pub bootstrap_periods: ObservedU64V1,
    /// Reserved V1 count of genuine CSCV/PBO splits that contributed.
    ///
    /// A measured value is refused in V1 for the same reason as [`Self::pbo_ppm`].
    pub pbo_contributing_folds: ObservedU64V1,
    /// Reserved V1 count of genuine CSCV/PBO splits that could not be ranked.
    ///
    /// A measured value is refused in V1 for the same reason as [`Self::pbo_ppm`].
    pub pbo_unrankable_folds: ObservedU64V1,
    /// Out-of-sample folds with positive pessimistic return.
    pub profitable_oos_folds: ObservedU64V1,
    /// Aggregate pessimistic out-of-sample return in paisa.
    pub oos_pessimistic_return_paisa: ObservedI64V1,
    /// White Reality Check p-value comparison projection in ppm.
    pub white_reality_p_value_ppm: ObservedU64V1,
    /// Candidate-specific Romano–Wolf p-value comparison projection in ppm.
    pub romano_wolf_p_value_ppm: ObservedU64V1,
    /// White Reality Check's explicit family decision.
    pub white_reality_decision: HypothesisDecisionV1,
    /// Candidate-specific Romano–Wolf decision.
    pub romano_wolf_decision: HypothesisDecisionV1,
    /// Whether full-precision source statistics and denominators exist upstream.
    pub full_precision_statistics_complete: CompletenessV1,
}

/// Evidence field named by construction refusals.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionEvidenceFieldV1 {
    /// Winning-trade rate projection.
    WinRate = 0,
    /// Wilson lower-bound projection.
    WilsonWinRate = 1,
    /// PBO projection.
    Pbo = 2,
    /// Family-wise adjusted p-value projection.
    FwerPValue = 3,
    /// SPA p-value projection.
    SpaPValue = 4,
    /// Ambiguous-fill rate projection.
    AmbiguousFillRate = 5,
    /// Gap-affected rate projection.
    GapAffectedRate = 6,
    /// Session-concentration projection.
    SessionConcentration = 7,
    /// Largest-trade profit-share projection.
    LargestTradeProfitShare = 8,
    /// Losing-trade rate projection.
    LosingTradeRate = 9,
    /// The trades/wins/losses count tuple.
    TradeCounts = 10,
    /// White Reality Check p-value projection.
    WhiteRealityPValue = 11,
    /// Romano–Wolf candidate p-value projection.
    RomanoWolfPValue = 12,
    /// Independent supporting sessions relative to support hits.
    SupportSessions = 13,
    /// Consecutive winning streak relative to winning trades.
    WinningStreak = 14,
    /// Consecutive losing streak relative to losing trades.
    LosingStreak = 15,
    /// Profitable out-of-sample folds relative to decided folds.
    OosFolds = 16,
    /// Reserved genuine-CSCV/PBO contributing-split count.
    PboContributingFolds = 17,
    /// Reserved genuine-CSCV/PBO unrankable-split count.
    PboUnrankableFolds = 18,
}

/// Exact evidence-domain rule that construction refused.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionEvidenceRefusalKindV1 {
    /// A bounded comparison projection exceeded one million ppm.
    AboveOneMillion = 0,
    /// Adding winning and losing counts overflowed `u64`.
    TradeCountOverflow = 1,
    /// Winning plus losing trades did not equal total trades.
    TradeCountMismatch = 2,
    /// A measured rate had no measured count and total to support it.
    RateWithoutCounts = 3,
    /// A measured rate was supplied for a zero-trade denominator.
    RateWithZeroDenominator = 4,
    /// A measured rate did not equal the canonical count projection.
    RateCountMismatch = 5,
    /// A measured subset count exceeded its measured containing total.
    SubsetExceedsTotal = 6,
    /// V1 carried a measured PBO-named value without a genuine CSCV authority.
    UnsupportedMeasuredPboV1 = 7,
}

/// Why measured evidence was outside its physical or reconciliation domain.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionEvidenceRefusalV1 {
    /// Exact evidence field or tuple refused.
    pub field: AdmissionEvidenceFieldV1,
    /// Exact domain or reconciliation rule violated.
    pub kind: AdmissionEvidenceRefusalKindV1,
}

/// Validated V1 evidence for one candidate.
///
/// V1 reserved three PBO-named slots but carries no CSCV procedure identity,
/// complete split-family receipt or authority digest. Because the repository
/// never implemented genuine CSCV/PBO for this version, accepting any measured
/// value in those slots would let the legacy anchored-fold diagnostic authorize
/// a strategy after reopen. Construction and decoding therefore refuse all
/// three measured states. Their `Unmeasured` and `Refused` encodings remain
/// readable; a future genuine CSCV authority requires a successor evidence
/// version rather than reinterpretation of these bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionEvidenceV1 {
    values: AdmissionEvidenceValuesV1,
}

impl AdmissionEvidenceV1 {
    /// Refuses unsupported V1 PBO claims, invalid projections and contradictory
    /// trade counts/rates.
    ///
    /// Ratios are deliberately not bounded at one million: both reward/risk
    /// and return/drawdown can validly exceed one.  `Unmeasured` and `Refused`
    /// are accepted as explicit states and will remain visible in the verdict.
    ///
    /// # Errors
    ///
    /// Returns the exact evidence field and violated domain rule.  When total,
    /// winning and losing counts are all measured, winning plus losing must
    /// equal total exactly.  A measured count rate uses the canonical floor
    /// projection `count * 1_000_000 / trades`, computed in `u128`; a zero
    /// denominator has no measured rate and must use an explicit unmeasured or
    /// refused state. All three PBO-named V1 observations must likewise be
    /// `Unmeasured` or `Refused`; this version has no authority capable of
    /// producing a measured value.
    pub fn new(values: AdmissionEvidenceValuesV1) -> Result<Self, AdmissionEvidenceRefusalV1> {
        refuse_unsupported_measured_pbo_v1(&values)?;
        validate_observed_ppm(values.win_rate_ppm, AdmissionEvidenceFieldV1::WinRate)?;
        validate_observed_ppm(
            values.wilson_win_rate_ppm,
            AdmissionEvidenceFieldV1::WilsonWinRate,
        )?;
        validate_observed_ppm(values.pbo_ppm, AdmissionEvidenceFieldV1::Pbo)?;
        validate_observed_ppm(
            values.fwer_p_value_ppm,
            AdmissionEvidenceFieldV1::FwerPValue,
        )?;
        validate_observed_ppm(values.spa_p_value_ppm, AdmissionEvidenceFieldV1::SpaPValue)?;
        validate_observed_ppm(
            values.ambiguous_fill_rate_ppm,
            AdmissionEvidenceFieldV1::AmbiguousFillRate,
        )?;
        validate_observed_ppm(
            values.gap_affected_rate_ppm,
            AdmissionEvidenceFieldV1::GapAffectedRate,
        )?;
        validate_observed_ppm(
            values.session_concentration_ppm,
            AdmissionEvidenceFieldV1::SessionConcentration,
        )?;
        validate_observed_ppm(
            values.largest_trade_profit_share_ppm,
            AdmissionEvidenceFieldV1::LargestTradeProfitShare,
        )?;
        validate_observed_ppm(
            values.losing_trade_rate_ppm,
            AdmissionEvidenceFieldV1::LosingTradeRate,
        )?;
        validate_observed_ppm(
            values.white_reality_p_value_ppm,
            AdmissionEvidenceFieldV1::WhiteRealityPValue,
        )?;
        validate_observed_ppm(
            values.romano_wolf_p_value_ppm,
            AdmissionEvidenceFieldV1::RomanoWolfPValue,
        )?;
        reconcile_trade_counts_and_rates(&values)?;
        reconcile_measured_subsets(&values)?;
        Ok(Self { values })
    }

    /// Validated in-memory comparison evidence.
    ///
    /// This value is not a durable statistical record or wire encoding.
    #[must_use]
    pub const fn values(self) -> AdmissionEvidenceValuesV1 {
        self.values
    }

    /// Canonical, fixed-size V1 evidence bytes.
    ///
    /// Measured values, unmeasured values and upstream refusals have distinct
    /// tags.  Payload-less variants still occupy a zeroed eight-byte payload,
    /// so neither an omitted value nor a numeric sentinel can alias zero.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V1] {
        canonical_evidence_bytes(&self.values)
    }

    /// Decodes and revalidates one exact canonical V1 evidence record.
    ///
    /// # Errors
    ///
    /// Refuses every wrong length, header field, tag, nonzero payload reserved
    /// by a payload-less state, or value rejected by [`Self::new`].
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV1> {
        decode_evidence(bytes)
    }

    /// Domain-separated BLAKE3 of [`Self::canonical_bytes`].
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }
}

fn refuse_unsupported_measured_pbo_v1(
    values: &AdmissionEvidenceValuesV1,
) -> Result<(), AdmissionEvidenceRefusalV1> {
    for (observed, field) in [
        (values.pbo_ppm, AdmissionEvidenceFieldV1::Pbo),
        (
            values.pbo_contributing_folds,
            AdmissionEvidenceFieldV1::PboContributingFolds,
        ),
        (
            values.pbo_unrankable_folds,
            AdmissionEvidenceFieldV1::PboUnrankableFolds,
        ),
    ] {
        if matches!(observed, ObservedU64V1::Measured(_)) {
            return Err(AdmissionEvidenceRefusalV1 {
                field,
                kind: AdmissionEvidenceRefusalKindV1::UnsupportedMeasuredPboV1,
            });
        }
    }
    Ok(())
}

fn validate_observed_ppm(
    observed: ObservedU64V1,
    field: AdmissionEvidenceFieldV1,
) -> Result<(), AdmissionEvidenceRefusalV1> {
    if matches!(observed, ObservedU64V1::Measured(value) if value > PPM) {
        return Err(AdmissionEvidenceRefusalV1 {
            field,
            kind: AdmissionEvidenceRefusalKindV1::AboveOneMillion,
        });
    }
    Ok(())
}

fn reconcile_trade_counts_and_rates(
    values: &AdmissionEvidenceValuesV1,
) -> Result<(), AdmissionEvidenceRefusalV1> {
    if let (
        ObservedU64V1::Measured(trades),
        ObservedU64V1::Measured(wins),
        ObservedU64V1::Measured(losses),
    ) = (values.trades, values.winning_trades, values.losing_trades)
    {
        let Some(classified) = wins.checked_add(losses) else {
            return Err(AdmissionEvidenceRefusalV1 {
                field: AdmissionEvidenceFieldV1::TradeCounts,
                kind: AdmissionEvidenceRefusalKindV1::TradeCountOverflow,
            });
        };
        if classified != trades {
            return Err(AdmissionEvidenceRefusalV1 {
                field: AdmissionEvidenceFieldV1::TradeCounts,
                kind: AdmissionEvidenceRefusalKindV1::TradeCountMismatch,
            });
        }
    }
    validate_count_rate(
        values.trades,
        values.winning_trades,
        values.win_rate_ppm,
        AdmissionEvidenceFieldV1::WinRate,
    )?;
    validate_count_rate(
        values.trades,
        values.losing_trades,
        values.losing_trade_rate_ppm,
        AdmissionEvidenceFieldV1::LosingTradeRate,
    )
}

fn validate_count_rate(
    total: ObservedU64V1,
    count: ObservedU64V1,
    rate: ObservedU64V1,
    field: AdmissionEvidenceFieldV1,
) -> Result<(), AdmissionEvidenceRefusalV1> {
    let ObservedU64V1::Measured(rate) = rate else {
        return Ok(());
    };
    let (ObservedU64V1::Measured(total), ObservedU64V1::Measured(count)) = (total, count) else {
        return Err(AdmissionEvidenceRefusalV1 {
            field,
            kind: AdmissionEvidenceRefusalKindV1::RateWithoutCounts,
        });
    };
    if total == 0 {
        return Err(AdmissionEvidenceRefusalV1 {
            field,
            kind: AdmissionEvidenceRefusalKindV1::RateWithZeroDenominator,
        });
    }
    let Ok(projected) = u64::try_from(u128::from(count) * u128::from(PPM) / u128::from(total))
    else {
        return Err(AdmissionEvidenceRefusalV1 {
            field,
            kind: AdmissionEvidenceRefusalKindV1::RateCountMismatch,
        });
    };
    if projected != rate {
        return Err(AdmissionEvidenceRefusalV1 {
            field,
            kind: AdmissionEvidenceRefusalKindV1::RateCountMismatch,
        });
    }
    Ok(())
}

fn reconcile_measured_subsets(
    values: &AdmissionEvidenceValuesV1,
) -> Result<(), AdmissionEvidenceRefusalV1> {
    validate_subset(
        values.support_hits,
        values.independent_sessions,
        AdmissionEvidenceFieldV1::SupportSessions,
    )?;
    validate_subset(
        values.trades,
        values.winning_trades,
        AdmissionEvidenceFieldV1::TradeCounts,
    )?;
    validate_subset(
        values.trades,
        values.losing_trades,
        AdmissionEvidenceFieldV1::TradeCounts,
    )?;
    validate_subset(
        values.winning_trades,
        values.consecutive_winning_streak,
        AdmissionEvidenceFieldV1::WinningStreak,
    )?;
    validate_subset(
        values.losing_trades,
        values.consecutive_losing_streak,
        AdmissionEvidenceFieldV1::LosingStreak,
    )?;
    validate_subset(
        values.decided_folds,
        values.profitable_oos_folds,
        AdmissionEvidenceFieldV1::OosFolds,
    )
}

fn validate_subset(
    total: ObservedU64V1,
    subset: ObservedU64V1,
    field: AdmissionEvidenceFieldV1,
) -> Result<(), AdmissionEvidenceRefusalV1> {
    if let (ObservedU64V1::Measured(total), ObservedU64V1::Measured(subset)) = (total, subset)
        && subset > total
    {
        return Err(AdmissionEvidenceRefusalV1 {
            field,
            kind: AdmissionEvidenceRefusalKindV1::SubsetExceedsTotal,
        });
    }
    Ok(())
}

/// Append-only reason positions for policy V1.
///
/// Numeric values are canonical-codec identifiers. Never renumber or reuse one;
/// append a new value and introduce a new policy version instead.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionReasonV1 {
    /// Support hits below the floor.
    Support = 0,
    /// Too few independent sessions.
    SessionIndependence = 1,
    /// Too few closed trades.
    Trades = 2,
    /// Maximum adverse excursion above the ceiling.
    MaxMae = 3,
    /// Worst-case reward/risk below the floor.
    WorstRewardRisk = 4,
    /// Win rate below the floor.
    WinRate = 5,
    /// Wilson lower bound below the floor.
    WilsonWinRate = 6,
    /// Return/drawdown below the floor.
    ReturnDrawdown = 7,
    /// Weakest-period return below the floor.
    WeakestPeriod = 8,
    /// PBO above the ceiling.
    Pbo = 9,
    /// Family-wise adjusted p-value above the ceiling.
    Fwer = 10,
    /// SPA p-value above the ceiling.
    Spa = 11,
    /// Too few decided folds.
    DecidedFolds = 12,
    /// Ambiguous-fill rate above the ceiling.
    AmbiguousFills = 13,
    /// Gap-affected rate above the ceiling.
    GapAffected = 14,
    /// Session concentration above the ceiling.
    SessionConcentration = 15,
    /// Largest-trade profit share above the ceiling.
    LargestTradeProfitShare = 16,
    /// Execution evidence was not complete.
    ExecutionCompleteness = 17,
    /// Input data was not complete.
    DataCompleteness = 18,
    /// IST calendar evidence was not complete.
    CalendarCompleteness = 19,
    /// Candidate population was not complete.
    PopulationCompleteness = 20,
    /// Drawdown magnitude exceeded the ceiling.
    Drawdown = 21,
    /// Worst-trade loss magnitude exceeded the ceiling.
    WorstTradeLoss = 22,
    /// Losing-trade rate exceeded the ceiling.
    LosingTradeRate = 23,
    /// Losing-trade count exceeded the ceiling.
    LosingTrades = 24,
    /// Pessimistic total profit fell below the floor.
    PessimisticProfit = 25,
    /// Winning-trade count fell below the floor.
    WinningTrades = 26,
    /// Average winning-trade magnitude fell below the floor.
    AverageWin = 27,
    /// Average losing-trade magnitude exceeded the ceiling.
    AverageLoss = 28,
    /// Profit factor fell below the floor.
    ProfitFactor = 29,
    /// Consecutive losing-trade streak exceeded the ceiling.
    ConsecutiveLosingStreak = 30,
    /// Consecutive winning-trade streak fell below the floor.
    ConsecutiveWinningStreak = 31,
    /// Bootstrap draw count fell below the calibration floor.
    BootstrapDraws = 32,
    /// Compared strategy count fell below the calibration floor.
    BootstrapStrategies = 33,
    /// Aligned bootstrap period count fell below the calibration floor.
    BootstrapPeriods = 34,
    /// Too few PBO folds contributed valid placements.
    PboContributingFolds = 35,
    /// Too many PBO folds were unrankable.
    PboUnrankableFolds = 36,
    /// Too few out-of-sample folds were profitable pessimistically.
    ProfitableOosFolds = 37,
    /// Aggregate pessimistic out-of-sample return fell below the floor.
    OosPessimisticReturn = 38,
    /// White Reality Check p-value exceeded the ceiling.
    WhiteRealityPValue = 39,
    /// White Reality Check did not supply the required null rejection.
    WhiteRealityDecision = 40,
    /// Candidate-specific Romano–Wolf p-value exceeded the ceiling.
    RomanoWolfPValue = 41,
    /// Romano–Wolf did not supply the required candidate null rejection.
    RomanoWolfDecision = 42,
    /// Full-precision source statistics or denominators were incomplete.
    FullPrecisionStatisticsCompleteness = 43,
}

impl AdmissionReasonV1 {
    /// All V1 reasons in stable bit order.
    pub const ALL: [Self; 44] = [
        Self::Support,
        Self::SessionIndependence,
        Self::Trades,
        Self::MaxMae,
        Self::WorstRewardRisk,
        Self::WinRate,
        Self::WilsonWinRate,
        Self::ReturnDrawdown,
        Self::WeakestPeriod,
        Self::Pbo,
        Self::Fwer,
        Self::Spa,
        Self::DecidedFolds,
        Self::AmbiguousFills,
        Self::GapAffected,
        Self::SessionConcentration,
        Self::LargestTradeProfitShare,
        Self::ExecutionCompleteness,
        Self::DataCompleteness,
        Self::CalendarCompleteness,
        Self::PopulationCompleteness,
        Self::Drawdown,
        Self::WorstTradeLoss,
        Self::LosingTradeRate,
        Self::LosingTrades,
        Self::PessimisticProfit,
        Self::WinningTrades,
        Self::AverageWin,
        Self::AverageLoss,
        Self::ProfitFactor,
        Self::ConsecutiveLosingStreak,
        Self::ConsecutiveWinningStreak,
        Self::BootstrapDraws,
        Self::BootstrapStrategies,
        Self::BootstrapPeriods,
        Self::PboContributingFolds,
        Self::PboUnrankableFolds,
        Self::ProfitableOosFolds,
        Self::OosPessimisticReturn,
        Self::WhiteRealityPValue,
        Self::WhiteRealityDecision,
        Self::RomanoWolfPValue,
        Self::RomanoWolfDecision,
        Self::FullPrecisionStatisticsCompleteness,
    ];

    /// Stable machine-readable label used by audit and dashboard tables.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Support => "support",
            Self::SessionIndependence => "session_independence",
            Self::Trades => "trades",
            Self::MaxMae => "max_mae",
            Self::WorstRewardRisk => "worst_reward_risk",
            Self::WinRate => "win_rate",
            Self::WilsonWinRate => "wilson_win_rate",
            Self::ReturnDrawdown => "return_drawdown",
            Self::WeakestPeriod => "weakest_period",
            Self::Pbo => "pbo",
            Self::Fwer => "fwer",
            Self::Spa => "spa",
            Self::DecidedFolds => "decided_folds",
            Self::AmbiguousFills => "ambiguous_fills",
            Self::GapAffected => "gap_affected",
            Self::SessionConcentration => "session_concentration",
            Self::LargestTradeProfitShare => "largest_trade_profit_share",
            Self::ExecutionCompleteness => "execution_completeness",
            Self::DataCompleteness => "data_completeness",
            Self::CalendarCompleteness => "calendar_completeness",
            Self::PopulationCompleteness => "population_completeness",
            Self::Drawdown => "drawdown",
            Self::WorstTradeLoss => "worst_trade_loss",
            Self::LosingTradeRate => "losing_trade_rate",
            Self::LosingTrades => "losing_trades",
            Self::PessimisticProfit => "pessimistic_profit",
            Self::WinningTrades => "winning_trades",
            Self::AverageWin => "average_win",
            Self::AverageLoss => "average_loss",
            Self::ProfitFactor => "profit_factor",
            Self::ConsecutiveLosingStreak => "consecutive_losing_streak",
            Self::ConsecutiveWinningStreak => "consecutive_winning_streak",
            Self::BootstrapDraws => "bootstrap_draws",
            Self::BootstrapStrategies => "bootstrap_strategies",
            Self::BootstrapPeriods => "bootstrap_periods",
            Self::PboContributingFolds => "pbo_contributing_folds",
            Self::PboUnrankableFolds => "pbo_unrankable_folds",
            Self::ProfitableOosFolds => "profitable_oos_folds",
            Self::OosPessimisticReturn => "oos_pessimistic_return",
            Self::WhiteRealityPValue => "white_reality_p_value",
            Self::WhiteRealityDecision => "white_reality_decision",
            Self::RomanoWolfPValue => "romano_wolf_p_value",
            Self::RomanoWolfDecision => "romano_wolf_decision",
            Self::FullPrecisionStatisticsCompleteness => "full_precision_statistics_completeness",
        }
    }

    const fn mask(self) -> u64 {
        1_u64 << (self as u8)
    }
}

/// Fixed, append-only reason bit set.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasonBits(u64);

impl ReasonBits {
    /// No admission reason.
    pub const EMPTY: Self = Self(0);
    /// Mask containing every V1 reason bit and no reserved bit.
    pub const KNOWN: Self = Self((1_u64 << 44) - 1);

    /// Builds the singleton set for a stable reason.
    #[must_use]
    pub const fn one(reason: AdmissionReasonV1) -> Self {
        Self(reason.mask())
    }

    /// Decodes candidate future-codec bits, refusing unknown positions.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Option<Self> {
        if bits & !Self::KNOWN.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Raw stable in-memory bits for a future canonical encoder.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Whether the set contains no reason.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether one reason is present.
    #[must_use]
    pub const fn contains(self, reason: AdmissionReasonV1) -> bool {
        self.0 & reason.mask() != 0
    }

    /// Number of V1 reasons present.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    const fn insert(&mut self, reason: AdmissionReasonV1) {
        self.0 |= reason.mask();
    }

    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

/// Terminal classification of one admission evaluation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionStatusV1 {
    /// Every V1 check passed with measured, complete evidence.
    Admitted = 0,
    /// Every input was measured, and one or more policy checks failed.
    Rejected = 1,
    /// At least one required input was unmeasured and none was upstream-refused.
    Unmeasured = 2,
    /// At least one required input was explicitly refused upstream.
    Refused = 3,
}

/// Complete, fixed-layout admission result.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionVerdictV1 {
    reasons: ReasonBits,
    failed: ReasonBits,
    unmeasured: ReasonBits,
    refused: ReasonBits,
    status: AdmissionStatusV1,
}

impl AdmissionVerdictV1 {
    fn from_parts(failed: ReasonBits, unmeasured: ReasonBits, refused: ReasonBits) -> Self {
        let reasons = failed.union(unmeasured).union(refused);
        let status = if !refused.is_empty() {
            AdmissionStatusV1::Refused
        } else if !unmeasured.is_empty() {
            AdmissionStatusV1::Unmeasured
        } else if !failed.is_empty() {
            AdmissionStatusV1::Rejected
        } else {
            AdmissionStatusV1::Admitted
        };
        Self {
            reasons,
            failed,
            unmeasured,
            refused,
            status,
        }
    }

    /// Union of measured failures, unmeasured inputs and upstream refusals.
    #[must_use]
    pub const fn reasons(self) -> ReasonBits {
        self.reasons
    }

    /// Checks whose measured value violated policy.
    #[must_use]
    pub const fn failed(self) -> ReasonBits {
        self.failed
    }

    /// Checks that could not be measured.
    #[must_use]
    pub const fn unmeasured(self) -> ReasonBits {
        self.unmeasured
    }

    /// Checks explicitly refused by an upstream validator.
    #[must_use]
    pub const fn refused(self) -> ReasonBits {
        self.refused
    }

    /// Terminal classification with refused-before-unmeasured precedence.
    #[must_use]
    pub const fn status(self) -> AdmissionStatusV1 {
        self.status
    }

    /// True if and only if no reason bit is set.
    #[must_use]
    pub const fn is_admitted(self) -> bool {
        self.reasons.is_empty()
    }

    /// Whether all partitions, their union and status agree.
    #[must_use]
    pub const fn reconciles(self) -> bool {
        let disjoint = !self.failed.intersects(self.unmeasured)
            && !self.failed.intersects(self.refused)
            && !self.unmeasured.intersects(self.refused);
        let union_matches =
            self.reasons.0 == self.failed.union(self.unmeasured).union(self.refused).0;
        let status_matches = match self.status {
            AdmissionStatusV1::Admitted => self.reasons.is_empty(),
            AdmissionStatusV1::Rejected => {
                !self.failed.is_empty() && self.unmeasured.is_empty() && self.refused.is_empty()
            }
            AdmissionStatusV1::Unmeasured => !self.unmeasured.is_empty() && self.refused.is_empty(),
            AdmissionStatusV1::Refused => !self.refused.is_empty(),
        };
        disjoint && union_matches && status_matches
    }

    /// Canonical, fixed-size V1 verdict bytes.
    ///
    /// All four reason partitions and the terminal status are encoded even
    /// though some are derivable.  This makes persisted corruption visible
    /// instead of silently repairing it while hashing.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; ADMISSION_VERDICT_CANONICAL_LEN_V1] {
        canonical_verdict_bytes(self)
    }

    /// Decodes one exact canonical V1 verdict and proves its redundancy agrees.
    ///
    /// # Errors
    ///
    /// Refuses unknown reason bits, overlapping partitions, a wrong reason
    /// union, a wrong status, or any invalid canonical header or tag.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV1> {
        decode_verdict(bytes)
    }

    /// Domain-separated BLAKE3 of [`Self::canonical_bytes`].
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }
}

/// Recomputable identity seal for one institutional admission decision.
///
/// New decisions are constructed through [`AdmissionPolicyV1::evaluate_sealed`].
/// Persisted decisions may be restored only through this type's canonical
/// decoders, which revalidate policy and evidence and recompute the verdict.
/// The seal therefore always contains the exact validated policy and evidence
/// plus their computed verdict; callers cannot supply a preferred verdict.
/// Persisting the canonical bytes or digest remains the caller's responsibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionDecisionSealV1 {
    policy: AdmissionPolicyV1,
    evidence: AdmissionEvidenceV1,
    verdict: AdmissionVerdictV1,
}

impl AdmissionDecisionSealV1 {
    fn new(policy: &AdmissionPolicyV1, evidence: &AdmissionEvidenceV1) -> Self {
        let verdict = policy.evaluate(evidence);
        Self {
            policy: *policy,
            evidence: *evidence,
            verdict,
        }
    }

    /// Reconstructs an authoritative seal from three canonical records.
    ///
    /// Policy and evidence are decoded through their validating constructors.
    /// The supplied verdict must be canonical and byte-for-byte identical to a
    /// fresh [`AdmissionPolicyV1::evaluate`] result.  Callers therefore cannot
    /// authorize a preferred verdict by merely hashing it into a new seal.
    ///
    /// # Errors
    ///
    /// Returns the exact nested decode refusal, or
    /// [`AdmissionCanonicalRefusalV1::VerdictMismatch`] when the supplied
    /// verdict is valid in isolation but is not the result of this policy and
    /// evidence.
    pub fn from_canonical_parts(
        policy_bytes: &[u8],
        evidence_bytes: &[u8],
        verdict_bytes: &[u8],
    ) -> Result<Self, AdmissionCanonicalRefusalV1> {
        let policy = AdmissionPolicyV1::from_canonical_bytes(policy_bytes)?;
        let evidence = AdmissionEvidenceV1::from_canonical_bytes(evidence_bytes)?;
        let supplied = AdmissionVerdictV1::from_canonical_bytes(verdict_bytes)?;
        let computed = policy.evaluate(&evidence);
        if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
            return Err(AdmissionCanonicalRefusalV1::VerdictMismatch);
        }
        Ok(Self {
            policy,
            evidence,
            verdict: computed,
        })
    }

    /// Decodes a complete canonical V1 decision and re-authorizes its verdict.
    ///
    /// # Errors
    ///
    /// Refuses an invalid outer record, any invalid nested record, or a nested
    /// verdict that does not exactly match recomputation.  Recomputing an outer
    /// digest over a forged nested verdict therefore supplies no authority.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV1> {
        decode_decision(bytes)
    }

    /// Exact policy carried by this seal.
    #[must_use]
    pub const fn policy(self) -> AdmissionPolicyV1 {
        self.policy
    }

    /// Exact evidence carried by this seal.
    #[must_use]
    pub const fn evidence(self) -> AdmissionEvidenceV1 {
        self.evidence
    }

    /// Verdict computed from this seal's policy and evidence.
    #[must_use]
    pub const fn verdict(self) -> AdmissionVerdictV1 {
        self.verdict
    }

    /// Whether recomputing from the sealed policy and evidence is byte-exact.
    #[must_use]
    pub fn recomputes(self) -> bool {
        self.policy.evaluate(&self.evidence) == self.verdict
    }

    /// Canonical policy, evidence and verdict in one versioned record.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; ADMISSION_DECISION_CANONICAL_LEN_V1] {
        let policy = self.policy.canonical_bytes();
        let evidence = self.evidence.canonical_bytes();
        let verdict = self.verdict.canonical_bytes();
        let mut bytes = [0_u8; ADMISSION_DECISION_CANONICAL_LEN_V1];
        let mut writer =
            CanonicalWriter::new(&mut bytes, DECISION_DOMAIN_V1, DECISION_PAYLOAD_LEN_V1);
        writer.put_slice(&policy);
        writer.put_slice(&evidence);
        writer.put_slice(&verdict);
        writer.finish();
        bytes
    }

    /// Domain-separated BLAKE3 of [`Self::canonical_bytes`].
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }
}

struct CanonicalWriter<'a> {
    bytes: &'a mut [u8],
    offset: usize,
}

impl<'a> CanonicalWriter<'a> {
    fn new(bytes: &'a mut [u8], domain: u8, payload_len: u32) -> Self {
        Self::new_version(bytes, domain, payload_len, ADMISSION_VERSION_V1)
    }

    fn new_version(bytes: &'a mut [u8], domain: u8, payload_len: u32, version: u16) -> Self {
        let mut writer = Self { bytes, offset: 0 };
        writer.put_slice(b"BADM");
        writer.put_slice(&[domain, 0]);
        writer.put_slice(&version.to_le_bytes());
        writer.put_slice(&payload_len.to_le_bytes());
        debug_assert_eq!(writer.offset, CANONICAL_HEADER_LEN);
        writer
    }

    fn put_slice(&mut self, value: &[u8]) {
        debug_assert!(self.bytes.len().saturating_sub(self.offset) >= value.len());
        for (target, source) in self.bytes.iter_mut().skip(self.offset).zip(value.iter()) {
            *target = *source;
        }
        self.offset += value.len();
    }

    fn put_u64(&mut self, value: u64) {
        self.put_slice(&value.to_le_bytes());
    }

    fn put_i64(&mut self, value: i64) {
        self.put_slice(&value.to_le_bytes());
    }

    fn put_bool(&mut self, value: bool) {
        self.put_slice(&[u8::from(value)]);
    }

    fn finish(self) {
        debug_assert_eq!(self.offset, self.bytes.len());
    }
}

struct CanonicalReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    record: AdmissionCanonicalRecordV1,
}

impl<'a> CanonicalReader<'a> {
    fn new(
        bytes: &'a [u8],
        record: AdmissionCanonicalRecordV1,
        expected_len: usize,
        expected_domain: u8,
        expected_payload_len: u32,
    ) -> Result<Self, AdmissionCanonicalRefusalV1> {
        if bytes.len() != expected_len {
            return Err(AdmissionCanonicalRefusalV1::Length {
                record,
                expected: expected_len,
                actual: bytes.len(),
            });
        }
        let mut reader = Self {
            bytes,
            offset: 0,
            record,
        };
        if reader.take::<4>()? != *b"BADM" {
            return Err(AdmissionCanonicalRefusalV1::Magic { record });
        }
        let domain = reader.read_u8()?;
        if domain != expected_domain {
            return Err(AdmissionCanonicalRefusalV1::Domain {
                record,
                actual: domain,
            });
        }
        let reserved_offset = reader.offset;
        if reader.read_u8()? != 0 {
            return Err(AdmissionCanonicalRefusalV1::Reserved {
                record,
                offset: reserved_offset,
            });
        }
        let version = reader.read_u16()?;
        if version != ADMISSION_VERSION_V1 {
            return Err(AdmissionCanonicalRefusalV1::Version {
                record,
                actual: version,
            });
        }
        let payload_len = reader.read_u32()?;
        if payload_len != expected_payload_len {
            return Err(AdmissionCanonicalRefusalV1::PayloadLength {
                record,
                actual: payload_len,
            });
        }
        debug_assert_eq!(reader.offset, CANONICAL_HEADER_LEN);
        Ok(reader)
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], AdmissionCanonicalRefusalV1> {
        let Some(end) = self.offset.checked_add(N) else {
            return Err(AdmissionCanonicalRefusalV1::Length {
                record: self.record,
                expected: self.offset,
                actual: self.bytes.len(),
            });
        };
        let Some(source) = self.bytes.get(self.offset..end) else {
            return Err(AdmissionCanonicalRefusalV1::Length {
                record: self.record,
                expected: end,
                actual: self.bytes.len(),
            });
        };
        let mut value = [0_u8; N];
        value.copy_from_slice(source);
        self.offset = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, AdmissionCanonicalRefusalV1> {
        Ok(self.take::<1>()?[0])
    }

    fn read_u16(&mut self) -> Result<u16, AdmissionCanonicalRefusalV1> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    fn read_u32(&mut self) -> Result<u32, AdmissionCanonicalRefusalV1> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    fn read_u64(&mut self) -> Result<u64, AdmissionCanonicalRefusalV1> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    fn read_i64(&mut self) -> Result<i64, AdmissionCanonicalRefusalV1> {
        Ok(i64::from_le_bytes(self.take()?))
    }

    fn read_bool(&mut self) -> Result<bool, AdmissionCanonicalRefusalV1> {
        let offset = self.offset;
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            actual => Err(AdmissionCanonicalRefusalV1::Tag {
                record: self.record,
                offset,
                actual,
            }),
        }
    }

    fn read_observed_u64(&mut self) -> Result<ObservedU64V1, AdmissionCanonicalRefusalV1> {
        let tag_offset = self.offset;
        let tag = self.read_u8()?;
        let payload_offset = self.offset;
        let payload = self.read_u64()?;
        match tag {
            0 => Ok(ObservedU64V1::Measured(payload)),
            1 if payload == 0 => Ok(ObservedU64V1::Unmeasured),
            2 if payload == 0 => Ok(ObservedU64V1::Refused),
            1 | 2 => Err(AdmissionCanonicalRefusalV1::Reserved {
                record: self.record,
                offset: payload_offset,
            }),
            actual => Err(AdmissionCanonicalRefusalV1::Tag {
                record: self.record,
                offset: tag_offset,
                actual,
            }),
        }
    }

    fn read_observed_i64(&mut self) -> Result<ObservedI64V1, AdmissionCanonicalRefusalV1> {
        let tag_offset = self.offset;
        let tag = self.read_u8()?;
        let payload_offset = self.offset;
        let payload = self.read_i64()?;
        match tag {
            0 => Ok(ObservedI64V1::Measured(payload)),
            1 if payload == 0 => Ok(ObservedI64V1::Unmeasured),
            2 if payload == 0 => Ok(ObservedI64V1::Refused),
            1 | 2 => Err(AdmissionCanonicalRefusalV1::Reserved {
                record: self.record,
                offset: payload_offset,
            }),
            actual => Err(AdmissionCanonicalRefusalV1::Tag {
                record: self.record,
                offset: tag_offset,
                actual,
            }),
        }
    }

    fn read_completeness(&mut self) -> Result<CompletenessV1, AdmissionCanonicalRefusalV1> {
        let offset = self.offset;
        match self.read_u8()? {
            0 => Ok(CompletenessV1::Complete),
            1 => Ok(CompletenessV1::Incomplete),
            2 => Ok(CompletenessV1::Unmeasured),
            3 => Ok(CompletenessV1::Refused),
            actual => Err(AdmissionCanonicalRefusalV1::Tag {
                record: self.record,
                offset,
                actual,
            }),
        }
    }

    fn read_hypothesis_decision(
        &mut self,
    ) -> Result<HypothesisDecisionV1, AdmissionCanonicalRefusalV1> {
        let offset = self.offset;
        match self.read_u8()? {
            0 => Ok(HypothesisDecisionV1::RejectedNull),
            1 => Ok(HypothesisDecisionV1::DidNotReject),
            2 => Ok(HypothesisDecisionV1::Unmeasured),
            3 => Ok(HypothesisDecisionV1::Refused),
            actual => Err(AdmissionCanonicalRefusalV1::Tag {
                record: self.record,
                offset,
                actual,
            }),
        }
    }

    fn finish(self) {
        debug_assert_eq!(self.offset, self.bytes.len());
    }
}

fn decode_policy(bytes: &[u8]) -> Result<AdmissionPolicyV1, AdmissionCanonicalRefusalV1> {
    let mut reader = CanonicalReader::new(
        bytes,
        AdmissionCanonicalRecordV1::Policy,
        ADMISSION_POLICY_CANONICAL_LEN_V1,
        POLICY_DOMAIN_V1,
        POLICY_PAYLOAD_LEN_V1,
    )?;
    let draft = AdmissionPolicyDraftV1 {
        min_support_hits: Some(reader.read_u64()?),
        min_independent_sessions: Some(reader.read_u64()?),
        min_trades: Some(reader.read_u64()?),
        max_mae_paisa: Some(reader.read_u64()?),
        min_worst_reward_risk_ppm: Some(reader.read_u64()?),
        min_win_rate_ppm: Some(reader.read_u64()?),
        min_wilson_win_rate_ppm: Some(reader.read_u64()?),
        min_return_drawdown_ppm: Some(reader.read_u64()?),
        min_weakest_period_return_paisa: Some(reader.read_i64()?),
        max_pbo_ppm: Some(reader.read_u64()?),
        max_fwer_p_value_ppm: Some(reader.read_u64()?),
        max_spa_p_value_ppm: Some(reader.read_u64()?),
        min_decided_folds: Some(reader.read_u64()?),
        max_ambiguous_fill_rate_ppm: Some(reader.read_u64()?),
        max_gap_affected_rate_ppm: Some(reader.read_u64()?),
        max_session_concentration_ppm: Some(reader.read_u64()?),
        max_largest_trade_profit_share_ppm: Some(reader.read_u64()?),
        max_drawdown_paisa: Some(reader.read_u64()?),
        max_worst_trade_loss_paisa: Some(reader.read_u64()?),
        max_losing_trade_rate_ppm: Some(reader.read_u64()?),
        max_losing_trades: Some(reader.read_u64()?),
        min_pessimistic_profit_paisa: Some(reader.read_i64()?),
        min_winning_trades: Some(reader.read_u64()?),
        min_average_win_paisa: Some(reader.read_u64()?),
        max_average_loss_paisa: Some(reader.read_u64()?),
        min_profit_factor_ppm: Some(reader.read_u64()?),
        max_consecutive_losing_streak: Some(reader.read_u64()?),
        min_consecutive_winning_streak: Some(reader.read_u64()?),
        min_bootstrap_draws: Some(reader.read_u64()?),
        min_bootstrap_strategies: Some(reader.read_u64()?),
        min_bootstrap_periods: Some(reader.read_u64()?),
        min_pbo_contributing_folds: Some(reader.read_u64()?),
        max_pbo_unrankable_folds: Some(reader.read_u64()?),
        min_profitable_oos_folds: Some(reader.read_u64()?),
        min_oos_pessimistic_return_paisa: Some(reader.read_i64()?),
        max_white_reality_p_value_ppm: Some(reader.read_u64()?),
        require_white_reality_rejection: Some(reader.read_bool()?),
        max_romano_wolf_p_value_ppm: Some(reader.read_u64()?),
        require_romano_wolf_rejection: Some(reader.read_bool()?),
    };
    reader.finish();
    AdmissionPolicyV1::new(draft).map_err(AdmissionCanonicalRefusalV1::Policy)
}

fn decode_evidence(bytes: &[u8]) -> Result<AdmissionEvidenceV1, AdmissionCanonicalRefusalV1> {
    let mut reader = CanonicalReader::new(
        bytes,
        AdmissionCanonicalRecordV1::Evidence,
        ADMISSION_EVIDENCE_CANONICAL_LEN_V1,
        EVIDENCE_DOMAIN_V1,
        EVIDENCE_PAYLOAD_LEN_V1,
    )?;
    let values = AdmissionEvidenceValuesV1 {
        support_hits: reader.read_observed_u64()?,
        independent_sessions: reader.read_observed_u64()?,
        trades: reader.read_observed_u64()?,
        max_mae_paisa: reader.read_observed_u64()?,
        worst_reward_risk_ppm: reader.read_observed_u64()?,
        win_rate_ppm: reader.read_observed_u64()?,
        wilson_win_rate_ppm: reader.read_observed_u64()?,
        return_drawdown_ppm: reader.read_observed_u64()?,
        weakest_period_return_paisa: reader.read_observed_i64()?,
        pbo_ppm: reader.read_observed_u64()?,
        fwer_p_value_ppm: reader.read_observed_u64()?,
        spa_p_value_ppm: reader.read_observed_u64()?,
        decided_folds: reader.read_observed_u64()?,
        ambiguous_fill_rate_ppm: reader.read_observed_u64()?,
        gap_affected_rate_ppm: reader.read_observed_u64()?,
        session_concentration_ppm: reader.read_observed_u64()?,
        largest_trade_profit_share_ppm: reader.read_observed_u64()?,
        execution_complete: reader.read_completeness()?,
        data_complete: reader.read_completeness()?,
        calendar_complete: reader.read_completeness()?,
        population_complete: reader.read_completeness()?,
        drawdown_paisa: reader.read_observed_u64()?,
        worst_trade_loss_paisa: reader.read_observed_u64()?,
        losing_trade_rate_ppm: reader.read_observed_u64()?,
        losing_trades: reader.read_observed_u64()?,
        pessimistic_profit_paisa: reader.read_observed_i64()?,
        winning_trades: reader.read_observed_u64()?,
        average_win_paisa: reader.read_observed_u64()?,
        average_loss_paisa: reader.read_observed_u64()?,
        profit_factor_ppm: reader.read_observed_u64()?,
        consecutive_losing_streak: reader.read_observed_u64()?,
        consecutive_winning_streak: reader.read_observed_u64()?,
        bootstrap_draws: reader.read_observed_u64()?,
        bootstrap_strategies: reader.read_observed_u64()?,
        bootstrap_periods: reader.read_observed_u64()?,
        pbo_contributing_folds: reader.read_observed_u64()?,
        pbo_unrankable_folds: reader.read_observed_u64()?,
        profitable_oos_folds: reader.read_observed_u64()?,
        oos_pessimistic_return_paisa: reader.read_observed_i64()?,
        white_reality_p_value_ppm: reader.read_observed_u64()?,
        romano_wolf_p_value_ppm: reader.read_observed_u64()?,
        white_reality_decision: reader.read_hypothesis_decision()?,
        romano_wolf_decision: reader.read_hypothesis_decision()?,
        full_precision_statistics_complete: reader.read_completeness()?,
    };
    reader.finish();
    AdmissionEvidenceV1::new(values).map_err(AdmissionCanonicalRefusalV1::Evidence)
}

fn decode_reason_bits(
    reader: &mut CanonicalReader<'_>,
    partition: AdmissionVerdictPartitionV1,
) -> Result<ReasonBits, AdmissionCanonicalRefusalV1> {
    ReasonBits::from_bits(reader.read_u64()?)
        .ok_or(AdmissionCanonicalRefusalV1::UnknownReasonBits { partition })
}

fn decode_verdict(bytes: &[u8]) -> Result<AdmissionVerdictV1, AdmissionCanonicalRefusalV1> {
    let mut reader = CanonicalReader::new(
        bytes,
        AdmissionCanonicalRecordV1::Verdict,
        ADMISSION_VERDICT_CANONICAL_LEN_V1,
        VERDICT_DOMAIN_V1,
        VERDICT_PAYLOAD_LEN_V1,
    )?;
    let reasons = decode_reason_bits(&mut reader, AdmissionVerdictPartitionV1::Reasons)?;
    let failed = decode_reason_bits(&mut reader, AdmissionVerdictPartitionV1::Failed)?;
    let unmeasured = decode_reason_bits(&mut reader, AdmissionVerdictPartitionV1::Unmeasured)?;
    let refused = decode_reason_bits(&mut reader, AdmissionVerdictPartitionV1::Refused)?;
    let status_offset = reader.offset;
    let status = match reader.read_u8()? {
        0 => AdmissionStatusV1::Admitted,
        1 => AdmissionStatusV1::Rejected,
        2 => AdmissionStatusV1::Unmeasured,
        3 => AdmissionStatusV1::Refused,
        actual => {
            return Err(AdmissionCanonicalRefusalV1::Tag {
                record: AdmissionCanonicalRecordV1::Verdict,
                offset: status_offset,
                actual,
            });
        }
    };
    reader.finish();
    let verdict = AdmissionVerdictV1::from_parts(failed, unmeasured, refused);
    if !verdict.reconciles() || verdict.reasons != reasons || verdict.status != status {
        return Err(AdmissionCanonicalRefusalV1::VerdictInconsistent);
    }
    Ok(verdict)
}

fn decode_decision(bytes: &[u8]) -> Result<AdmissionDecisionSealV1, AdmissionCanonicalRefusalV1> {
    let mut reader = CanonicalReader::new(
        bytes,
        AdmissionCanonicalRecordV1::Decision,
        ADMISSION_DECISION_CANONICAL_LEN_V1,
        DECISION_DOMAIN_V1,
        DECISION_PAYLOAD_LEN_V1,
    )?;
    let policy = reader.take::<ADMISSION_POLICY_CANONICAL_LEN_V1>()?;
    let evidence = reader.take::<ADMISSION_EVIDENCE_CANONICAL_LEN_V1>()?;
    let verdict = reader.take::<ADMISSION_VERDICT_CANONICAL_LEN_V1>()?;
    reader.finish();
    AdmissionDecisionSealV1::from_canonical_parts(&policy, &evidence, &verdict)
}

fn canonical_policy_bytes(
    values: &AdmissionPolicyValuesV1,
) -> [u8; ADMISSION_POLICY_CANONICAL_LEN_V1] {
    let mut bytes = [0_u8; ADMISSION_POLICY_CANONICAL_LEN_V1];
    let mut writer = CanonicalWriter::new(&mut bytes, POLICY_DOMAIN_V1, POLICY_PAYLOAD_LEN_V1);
    writer.put_u64(values.min_support_hits);
    writer.put_u64(values.min_independent_sessions);
    writer.put_u64(values.min_trades);
    writer.put_u64(values.max_mae_paisa);
    writer.put_u64(values.min_worst_reward_risk_ppm);
    writer.put_u64(values.min_win_rate_ppm);
    writer.put_u64(values.min_wilson_win_rate_ppm);
    writer.put_u64(values.min_return_drawdown_ppm);
    writer.put_i64(values.min_weakest_period_return_paisa);
    writer.put_u64(values.max_pbo_ppm);
    writer.put_u64(values.max_fwer_p_value_ppm);
    writer.put_u64(values.max_spa_p_value_ppm);
    writer.put_u64(values.min_decided_folds);
    writer.put_u64(values.max_ambiguous_fill_rate_ppm);
    writer.put_u64(values.max_gap_affected_rate_ppm);
    writer.put_u64(values.max_session_concentration_ppm);
    writer.put_u64(values.max_largest_trade_profit_share_ppm);
    writer.put_u64(values.max_drawdown_paisa);
    writer.put_u64(values.max_worst_trade_loss_paisa);
    writer.put_u64(values.max_losing_trade_rate_ppm);
    writer.put_u64(values.max_losing_trades);
    writer.put_i64(values.min_pessimistic_profit_paisa);
    writer.put_u64(values.min_winning_trades);
    writer.put_u64(values.min_average_win_paisa);
    writer.put_u64(values.max_average_loss_paisa);
    writer.put_u64(values.min_profit_factor_ppm);
    writer.put_u64(values.max_consecutive_losing_streak);
    writer.put_u64(values.min_consecutive_winning_streak);
    writer.put_u64(values.min_bootstrap_draws);
    writer.put_u64(values.min_bootstrap_strategies);
    writer.put_u64(values.min_bootstrap_periods);
    writer.put_u64(values.min_pbo_contributing_folds);
    writer.put_u64(values.max_pbo_unrankable_folds);
    writer.put_u64(values.min_profitable_oos_folds);
    writer.put_i64(values.min_oos_pessimistic_return_paisa);
    writer.put_u64(values.max_white_reality_p_value_ppm);
    writer.put_bool(values.require_white_reality_rejection);
    writer.put_u64(values.max_romano_wolf_p_value_ppm);
    writer.put_bool(values.require_romano_wolf_rejection);
    writer.finish();
    bytes
}

fn put_observed_u64(writer: &mut CanonicalWriter<'_>, observed: ObservedU64V1) {
    match observed {
        ObservedU64V1::Measured(value) => {
            writer.put_slice(&[0]);
            writer.put_u64(value);
        }
        ObservedU64V1::Unmeasured => {
            writer.put_slice(&[1]);
            writer.put_u64(0);
        }
        ObservedU64V1::Refused => {
            writer.put_slice(&[2]);
            writer.put_u64(0);
        }
    }
}

fn put_observed_i64(writer: &mut CanonicalWriter<'_>, observed: ObservedI64V1) {
    match observed {
        ObservedI64V1::Measured(value) => {
            writer.put_slice(&[0]);
            writer.put_i64(value);
        }
        ObservedI64V1::Unmeasured => {
            writer.put_slice(&[1]);
            writer.put_i64(0);
        }
        ObservedI64V1::Refused => {
            writer.put_slice(&[2]);
            writer.put_i64(0);
        }
    }
}

fn put_completeness(writer: &mut CanonicalWriter<'_>, completeness: CompletenessV1) {
    let tag = match completeness {
        CompletenessV1::Complete => 0,
        CompletenessV1::Incomplete => 1,
        CompletenessV1::Unmeasured => 2,
        CompletenessV1::Refused => 3,
    };
    writer.put_slice(&[tag]);
}

fn put_hypothesis_decision(writer: &mut CanonicalWriter<'_>, decision: HypothesisDecisionV1) {
    let tag = match decision {
        HypothesisDecisionV1::RejectedNull => 0,
        HypothesisDecisionV1::DidNotReject => 1,
        HypothesisDecisionV1::Unmeasured => 2,
        HypothesisDecisionV1::Refused => 3,
    };
    writer.put_slice(&[tag]);
}

fn canonical_evidence_bytes(
    values: &AdmissionEvidenceValuesV1,
) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V1] {
    let mut bytes = [0_u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V1];
    let mut writer = CanonicalWriter::new(&mut bytes, EVIDENCE_DOMAIN_V1, EVIDENCE_PAYLOAD_LEN_V1);
    put_evidence_values(&mut writer, values);
    writer.finish();
    bytes
}

fn put_evidence_values(writer: &mut CanonicalWriter<'_>, values: &AdmissionEvidenceValuesV1) {
    put_observed_u64(writer, values.support_hits);
    put_observed_u64(writer, values.independent_sessions);
    put_observed_u64(writer, values.trades);
    put_observed_u64(writer, values.max_mae_paisa);
    put_observed_u64(writer, values.worst_reward_risk_ppm);
    put_observed_u64(writer, values.win_rate_ppm);
    put_observed_u64(writer, values.wilson_win_rate_ppm);
    put_observed_u64(writer, values.return_drawdown_ppm);
    put_observed_i64(writer, values.weakest_period_return_paisa);
    put_observed_u64(writer, values.pbo_ppm);
    put_observed_u64(writer, values.fwer_p_value_ppm);
    put_observed_u64(writer, values.spa_p_value_ppm);
    put_observed_u64(writer, values.decided_folds);
    put_observed_u64(writer, values.ambiguous_fill_rate_ppm);
    put_observed_u64(writer, values.gap_affected_rate_ppm);
    put_observed_u64(writer, values.session_concentration_ppm);
    put_observed_u64(writer, values.largest_trade_profit_share_ppm);
    put_completeness(writer, values.execution_complete);
    put_completeness(writer, values.data_complete);
    put_completeness(writer, values.calendar_complete);
    put_completeness(writer, values.population_complete);
    put_observed_u64(writer, values.drawdown_paisa);
    put_observed_u64(writer, values.worst_trade_loss_paisa);
    put_observed_u64(writer, values.losing_trade_rate_ppm);
    put_observed_u64(writer, values.losing_trades);
    put_observed_i64(writer, values.pessimistic_profit_paisa);
    put_observed_u64(writer, values.winning_trades);
    put_observed_u64(writer, values.average_win_paisa);
    put_observed_u64(writer, values.average_loss_paisa);
    put_observed_u64(writer, values.profit_factor_ppm);
    put_observed_u64(writer, values.consecutive_losing_streak);
    put_observed_u64(writer, values.consecutive_winning_streak);
    put_observed_u64(writer, values.bootstrap_draws);
    put_observed_u64(writer, values.bootstrap_strategies);
    put_observed_u64(writer, values.bootstrap_periods);
    put_observed_u64(writer, values.pbo_contributing_folds);
    put_observed_u64(writer, values.pbo_unrankable_folds);
    put_observed_u64(writer, values.profitable_oos_folds);
    put_observed_i64(writer, values.oos_pessimistic_return_paisa);
    put_observed_u64(writer, values.white_reality_p_value_ppm);
    put_observed_u64(writer, values.romano_wolf_p_value_ppm);
    put_hypothesis_decision(writer, values.white_reality_decision);
    put_hypothesis_decision(writer, values.romano_wolf_decision);
    put_completeness(writer, values.full_precision_statistics_complete);
}

fn canonical_verdict_bytes(
    verdict: AdmissionVerdictV1,
) -> [u8; ADMISSION_VERDICT_CANONICAL_LEN_V1] {
    let mut bytes = [0_u8; ADMISSION_VERDICT_CANONICAL_LEN_V1];
    let mut writer = CanonicalWriter::new(&mut bytes, VERDICT_DOMAIN_V1, VERDICT_PAYLOAD_LEN_V1);
    writer.put_u64(verdict.reasons.bits());
    writer.put_u64(verdict.failed.bits());
    writer.put_u64(verdict.unmeasured.bits());
    writer.put_u64(verdict.refused.bits());
    let status = match verdict.status {
        AdmissionStatusV1::Admitted => 0,
        AdmissionStatusV1::Rejected => 1,
        AdmissionStatusV1::Unmeasured => 2,
        AdmissionStatusV1::Refused => 3,
    };
    writer.put_slice(&[status]);
    writer.finish();
    bytes
}

fn record_observation(
    observation: ObservedU64V1,
    reason: AdmissionReasonV1,
    measured_failed: bool,
    failed: &mut ReasonBits,
    unmeasured: &mut ReasonBits,
    refused: &mut ReasonBits,
) {
    match observation {
        ObservedU64V1::Measured(_) => {
            if measured_failed {
                failed.insert(reason);
            }
        }
        ObservedU64V1::Unmeasured => unmeasured.insert(reason),
        ObservedU64V1::Refused => refused.insert(reason),
    }
}

fn check_min_u64(
    observation: ObservedU64V1,
    floor: u64,
    reason: AdmissionReasonV1,
    failed: &mut ReasonBits,
    unmeasured: &mut ReasonBits,
    refused: &mut ReasonBits,
) {
    let violates = matches!(observation, ObservedU64V1::Measured(value) if value < floor);
    record_observation(observation, reason, violates, failed, unmeasured, refused);
}

fn check_max_u64(
    observation: ObservedU64V1,
    ceiling: u64,
    reason: AdmissionReasonV1,
    failed: &mut ReasonBits,
    unmeasured: &mut ReasonBits,
    refused: &mut ReasonBits,
) {
    let violates = matches!(observation, ObservedU64V1::Measured(value) if value > ceiling);
    record_observation(observation, reason, violates, failed, unmeasured, refused);
}

fn check_min_i64(
    observation: ObservedI64V1,
    floor: i64,
    reason: AdmissionReasonV1,
    failed: &mut ReasonBits,
    unmeasured: &mut ReasonBits,
    refused: &mut ReasonBits,
) {
    match observation {
        ObservedI64V1::Measured(value) => {
            if value < floor {
                failed.insert(reason);
            }
        }
        ObservedI64V1::Unmeasured => unmeasured.insert(reason),
        ObservedI64V1::Refused => refused.insert(reason),
    }
}

fn check_completeness(
    completeness: CompletenessV1,
    reason: AdmissionReasonV1,
    failed: &mut ReasonBits,
    unmeasured: &mut ReasonBits,
    refused: &mut ReasonBits,
) {
    match completeness {
        CompletenessV1::Complete => {}
        CompletenessV1::Incomplete => failed.insert(reason),
        CompletenessV1::Unmeasured => unmeasured.insert(reason),
        CompletenessV1::Refused => refused.insert(reason),
    }
}

fn check_hypothesis_decision(
    decision: HypothesisDecisionV1,
    rejection_required: bool,
    reason: AdmissionReasonV1,
    failed: &mut ReasonBits,
    unmeasured: &mut ReasonBits,
    refused: &mut ReasonBits,
) {
    match decision {
        HypothesisDecisionV1::RejectedNull => {}
        HypothesisDecisionV1::DidNotReject => {
            if rejection_required {
                failed.insert(reason);
            }
        }
        HypothesisDecisionV1::Unmeasured => unmeasured.insert(reason),
        HypothesisDecisionV1::Refused => refused.insert(reason),
    }
}

// -------------------------------------------------------------------------
// Admission V2: two-source statistics + anchored walk-forward authority.
// -------------------------------------------------------------------------

const STATISTICS_DIGEST_COUNT_V2: usize = 13;
const STATISTICS_DIGEST_COUNT_V3: usize = 9;
const CANONICAL_FAMILY_ALPHA_PPM_V2: u64 = 50_000;

const WALK_FORWARD_AUTHORITY_DOMAIN_V2: &[u8] =
    b"brutex.admission.anchored-walk-forward.authority.v2\0";
const WALK_FORWARD_AUTHORITY_DOMAIN_V3: &[u8] = b"brutex.admission.anchored-search.authority.v3\0";

/// Exact rational probability supplied by Statistics V2.
///
/// This is the comparison source, not a rounded storage substitute. The ppm
/// projection used by the fixed policy is derived by integer floor division.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionExactProbabilityV2 {
    numerator: u64,
    denominator: u64,
}

impl AdmissionExactProbabilityV2 {
    /// Constructs one exact finite probability.
    ///
    /// # Errors
    ///
    /// Refuses a zero denominator or a numerator larger than its denominator.
    pub const fn new(
        numerator: u64,
        denominator: u64,
    ) -> Result<Self, AdmissionProbabilityRefusalV2> {
        if denominator == 0 {
            return Err(AdmissionProbabilityRefusalV2::ZeroDenominator);
        }
        if numerator > denominator {
            return Err(AdmissionProbabilityRefusalV2::NumeratorExceedsDenominator);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Exact numerator.
    #[must_use]
    pub const fn numerator(self) -> u64 {
        self.numerator
    }

    /// Exact nonzero denominator.
    #[must_use]
    pub const fn denominator(self) -> u64 {
        self.denominator
    }

    /// Canonical floor projection onto one million comparison points.
    #[must_use]
    pub fn ppm(self) -> u64 {
        let projected = u128::from(self.numerator) * u128::from(PPM) / u128::from(self.denominator);
        u64::try_from(projected).unwrap_or(PPM)
    }

    fn rejects_at_ppm(self, alpha_ppm: u64) -> bool {
        u128::from(self.numerator) * u128::from(PPM)
            <= u128::from(self.denominator) * u128::from(alpha_ppm)
    }
}

/// Exact malformed-fraction cause.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionProbabilityRefusalV2 {
    /// The fraction had no denominator.
    ZeroDenominator = 0,
    /// The numerator was outside the closed unit interval.
    NumeratorExceedsDenominator = 1,
}

/// Stable Statistics V2 probability role named by a refusal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionStatisticsProbabilityV2 {
    /// Genuine CSCV bottom-half placement probability.
    CscvPbo = 0,
    /// White Reality Check family probability.
    WhiteReality = 1,
    /// Hansen SPA family probability.
    Spa = 2,
    /// D-0460 complete-family maximum/intersection adjusted probability.
    FamilywiseRomanoWolf = 3,
    /// Selected candidate's Romano--Wolf stepdown-adjusted probability.
    CandidateRomanoWolf = 4,
}

/// Stable identity field named by Statistics V2 validation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionStatisticsIdentityV2 {
    /// Exact Candidate semantic identity.
    CandidateSemantic = 0,
    /// Aggregate source-policy identity.
    SourcePolicy = 1,
    /// Complete uncapped finalization family identity.
    FinalizationFamily = 2,
    /// Freshly reopened Statistics V2 audit identity.
    StatisticsAudit = 3,
    /// Statistics completion/receipt identity.
    StatisticsCompletion = 4,
    /// Detached Observation-to-Statistics link identity.
    ObservationStatisticsLink = 5,
    /// CSCV procedure identity.
    CscvPolicy = 6,
    /// Ordered complementary-split family identity.
    CscvSplitFamily = 7,
    /// White return-family identity.
    WhiteFamily = 8,
    /// SPA return-family identity.
    SpaFamily = 9,
    /// Romano--Wolf complete ordered-family identity.
    RomanoWolfFamily = 10,
    /// Exact anchored population-search identity named by finalization.
    PopulationSearch = 11,
    /// Versioned anchored ranking/validation policy identity.
    RankingValidationPolicy = 12,
}

/// Why supplied Statistics V2 fields cannot back Admission V2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionStatisticsRefusalV2 {
    /// A required identity was the all-zero non-identity.
    ZeroIdentity(AdmissionStatisticsIdentityV2),
    /// A stored exact probability was malformed.
    Probability {
        /// Statistical role whose fraction was malformed.
        field: AdmissionStatisticsProbabilityV2,
        /// Exact fraction rule violated.
        kind: AdmissionProbabilityRefusalV2,
    },
    /// A `+1` finite-resample probability had a zero numerator.
    ZeroFiniteResampleNumerator(AdmissionStatisticsProbabilityV2),
    /// A finite-resample denominator differed from `draws + 1`.
    FiniteResampleDenominator {
        /// Statistical role whose denominator differed.
        field: AdmissionStatisticsProbabilityV2,
        /// Required shared denominator.
        expected: u64,
        /// Supplied denominator.
        actual: u64,
    },
    /// `draws + 1` overflowed.
    DrawDenominatorOverflow,
    /// Draw, strategy, or period cardinality was outside the Statistics V2 domain.
    BootstrapShape,
    /// Winning trades exceeded all trades.
    WinningTradesExceedTrades,
    /// Wilson bits were not finite within `[0, 1]` or disagreed with their ppm projection.
    WilsonProjection,
    /// CSCV split counts or the exact PBO fraction contradicted one another.
    CscvCounts,
    /// The selected-candidate adjusted probability was smaller than the
    /// D-0460 complete-family maximum/intersection probability.
    RomanoWolfOrdering,
}

/// Caller-visible detached projection of one Statistics V2 candidate.
///
/// Runner cannot depend on `cli`, so this value retains every identity that a
/// later CLI Finalization join must freshly reopen and revalidate. Supplying or
/// arithmetically verifying this value does **not** prove that disk reopen and
/// cannot create an authoritative admission decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionStatisticsDraftV2 {
    /// Exact Candidate semantic identity.
    pub candidate_semantic_id: [u8; 32],
    /// Exact anchored population-search identity shared by candidates in this family.
    pub population_search_id: [u8; 32],
    /// Versioned anchored ranking/validation policy identity.
    pub ranking_validation_policy_digest: [u8; 32],
    /// Aggregate identity of all source policies used by the statistics family.
    pub source_policy_digest: [u8; 32],
    /// Complete uncapped family used by finalization.
    pub finalization_family_digest: [u8; 32],
    /// Freshly reopened Statistics V2 audit identity.
    pub statistics_audit_id: [u8; 32],
    /// Receipt-last Statistics V2 completion identity.
    pub statistics_completion_digest: [u8; 32],
    /// Detached Observation-to-Statistics authority-link identity.
    pub observation_statistics_link_id: [u8; 32],
    /// Exact CSCV procedure identity.
    pub cscv_policy_digest: [u8; 32],
    /// Complete ordered complementary-split family identity.
    pub cscv_split_family_digest: [u8; 32],
    /// White return-family identity.
    pub white_family_digest: [u8; 32],
    /// SPA return-family identity.
    pub spa_family_digest: [u8; 32],
    /// Romano--Wolf complete ordered-family identity.
    pub romano_wolf_family_digest: [u8; 32],
    /// Exact trade denominator used by Wilson.
    pub trades: u64,
    /// Exact winning-trade numerator used by Wilson.
    pub wins: u64,
    /// Full-precision Wilson lower-bound bits retained by Statistics V2.
    pub wilson_lower_bits: u64,
    /// Canonical floor ppm projection of those exact bits.
    pub wilson_win_rate_ppm: u64,
    /// Canonical complementary split count.
    pub cscv_split_count: u64,
    /// Rankable complementary splits.
    pub pbo_contributing_splits: u64,
    /// Unrankable complementary splits.
    pub pbo_unrankable_splits: u64,
    /// Exact genuine-CSCV PBO probability.
    pub pbo_probability: AdmissionExactProbabilityV2,
    /// Exact White Reality Check probability.
    pub white_probability: AdmissionExactProbabilityV2,
    /// Exact Hansen SPA probability.
    pub spa_probability: AdmissionExactProbabilityV2,
    /// Exact D-0460 complete-family probability.
    pub familywise_romano_wolf_probability: AdmissionExactProbabilityV2,
    /// Exact selected-candidate adjusted Romano--Wolf probability.
    pub candidate_romano_wolf_probability: AdmissionExactProbabilityV2,
    /// Shared deterministic bootstrap draw count.
    pub bootstrap_draws: u64,
    /// Complete compared-strategy count.
    pub bootstrap_strategies: u64,
    /// Aligned return-period count.
    pub bootstrap_periods: u64,
}

/// Validated Statistics V2 fields retained by Admission Evidence V2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionStatisticsFieldsV2 {
    values: AdmissionStatisticsDraftV2,
}

fn validate_statistics_identities_v2(
    values: &AdmissionStatisticsDraftV2,
) -> Result<(), AdmissionStatisticsRefusalV2> {
    let identities = [
        values.candidate_semantic_id,
        values.population_search_id,
        values.ranking_validation_policy_digest,
        values.source_policy_digest,
        values.finalization_family_digest,
        values.statistics_audit_id,
        values.statistics_completion_digest,
        values.observation_statistics_link_id,
        values.cscv_policy_digest,
        values.cscv_split_family_digest,
        values.white_family_digest,
        values.spa_family_digest,
        values.romano_wolf_family_digest,
    ];
    let names = [
        AdmissionStatisticsIdentityV2::CandidateSemantic,
        AdmissionStatisticsIdentityV2::PopulationSearch,
        AdmissionStatisticsIdentityV2::RankingValidationPolicy,
        AdmissionStatisticsIdentityV2::SourcePolicy,
        AdmissionStatisticsIdentityV2::FinalizationFamily,
        AdmissionStatisticsIdentityV2::StatisticsAudit,
        AdmissionStatisticsIdentityV2::StatisticsCompletion,
        AdmissionStatisticsIdentityV2::ObservationStatisticsLink,
        AdmissionStatisticsIdentityV2::CscvPolicy,
        AdmissionStatisticsIdentityV2::CscvSplitFamily,
        AdmissionStatisticsIdentityV2::WhiteFamily,
        AdmissionStatisticsIdentityV2::SpaFamily,
        AdmissionStatisticsIdentityV2::RomanoWolfFamily,
    ];
    debug_assert_eq!(identities.len(), STATISTICS_DIGEST_COUNT_V2);
    for (identity, name) in identities.into_iter().zip(names) {
        if identity == [0; 32] {
            return Err(AdmissionStatisticsRefusalV2::ZeroIdentity(name));
        }
    }
    Ok(())
}

impl AdmissionStatisticsFieldsV2 {
    /// Reconciles a detached Statistics projection for pure arithmetic use.
    ///
    /// # Errors
    ///
    /// Returns the first exact Statistics V2 field contradiction. Success is
    /// deliberately non-authoritative: it proves field reconciliation only,
    /// never that CLI freshly reopened the named receipts or lineage.
    fn verify_detached(
        values: &AdmissionStatisticsDraftV2,
    ) -> Result<Self, AdmissionStatisticsRefusalV2> {
        validate_statistics_identities_v2(values)?;
        if values.wins > values.trades {
            return Err(AdmissionStatisticsRefusalV2::WinningTradesExceedTrades);
        }
        validate_wilson_projection_v2(
            values.wins,
            values.trades,
            values.wilson_lower_bits,
            values.wilson_win_rate_ppm,
        )?;
        if values.bootstrap_draws == 0
            || values.bootstrap_strategies == 0
            || values.bootstrap_periods < 2
        {
            return Err(AdmissionStatisticsRefusalV2::BootstrapShape);
        }
        let denominator = values
            .bootstrap_draws
            .checked_add(1)
            .ok_or(AdmissionStatisticsRefusalV2::DrawDenominatorOverflow)?;
        for (field, probability) in [
            (
                AdmissionStatisticsProbabilityV2::WhiteReality,
                values.white_probability,
            ),
            (
                AdmissionStatisticsProbabilityV2::Spa,
                values.spa_probability,
            ),
            (
                AdmissionStatisticsProbabilityV2::FamilywiseRomanoWolf,
                values.familywise_romano_wolf_probability,
            ),
            (
                AdmissionStatisticsProbabilityV2::CandidateRomanoWolf,
                values.candidate_romano_wolf_probability,
            ),
        ] {
            if probability.numerator == 0 {
                return Err(AdmissionStatisticsRefusalV2::ZeroFiniteResampleNumerator(
                    field,
                ));
            }
            if probability.denominator != denominator {
                return Err(AdmissionStatisticsRefusalV2::FiniteResampleDenominator {
                    field,
                    expected: denominator,
                    actual: probability.denominator,
                });
            }
        }
        if values.pbo_contributing_splits == 0
            || values
                .pbo_contributing_splits
                .checked_add(values.pbo_unrankable_splits)
                != Some(values.cscv_split_count)
            || values.pbo_probability.denominator != values.pbo_contributing_splits
        {
            return Err(AdmissionStatisticsRefusalV2::CscvCounts);
        }
        if values.familywise_romano_wolf_probability.numerator
            > values.candidate_romano_wolf_probability.numerator
        {
            return Err(AdmissionStatisticsRefusalV2::RomanoWolfOrdering);
        }
        Ok(Self { values: *values })
    }

    /// Exact validated field projection.
    #[must_use]
    pub const fn values(self) -> AdmissionStatisticsDraftV2 {
        self.values
    }

    /// Exact Candidate semantic identity shared with walk-forward authority.
    #[must_use]
    pub const fn candidate_semantic_id(self) -> [u8; 32] {
        self.values.candidate_semantic_id
    }

    /// Exact anchored population-search identity shared by the family.
    #[must_use]
    pub const fn population_search_id(self) -> [u8; 32] {
        self.values.population_search_id
    }

    /// Versioned anchored ranking/validation policy identity.
    #[must_use]
    pub const fn ranking_validation_policy_digest(self) -> [u8; 32] {
        self.values.ranking_validation_policy_digest
    }

    /// Aggregate source-policy identity shared with walk-forward authority.
    #[must_use]
    pub const fn source_policy_digest(self) -> [u8; 32] {
        self.values.source_policy_digest
    }

    /// Complete finalization-family identity shared with walk-forward authority.
    #[must_use]
    pub const fn finalization_family_digest(self) -> [u8; 32] {
        self.values.finalization_family_digest
    }

    /// Freshly reopened Statistics V2 audit identity to be checked by CLI finalization.
    #[must_use]
    pub const fn statistics_audit_id(self) -> [u8; 32] {
        self.values.statistics_audit_id
    }
}

fn validate_wilson_projection_v2(
    wins: u64,
    trades: u64,
    bits: u64,
    expected_ppm: u64,
) -> Result<(), AdmissionStatisticsRefusalV2> {
    let (expected_bits, canonical_ppm) = canonical_wilson_projection_v2(wins, trades);
    if bits != expected_bits || expected_ppm != canonical_ppm {
        return Err(AdmissionStatisticsRefusalV2::WilsonProjection);
    }
    Ok(())
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_arithmetic,
    reason = "bit-exact reproduction of the upstream Wilson statistic and its comparison-only ppm projection"
)]
fn canonical_wilson_projection_v2(wins: u64, trades: u64) -> (u64, u64) {
    let value = if trades == 0 {
        0.0
    } else {
        const Z: f64 = 1.959_964;
        let n = trades as f64;
        let p = wins.min(trades) as f64 / n;
        let z2 = Z * Z;
        let denominator = 1.0 + z2 / n;
        let centre = p + z2 / (2.0 * n);
        let margin = Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
        ((centre - margin) / denominator).clamp(0.0, 1.0)
    };
    let ppm = (value * PPM as f64).clamp(0.0, PPM as f64).floor() as u64;
    (value.to_bits(), ppm)
}

/// Stable identity field named by Statistics V3 validation.
///
/// The nine variants are exactly the identities a freshly reopened Statistics
/// authority can project without inventing Search, ranking, source-policy, or
/// Finalization lineage.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionStatisticsIdentityV3 {
    /// Exact Candidate semantic identity.
    CandidateSemantic = 0,
    /// Freshly reopened Statistics V2 audit identity.
    StatisticsAudit = 1,
    /// Receipt-last Statistics V2 completion identity.
    StatisticsCompletion = 2,
    /// Detached Observation-to-Statistics link identity.
    ObservationStatisticsLink = 3,
    /// CSCV procedure identity.
    CscvPolicy = 4,
    /// Ordered complementary-split family identity.
    CscvSplitFamily = 5,
    /// White return-family identity.
    WhiteFamily = 6,
    /// SPA return-family identity.
    SpaFamily = 7,
    /// Romano--Wolf complete ordered-family identity.
    RomanoWolfFamily = 8,
}

/// Why supplied Statistics fields cannot back Admission V3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionStatisticsRefusalV3 {
    /// A required identity was the all-zero non-identity.
    ZeroIdentity(AdmissionStatisticsIdentityV3),
    /// A stored exact probability was malformed.
    Probability {
        /// Statistical role whose fraction was malformed.
        field: AdmissionStatisticsProbabilityV2,
        /// Exact fraction rule violated.
        kind: AdmissionProbabilityRefusalV2,
    },
    /// A `+1` finite-resample probability had a zero numerator.
    ZeroFiniteResampleNumerator(AdmissionStatisticsProbabilityV2),
    /// A finite-resample denominator differed from `draws + 1`.
    FiniteResampleDenominator {
        /// Statistical role whose denominator differed.
        field: AdmissionStatisticsProbabilityV2,
        /// Required shared denominator.
        expected: u64,
        /// Supplied denominator.
        actual: u64,
    },
    /// `draws + 1` overflowed.
    DrawDenominatorOverflow,
    /// Draw, strategy, or period cardinality was outside the Statistics domain.
    BootstrapShape,
    /// Winning trades exceeded all trades.
    WinningTradesExceedTrades,
    /// Wilson bits or their ppm projection differed from exact recomputation.
    WilsonProjection,
    /// CSCV split counts or the exact PBO fraction contradicted one another.
    CscvCounts,
    /// Candidate-adjusted probability was below the complete-family maximum.
    RomanoWolfOrdering,
}

/// Detached projection of the exact Statistics facts Admission V3 consumes.
///
/// This is an arithmetic input, not a durable authority.  A downstream caller
/// must obtain every field from freshly reopened typed Statistics capability;
/// the public fields cannot promote caller-authored bytes into authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionStatisticsDraftV3 {
    /// Exact Candidate semantic identity.
    pub candidate_semantic_id: [u8; 32],
    /// Freshly reopened Statistics audit identity.
    pub statistics_audit_id: [u8; 32],
    /// Receipt-last Statistics completion identity.
    pub statistics_completion_digest: [u8; 32],
    /// Detached Observation-to-Statistics authority-link identity.
    pub observation_statistics_link_id: [u8; 32],
    /// Exact CSCV procedure identity.
    pub cscv_policy_digest: [u8; 32],
    /// Complete ordered complementary-split family identity.
    pub cscv_split_family_digest: [u8; 32],
    /// White return-family identity.
    pub white_family_digest: [u8; 32],
    /// SPA return-family identity.
    pub spa_family_digest: [u8; 32],
    /// Romano--Wolf complete ordered-family identity.
    pub romano_wolf_family_digest: [u8; 32],
    /// Exact trade denominator used by Wilson.
    pub trades: u64,
    /// Exact winning-trade numerator used by Wilson.
    pub wins: u64,
    /// Full-precision Wilson lower-bound bits retained by Statistics.
    pub wilson_lower_bits: u64,
    /// Canonical floor ppm projection of those exact bits.
    pub wilson_win_rate_ppm: u64,
    /// Canonical complementary split count.
    pub cscv_split_count: u64,
    /// Rankable complementary splits.
    pub pbo_contributing_splits: u64,
    /// Unrankable complementary splits.
    pub pbo_unrankable_splits: u64,
    /// Exact genuine-CSCV PBO probability.
    pub pbo_probability: AdmissionExactProbabilityV2,
    /// Exact White Reality Check probability.
    pub white_probability: AdmissionExactProbabilityV2,
    /// Exact Hansen SPA probability.
    pub spa_probability: AdmissionExactProbabilityV2,
    /// Exact complete-family Romano--Wolf probability.
    pub familywise_romano_wolf_probability: AdmissionExactProbabilityV2,
    /// Exact selected-candidate adjusted Romano--Wolf probability.
    pub candidate_romano_wolf_probability: AdmissionExactProbabilityV2,
    /// Shared deterministic bootstrap draw count.
    pub bootstrap_draws: u64,
    /// Complete compared-strategy count.
    pub bootstrap_strategies: u64,
    /// Aligned return-period count.
    pub bootstrap_periods: u64,
}

/// Validated Statistics fields retained by Admission Evidence V3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionStatisticsFieldsV3 {
    values: AdmissionStatisticsDraftV3,
}

impl AdmissionStatisticsFieldsV3 {
    fn verify_detached(
        values: &AdmissionStatisticsDraftV3,
    ) -> Result<Self, AdmissionStatisticsRefusalV3> {
        let identities = [
            values.candidate_semantic_id,
            values.statistics_audit_id,
            values.statistics_completion_digest,
            values.observation_statistics_link_id,
            values.cscv_policy_digest,
            values.cscv_split_family_digest,
            values.white_family_digest,
            values.spa_family_digest,
            values.romano_wolf_family_digest,
        ];
        let names = [
            AdmissionStatisticsIdentityV3::CandidateSemantic,
            AdmissionStatisticsIdentityV3::StatisticsAudit,
            AdmissionStatisticsIdentityV3::StatisticsCompletion,
            AdmissionStatisticsIdentityV3::ObservationStatisticsLink,
            AdmissionStatisticsIdentityV3::CscvPolicy,
            AdmissionStatisticsIdentityV3::CscvSplitFamily,
            AdmissionStatisticsIdentityV3::WhiteFamily,
            AdmissionStatisticsIdentityV3::SpaFamily,
            AdmissionStatisticsIdentityV3::RomanoWolfFamily,
        ];
        debug_assert_eq!(identities.len(), STATISTICS_DIGEST_COUNT_V3);
        for (identity, name) in identities.into_iter().zip(names) {
            if identity == [0; 32] {
                return Err(AdmissionStatisticsRefusalV3::ZeroIdentity(name));
            }
        }
        if values.wins > values.trades {
            return Err(AdmissionStatisticsRefusalV3::WinningTradesExceedTrades);
        }
        let (wilson_bits, wilson_ppm) = canonical_wilson_projection_v2(values.wins, values.trades);
        if values.wilson_lower_bits != wilson_bits || values.wilson_win_rate_ppm != wilson_ppm {
            return Err(AdmissionStatisticsRefusalV3::WilsonProjection);
        }
        if values.bootstrap_draws == 0
            || values.bootstrap_strategies == 0
            || values.bootstrap_periods < 2
        {
            return Err(AdmissionStatisticsRefusalV3::BootstrapShape);
        }
        let denominator = values
            .bootstrap_draws
            .checked_add(1)
            .ok_or(AdmissionStatisticsRefusalV3::DrawDenominatorOverflow)?;
        for (field, probability) in [
            (
                AdmissionStatisticsProbabilityV2::WhiteReality,
                values.white_probability,
            ),
            (
                AdmissionStatisticsProbabilityV2::Spa,
                values.spa_probability,
            ),
            (
                AdmissionStatisticsProbabilityV2::FamilywiseRomanoWolf,
                values.familywise_romano_wolf_probability,
            ),
            (
                AdmissionStatisticsProbabilityV2::CandidateRomanoWolf,
                values.candidate_romano_wolf_probability,
            ),
        ] {
            if probability.numerator == 0 {
                return Err(AdmissionStatisticsRefusalV3::ZeroFiniteResampleNumerator(
                    field,
                ));
            }
            if probability.denominator != denominator {
                return Err(AdmissionStatisticsRefusalV3::FiniteResampleDenominator {
                    field,
                    expected: denominator,
                    actual: probability.denominator,
                });
            }
        }
        if values.pbo_contributing_splits == 0
            || values
                .pbo_contributing_splits
                .checked_add(values.pbo_unrankable_splits)
                != Some(values.cscv_split_count)
            || values.pbo_probability.denominator != values.pbo_contributing_splits
        {
            return Err(AdmissionStatisticsRefusalV3::CscvCounts);
        }
        if values.familywise_romano_wolf_probability.numerator
            > values.candidate_romano_wolf_probability.numerator
        {
            return Err(AdmissionStatisticsRefusalV3::RomanoWolfOrdering);
        }
        Ok(Self { values: *values })
    }

    /// Exact validated field projection.
    #[must_use]
    pub const fn values(self) -> AdmissionStatisticsDraftV3 {
        self.values
    }

    /// Exact Candidate semantic identity carried by Statistics.
    #[must_use]
    pub const fn candidate_semantic_id(self) -> [u8; 32] {
        self.values.candidate_semantic_id
    }

    /// Freshly reopened Statistics audit identity.
    #[must_use]
    pub const fn statistics_audit_id(self) -> [u8; 32] {
        self.values.statistics_audit_id
    }
}

/// Exact reason an opaque anchored walk authority could not be consumed or
/// reopened inside the detached arithmetic kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchoredWalkForwardRefusalV2 {
    /// The supposedly opaque validation object did not reproduce its private
    /// provenance seals.
    OpaqueProvenanceMismatch,
    /// A decoded authority contained an all-zero required identity.
    ZeroIdentity,
    /// Decoded fold/decision/profit counts contradicted their hierarchy.
    CountHierarchy,
    /// Decoded outcome signs contradicted decided/profitable counts.
    OutcomeHierarchy,
    /// The authority ID did not reproduce from its facts and outcomes.
    AuthorityIdentityMismatch,
}

/// Runner-owned anchored validation authority.
///
/// Its policy, family and fact identities are generated inside
/// `validate`; none is copied from detached Statistics. They intentionally do
/// not claim a durable feed, Candidate-Universe, population receipt or CLI
/// finalization identity. Those cross-crate terms remain work for typed CLI
/// Finalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchoredWalkForwardAuthorityV2 {
    validation_policy_digest: [u8; 32],
    validation_family_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
    authority_id: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

impl AnchoredWalkForwardAuthorityV2 {
    fn from_opaque(
        anchored: &crate::validate::AnchoredAdmissionValidationV2,
    ) -> Result<Self, AnchoredWalkForwardRefusalV2> {
        let projection = anchored
            .search_authority_projection()
            .map_err(|_| AnchoredWalkForwardRefusalV2::OpaqueProvenanceMismatch)?;
        Ok(Self {
            validation_policy_digest: projection.policy_identity().digest(),
            validation_family_digest: projection.family_identity().digest(),
            walk_facts_digest: projection.walk_identity().digest(),
            authority_id: [0; 32],
            fold_count: projection.fold_count(),
            decided_folds: projection.decided_folds(),
            profitable_oos_folds: projection.profitable_oos_folds(),
            aggregate_oos_paisa: projection.aggregate_oos_paisa(),
        }
        .with_derived_authority_id())
    }

    #[cfg(test)]
    fn from_canonical_fields(mut value: Self) -> Result<Self, AnchoredWalkForwardRefusalV2> {
        if value.validation_policy_digest == [0; 32]
            || value.validation_family_digest == [0; 32]
            || value.walk_facts_digest == [0; 32]
            || value.authority_id == [0; 32]
        {
            return Err(AnchoredWalkForwardRefusalV2::ZeroIdentity);
        }
        if value.fold_count == 0
            || value.decided_folds > value.fold_count
            || value.profitable_oos_folds > value.decided_folds
        {
            return Err(AnchoredWalkForwardRefusalV2::CountHierarchy);
        }
        if (value.decided_folds == 0 && value.aggregate_oos_paisa != 0)
            || (value.profitable_oos_folds == 0 && value.aggregate_oos_paisa > 0)
            || (value.profitable_oos_folds == value.decided_folds
                && value.decided_folds > 0
                && value.aggregate_oos_paisa <= 0)
        {
            return Err(AnchoredWalkForwardRefusalV2::OutcomeHierarchy);
        }
        let supplied = value.authority_id;
        value.authority_id = [0; 32];
        value = value.with_derived_authority_id();
        if value.authority_id != supplied {
            return Err(AnchoredWalkForwardRefusalV2::AuthorityIdentityMismatch);
        }
        Ok(value)
    }

    fn with_derived_authority_id(mut self) -> Self {
        self.authority_id = derive_walk_forward_authority_id_v2(&self);
        self
    }

    /// Internally derived identity of the exact anchored validation algorithm
    /// parameters used by Runner.
    #[must_use]
    pub const fn validation_policy_digest(self) -> [u8; 32] {
        self.validation_policy_digest
    }

    /// Internally derived identity of exact split windows and ordered candidate
    /// families.
    #[must_use]
    pub const fn validation_family_digest(self) -> [u8; 32] {
        self.validation_family_digest
    }

    /// Identity of the exact retained fold outputs validated by this kernel.
    ///
    /// This is deliberately separate from population and policy lineage:
    /// identical outputs do not prove that two searches used identical inputs,
    /// horizon, split shape, exit rungs, or sweep configuration.
    #[must_use]
    pub const fn walk_facts_digest(self) -> [u8; 32] {
        self.walk_facts_digest
    }

    /// Domain-separated identity over subject, complete facts and derived outcomes.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.authority_id
    }

    /// Ordered anchored fold count.
    #[must_use]
    pub const fn fold_count(self) -> u64 {
        self.fold_count
    }

    /// Folds carrying a validated chosen candidate and complete chosen OOS outcome.
    #[must_use]
    pub const fn decided_folds(self) -> u64 {
        self.decided_folds
    }

    /// Decided folds whose exact chosen OOS exit was strictly positive.
    #[must_use]
    pub const fn profitable_oos_folds(self) -> u64 {
        self.profitable_oos_folds
    }

    /// Checked sum over the same complete chosen OOS exits.
    #[must_use]
    pub const fn aggregate_oos_paisa(self) -> i64 {
        self.aggregate_oos_paisa
    }
}

fn derive_walk_forward_authority_id_v2(value: &AnchoredWalkForwardAuthorityV2) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(WALK_FORWARD_AUTHORITY_DOMAIN_V2);
    for identity in [
        value.validation_policy_digest,
        value.validation_family_digest,
        value.walk_facts_digest,
    ] {
        hasher.update(&identity);
    }
    hasher.update(&value.fold_count.to_le_bytes());
    hasher.update(&value.decided_folds.to_le_bytes());
    hasher.update(&value.profitable_oos_folds.to_le_bytes());
    hasher.update(&value.aggregate_oos_paisa.to_le_bytes());
    hasher.finalize()
}

/// Exact reason an opaque bilateral V3 search authority could not be consumed
/// or reopened inside the detached arithmetic kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchoredWalkForwardRefusalV3 {
    /// The supposedly opaque V3 validation object did not reproduce its
    /// private provenance seals.
    OpaqueProvenanceMismatch,
    /// A decoded authority contained an all-zero required identity.
    ZeroIdentity,
    /// Decoded fold/decision/profit counts contradicted their hierarchy.
    CountHierarchy,
    /// Decoded outcome signs contradicted decided/profitable counts.
    OutcomeHierarchy,
    /// The V3 authority ID did not reproduce from its facts and outcomes.
    AuthorityIdentityMismatch,
}

/// Runner-owned bilateral anchored-search V3 authority.
///
/// Unlike V2, this authority can be derived only from
/// [`crate::validate::AnchoredSearchValidationV3`]. Its policy identity binds
/// the candidate-local side rule, so a direction-bearing V2 validation object
/// cannot be reinterpreted as V3 evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchoredWalkForwardAuthorityV3 {
    validation_policy_digest: [u8; 32],
    validation_family_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
    authority_id: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

impl AnchoredWalkForwardAuthorityV3 {
    fn from_opaque(
        anchored: &crate::validate::AnchoredSearchValidationV3,
    ) -> Result<Self, AnchoredWalkForwardRefusalV3> {
        let projection = anchored
            .search_authority_projection()
            .map_err(|_| AnchoredWalkForwardRefusalV3::OpaqueProvenanceMismatch)?;
        Ok(Self {
            validation_policy_digest: projection.policy_identity().digest(),
            validation_family_digest: projection.family_identity().digest(),
            walk_facts_digest: projection.walk_identity().digest(),
            authority_id: [0; 32],
            fold_count: projection.fold_count(),
            decided_folds: projection.decided_folds(),
            profitable_oos_folds: projection.profitable_oos_folds(),
            aggregate_oos_paisa: projection.aggregate_oos_paisa(),
        }
        .with_derived_authority_id())
    }

    /// Projects the exact-grid Search V4 object into Admission's fixed V3
    /// arithmetic fields.
    ///
    /// The returned arithmetic authority deliberately carries only the policy,
    /// family, walk and aggregate terms consumed by the immutable admission
    /// comparison. Search V4's typed Long/Short grid components and evaluated
    /// population count remain outside these bytes and must be equality-joined
    /// by the durable CLI Population-Admission source. Consequently this helper
    /// cannot by itself promote a Search result or authorize execution.
    fn from_exact_grid_opaque(
        anchored: &crate::validate::AnchoredSearchValidationV4,
    ) -> Result<Self, AnchoredWalkForwardRefusalV3> {
        let projection = anchored
            .search_authority_projection()
            .map_err(|_| AnchoredWalkForwardRefusalV3::OpaqueProvenanceMismatch)?;
        Ok(Self {
            validation_policy_digest: projection.policy_identity().digest(),
            validation_family_digest: projection.family_identity().digest(),
            walk_facts_digest: projection.walk_identity().digest(),
            authority_id: [0; 32],
            fold_count: projection.fold_count(),
            decided_folds: projection.decided_folds(),
            profitable_oos_folds: projection.profitable_oos_folds(),
            aggregate_oos_paisa: projection.aggregate_oos_paisa(),
        }
        .with_derived_authority_id())
    }

    fn from_canonical_fields(mut value: Self) -> Result<Self, AnchoredWalkForwardRefusalV3> {
        if value.validation_policy_digest == [0; 32]
            || value.validation_family_digest == [0; 32]
            || value.walk_facts_digest == [0; 32]
            || value.authority_id == [0; 32]
        {
            return Err(AnchoredWalkForwardRefusalV3::ZeroIdentity);
        }
        if value.fold_count == 0
            || value.decided_folds > value.fold_count
            || value.profitable_oos_folds > value.decided_folds
        {
            return Err(AnchoredWalkForwardRefusalV3::CountHierarchy);
        }
        if (value.decided_folds == 0 && value.aggregate_oos_paisa != 0)
            || (value.profitable_oos_folds == 0 && value.aggregate_oos_paisa > 0)
            || (value.profitable_oos_folds == value.decided_folds
                && value.decided_folds > 0
                && value.aggregate_oos_paisa <= 0)
        {
            return Err(AnchoredWalkForwardRefusalV3::OutcomeHierarchy);
        }
        let supplied = value.authority_id;
        value.authority_id = [0; 32];
        value = value.with_derived_authority_id();
        if value.authority_id != supplied {
            return Err(AnchoredWalkForwardRefusalV3::AuthorityIdentityMismatch);
        }
        Ok(value)
    }

    fn with_derived_authority_id(mut self) -> Self {
        self.authority_id = derive_walk_forward_authority_id_v3(&self);
        self
    }

    /// Internally derived identity of the exact bilateral V3 validation policy.
    #[must_use]
    pub const fn validation_policy_digest(self) -> [u8; 32] {
        self.validation_policy_digest
    }

    /// Internally derived identity of exact split windows and ordered candidate
    /// families.
    #[must_use]
    pub const fn validation_family_digest(self) -> [u8; 32] {
        self.validation_family_digest
    }

    /// Identity of the exact retained V3 fold outputs validated by Runner.
    #[must_use]
    pub const fn walk_facts_digest(self) -> [u8; 32] {
        self.walk_facts_digest
    }

    /// V3-domain-separated identity over the complete authority facts.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.authority_id
    }

    /// Ordered anchored fold count.
    #[must_use]
    pub const fn fold_count(self) -> u64 {
        self.fold_count
    }

    /// Folds carrying a revalidated chosen candidate and complete OOS outcome.
    #[must_use]
    pub const fn decided_folds(self) -> u64 {
        self.decided_folds
    }

    /// Decided folds whose exact chosen OOS exit was strictly positive.
    #[must_use]
    pub const fn profitable_oos_folds(self) -> u64 {
        self.profitable_oos_folds
    }

    /// Checked sum over the same complete chosen OOS exits.
    #[must_use]
    pub const fn aggregate_oos_paisa(self) -> i64 {
        self.aggregate_oos_paisa
    }
}

fn derive_walk_forward_authority_id_v3(value: &AnchoredWalkForwardAuthorityV3) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(WALK_FORWARD_AUTHORITY_DOMAIN_V3);
    for identity in [
        value.validation_policy_digest,
        value.validation_family_digest,
        value.walk_facts_digest,
    ] {
        hasher.update(&identity);
    }
    hasher.update(&value.fold_count.to_le_bytes());
    hasher.update(&value.decided_folds.to_le_bytes());
    hasher.update(&value.profitable_oos_folds.to_le_bytes());
    hasher.update(&value.aggregate_oos_paisa.to_le_bytes());
    hasher.finalize()
}

/// Field whose value must come from one of Admission V2's two authorities.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionAuthorityOwnedFieldV2 {
    /// Statistics V2 Wilson lower bound.
    WilsonWinRate = 0,
    /// Statistics V2 genuine CSCV/PBO probability.
    Pbo = 1,
    /// D-0460 complete-family Romano--Wolf probability.
    Fwer = 2,
    /// Statistics V2 Hansen SPA probability.
    Spa = 3,
    /// Anchored walk-forward decided count.
    DecidedFolds = 4,
    /// Statistics V2 deterministic resample count.
    BootstrapDraws = 5,
    /// Statistics V2 complete family size.
    BootstrapStrategies = 6,
    /// Statistics V2 aligned period count.
    BootstrapPeriods = 7,
    /// Statistics V2 rankable complementary splits.
    PboContributingSplits = 8,
    /// Statistics V2 unrankable complementary splits.
    PboUnrankableSplits = 9,
    /// Anchored walk-forward profitable chosen OOS count.
    ProfitableOosFolds = 10,
    /// Anchored walk-forward checked chosen OOS sum.
    OosPessimisticReturn = 11,
    /// Statistics V2 White Reality Check probability.
    WhiteRealityPValue = 12,
    /// Statistics V2 candidate-adjusted Romano--Wolf probability.
    CandidateRomanoWolfPValue = 13,
    /// Exact White 5% decision.
    WhiteRealityDecision = 14,
    /// Exact candidate Romano--Wolf 5% decision.
    CandidateRomanoWolfDecision = 15,
    /// Both complete exact authority records were supplied.
    FullPrecisionStatisticsCompleteness = 16,
}

/// Why two otherwise validated V2 authorities could not form evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionEvidenceRefusalV2 {
    /// Base evidence attempted to pre-fill a field owned by a V2 authority.
    AuthorityFieldAlreadySet(AdmissionAuthorityOwnedFieldV2),
    /// Base candidate trade count did not exactly match Statistics V2.
    TradeCountSourceMismatch,
    /// Base candidate winning-trade count did not exactly match Statistics V2.
    WinningTradeCountSourceMismatch,
    /// The completed fixed comparison values violated a V1-stable physical or reconciliation rule.
    Comparison(AdmissionEvidenceRefusalV1),
}

/// Exact reason the public non-authoritative Admission V2 arithmetic verifier
/// could not produce a comparison projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionV2ArithmeticRefusal {
    /// The detached Statistics projection was internally inconsistent.
    Statistics(AdmissionStatisticsRefusalV2),
    /// The anchored walk-forward facts could not be derived or reconciled.
    WalkForward(AnchoredWalkForwardRefusalV2),
    /// The two detached arithmetic sources could not be joined.
    Evidence(AdmissionEvidenceRefusalV2),
}

/// Admission Evidence V2 retaining both independent authorities and their
/// fixed comparison projection.
///
/// The base values must leave all authority-owned fields explicitly
/// `Unmeasured`; a refusal or numeric value cannot be overwritten. Candidate
/// trade/win counts must already be measured and reconcile byte-exactly with
/// Statistics V2, so statistics cannot silently replace a different candidate
/// source. Runner does not pretend the Statistics lineage terms independently
/// prove the walk's internally derived policy/family terms: both are retained
/// for a later typed CLI Finalization join. Anchored walk-forward outcomes
/// remain shared search diagnostics; they are not relabelled as outcomes of the
/// one candidate named by Statistics V2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionEvidenceV2 {
    values: AdmissionEvidenceValuesV1,
    statistics: AdmissionStatisticsFieldsV2,
    walk_forward: AnchoredWalkForwardAuthorityV2,
}

impl AdmissionEvidenceV2 {
    /// Joins Statistics V2 and anchored walk-forward facts without permitting
    /// either authority to supply the other's fields.
    ///
    /// # Errors
    ///
    /// Refuses a pre-filled authority slot, candidate count mismatch, or any
    /// completed comparison contradiction.
    fn new(
        mut base: AdmissionEvidenceValuesV1,
        statistics: &AdmissionStatisticsFieldsV2,
        walk_forward: AnchoredWalkForwardAuthorityV2,
    ) -> Result<Self, AdmissionEvidenceRefusalV2> {
        require_authority_slots_unmeasured_v2(&base)?;
        let statistics_values = statistics.values();
        if base.trades != ObservedU64V1::Measured(statistics_values.trades) {
            return Err(AdmissionEvidenceRefusalV2::TradeCountSourceMismatch);
        }
        if base.winning_trades != ObservedU64V1::Measured(statistics_values.wins) {
            return Err(AdmissionEvidenceRefusalV2::WinningTradeCountSourceMismatch);
        }
        base.wilson_win_rate_ppm = ObservedU64V1::Measured(statistics_values.wilson_win_rate_ppm);
        base.pbo_ppm = ObservedU64V1::Measured(statistics_values.pbo_probability.ppm());
        base.fwer_p_value_ppm =
            ObservedU64V1::Measured(statistics_values.familywise_romano_wolf_probability.ppm());
        base.spa_p_value_ppm = ObservedU64V1::Measured(statistics_values.spa_probability.ppm());
        base.decided_folds = ObservedU64V1::Measured(walk_forward.decided_folds());
        base.bootstrap_draws = ObservedU64V1::Measured(statistics_values.bootstrap_draws);
        base.bootstrap_strategies = ObservedU64V1::Measured(statistics_values.bootstrap_strategies);
        base.bootstrap_periods = ObservedU64V1::Measured(statistics_values.bootstrap_periods);
        base.pbo_contributing_folds =
            ObservedU64V1::Measured(statistics_values.pbo_contributing_splits);
        base.pbo_unrankable_folds =
            ObservedU64V1::Measured(statistics_values.pbo_unrankable_splits);
        base.profitable_oos_folds = ObservedU64V1::Measured(walk_forward.profitable_oos_folds());
        base.oos_pessimistic_return_paisa =
            ObservedI64V1::Measured(walk_forward.aggregate_oos_paisa());
        base.white_reality_p_value_ppm =
            ObservedU64V1::Measured(statistics_values.white_probability.ppm());
        base.romano_wolf_p_value_ppm =
            ObservedU64V1::Measured(statistics_values.candidate_romano_wolf_probability.ppm());
        base.white_reality_decision = hypothesis_decision_v2(statistics_values.white_probability);
        base.romano_wolf_decision =
            hypothesis_decision_v2(statistics_values.candidate_romano_wolf_probability);
        base.full_precision_statistics_complete = CompletenessV1::Complete;
        validate_evidence_values_v2(&base).map_err(AdmissionEvidenceRefusalV2::Comparison)?;
        Ok(Self {
            values: base,
            statistics: *statistics,
            walk_forward,
        })
    }

    /// Exact fixed comparison values after both sources were joined.
    #[must_use]
    pub const fn values(self) -> AdmissionEvidenceValuesV1 {
        self.values
    }

    /// Exact Statistics V2 authority fields.
    #[must_use]
    pub const fn statistics(self) -> AdmissionStatisticsFieldsV2 {
        self.statistics
    }

    /// Exact independently derived anchored walk-forward authority.
    #[must_use]
    pub const fn walk_forward(self) -> AnchoredWalkForwardAuthorityV2 {
        self.walk_forward
    }

    /// Canonical fixed-size V2 bytes retaining all comparison and authority fields.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V2] {
        canonical_evidence_bytes_v2(&self)
    }

    /// Decodes all V2 fields, revalidates each authority, re-runs the join and
    /// requires byte-exact reconstruction.
    ///
    /// # Errors
    ///
    /// Refuses a malformed header/tag/fraction, inconsistent authority, foreign
    /// subject, comparison contradiction, or any value not reproduced by the
    /// two decoded authority records.
    #[cfg(test)]
    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV2> {
        decode_evidence_v2(bytes)
    }

    /// Domain-separated BLAKE3 identity of the exact V2 evidence record.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }
}

fn hypothesis_decision_v2(probability: AdmissionExactProbabilityV2) -> HypothesisDecisionV1 {
    if probability.rejects_at_ppm(CANONICAL_FAMILY_ALPHA_PPM_V2) {
        HypothesisDecisionV1::RejectedNull
    } else {
        HypothesisDecisionV1::DidNotReject
    }
}

fn require_authority_slots_unmeasured_v2(
    values: &AdmissionEvidenceValuesV1,
) -> Result<(), AdmissionEvidenceRefusalV2> {
    for (value, field) in [
        (
            values.wilson_win_rate_ppm,
            AdmissionAuthorityOwnedFieldV2::WilsonWinRate,
        ),
        (values.pbo_ppm, AdmissionAuthorityOwnedFieldV2::Pbo),
        (
            values.fwer_p_value_ppm,
            AdmissionAuthorityOwnedFieldV2::Fwer,
        ),
        (values.spa_p_value_ppm, AdmissionAuthorityOwnedFieldV2::Spa),
        (
            values.decided_folds,
            AdmissionAuthorityOwnedFieldV2::DecidedFolds,
        ),
        (
            values.bootstrap_draws,
            AdmissionAuthorityOwnedFieldV2::BootstrapDraws,
        ),
        (
            values.bootstrap_strategies,
            AdmissionAuthorityOwnedFieldV2::BootstrapStrategies,
        ),
        (
            values.bootstrap_periods,
            AdmissionAuthorityOwnedFieldV2::BootstrapPeriods,
        ),
        (
            values.pbo_contributing_folds,
            AdmissionAuthorityOwnedFieldV2::PboContributingSplits,
        ),
        (
            values.pbo_unrankable_folds,
            AdmissionAuthorityOwnedFieldV2::PboUnrankableSplits,
        ),
        (
            values.profitable_oos_folds,
            AdmissionAuthorityOwnedFieldV2::ProfitableOosFolds,
        ),
        (
            values.white_reality_p_value_ppm,
            AdmissionAuthorityOwnedFieldV2::WhiteRealityPValue,
        ),
        (
            values.romano_wolf_p_value_ppm,
            AdmissionAuthorityOwnedFieldV2::CandidateRomanoWolfPValue,
        ),
    ] {
        if value != ObservedU64V1::Unmeasured {
            return Err(AdmissionEvidenceRefusalV2::AuthorityFieldAlreadySet(field));
        }
    }
    if values.oos_pessimistic_return_paisa != ObservedI64V1::Unmeasured {
        return Err(AdmissionEvidenceRefusalV2::AuthorityFieldAlreadySet(
            AdmissionAuthorityOwnedFieldV2::OosPessimisticReturn,
        ));
    }
    for (value, field) in [
        (
            values.white_reality_decision,
            AdmissionAuthorityOwnedFieldV2::WhiteRealityDecision,
        ),
        (
            values.romano_wolf_decision,
            AdmissionAuthorityOwnedFieldV2::CandidateRomanoWolfDecision,
        ),
    ] {
        if value != HypothesisDecisionV1::Unmeasured {
            return Err(AdmissionEvidenceRefusalV2::AuthorityFieldAlreadySet(field));
        }
    }
    if values.full_precision_statistics_complete != CompletenessV1::Unmeasured {
        return Err(AdmissionEvidenceRefusalV2::AuthorityFieldAlreadySet(
            AdmissionAuthorityOwnedFieldV2::FullPrecisionStatisticsCompleteness,
        ));
    }
    Ok(())
}

fn clear_authority_slots_v2(values: &mut AdmissionEvidenceValuesV1) {
    values.wilson_win_rate_ppm = ObservedU64V1::Unmeasured;
    values.pbo_ppm = ObservedU64V1::Unmeasured;
    values.fwer_p_value_ppm = ObservedU64V1::Unmeasured;
    values.spa_p_value_ppm = ObservedU64V1::Unmeasured;
    values.decided_folds = ObservedU64V1::Unmeasured;
    values.bootstrap_draws = ObservedU64V1::Unmeasured;
    values.bootstrap_strategies = ObservedU64V1::Unmeasured;
    values.bootstrap_periods = ObservedU64V1::Unmeasured;
    values.pbo_contributing_folds = ObservedU64V1::Unmeasured;
    values.pbo_unrankable_folds = ObservedU64V1::Unmeasured;
    values.profitable_oos_folds = ObservedU64V1::Unmeasured;
    values.oos_pessimistic_return_paisa = ObservedI64V1::Unmeasured;
    values.white_reality_p_value_ppm = ObservedU64V1::Unmeasured;
    values.romano_wolf_p_value_ppm = ObservedU64V1::Unmeasured;
    values.white_reality_decision = HypothesisDecisionV1::Unmeasured;
    values.romano_wolf_decision = HypothesisDecisionV1::Unmeasured;
    values.full_precision_statistics_complete = CompletenessV1::Unmeasured;
}

fn validate_evidence_values_v2(
    values: &AdmissionEvidenceValuesV1,
) -> Result<(), AdmissionEvidenceRefusalV1> {
    let mut v1_compatible = *values;
    v1_compatible.pbo_ppm = ObservedU64V1::Unmeasured;
    v1_compatible.pbo_contributing_folds = ObservedU64V1::Unmeasured;
    v1_compatible.pbo_unrankable_folds = ObservedU64V1::Unmeasured;
    AdmissionEvidenceV1::new(v1_compatible)?;
    validate_observed_ppm(values.pbo_ppm, AdmissionEvidenceFieldV1::Pbo)?;
    reconcile_measured_subsets(values)
}

/// Field whose value must come from one of Admission V3's two authorities.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionAuthorityOwnedFieldV3 {
    /// Statistics Wilson lower bound.
    WilsonWinRate = 0,
    /// Statistics genuine CSCV/PBO probability.
    Pbo = 1,
    /// Complete-family Romano--Wolf probability.
    Fwer = 2,
    /// Hansen SPA probability.
    Spa = 3,
    /// Anchored walk-forward decided count.
    DecidedFolds = 4,
    /// Deterministic resample count.
    BootstrapDraws = 5,
    /// Complete family size.
    BootstrapStrategies = 6,
    /// Aligned period count.
    BootstrapPeriods = 7,
    /// Rankable complementary splits.
    PboContributingSplits = 8,
    /// Unrankable complementary splits.
    PboUnrankableSplits = 9,
    /// Profitable chosen OOS count.
    ProfitableOosFolds = 10,
    /// Checked chosen OOS sum.
    OosPessimisticReturn = 11,
    /// White Reality Check probability.
    WhiteRealityPValue = 12,
    /// Candidate-adjusted Romano--Wolf probability.
    CandidateRomanoWolfPValue = 13,
    /// Exact White 5% decision.
    WhiteRealityDecision = 14,
    /// Exact candidate Romano--Wolf 5% decision.
    CandidateRomanoWolfDecision = 15,
    /// Both exact authority records were supplied.
    FullPrecisionStatisticsCompleteness = 16,
}

/// Why the validated V3 authorities could not form evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionEvidenceRefusalV3 {
    /// Base evidence attempted to pre-fill an authority-owned field.
    AuthorityFieldAlreadySet(AdmissionAuthorityOwnedFieldV3),
    /// Base candidate trade count did not exactly match Statistics.
    TradeCountSourceMismatch,
    /// Base candidate winning-trade count did not exactly match Statistics.
    WinningTradeCountSourceMismatch,
    /// Completed comparison values violated a V1-stable rule.
    Comparison(AdmissionEvidenceRefusalV1),
}

/// Exact reason detached Admission V3 arithmetic could not be projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionV3ArithmeticRefusal {
    /// The detached Statistics projection was internally inconsistent.
    Statistics(AdmissionStatisticsRefusalV3),
    /// Anchored walk facts could not be derived from the opaque object.
    WalkForward(AnchoredWalkForwardRefusalV3),
    /// The two arithmetic sources could not be joined.
    Evidence(AdmissionEvidenceRefusalV3),
}

/// Admission Evidence V3 with only provable Statistics identities and the
/// independently derived anchored walk authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionEvidenceV3 {
    values: AdmissionEvidenceValuesV1,
    statistics: AdmissionStatisticsFieldsV3,
    walk_forward: AnchoredWalkForwardAuthorityV3,
}

impl AdmissionEvidenceV3 {
    fn new(
        mut base: AdmissionEvidenceValuesV1,
        statistics: &AdmissionStatisticsFieldsV3,
        walk_forward: AnchoredWalkForwardAuthorityV3,
    ) -> Result<Self, AdmissionEvidenceRefusalV3> {
        require_authority_slots_unmeasured_v3(&base)?;
        let statistics_values = statistics.values();
        if base.trades != ObservedU64V1::Measured(statistics_values.trades) {
            return Err(AdmissionEvidenceRefusalV3::TradeCountSourceMismatch);
        }
        if base.winning_trades != ObservedU64V1::Measured(statistics_values.wins) {
            return Err(AdmissionEvidenceRefusalV3::WinningTradeCountSourceMismatch);
        }
        base.wilson_win_rate_ppm = ObservedU64V1::Measured(statistics_values.wilson_win_rate_ppm);
        base.pbo_ppm = ObservedU64V1::Measured(statistics_values.pbo_probability.ppm());
        base.fwer_p_value_ppm =
            ObservedU64V1::Measured(statistics_values.familywise_romano_wolf_probability.ppm());
        base.spa_p_value_ppm = ObservedU64V1::Measured(statistics_values.spa_probability.ppm());
        base.decided_folds = ObservedU64V1::Measured(walk_forward.decided_folds());
        base.bootstrap_draws = ObservedU64V1::Measured(statistics_values.bootstrap_draws);
        base.bootstrap_strategies = ObservedU64V1::Measured(statistics_values.bootstrap_strategies);
        base.bootstrap_periods = ObservedU64V1::Measured(statistics_values.bootstrap_periods);
        base.pbo_contributing_folds =
            ObservedU64V1::Measured(statistics_values.pbo_contributing_splits);
        base.pbo_unrankable_folds =
            ObservedU64V1::Measured(statistics_values.pbo_unrankable_splits);
        base.profitable_oos_folds = ObservedU64V1::Measured(walk_forward.profitable_oos_folds());
        base.oos_pessimistic_return_paisa =
            ObservedI64V1::Measured(walk_forward.aggregate_oos_paisa());
        base.white_reality_p_value_ppm =
            ObservedU64V1::Measured(statistics_values.white_probability.ppm());
        base.romano_wolf_p_value_ppm =
            ObservedU64V1::Measured(statistics_values.candidate_romano_wolf_probability.ppm());
        base.white_reality_decision = hypothesis_decision_v2(statistics_values.white_probability);
        base.romano_wolf_decision =
            hypothesis_decision_v2(statistics_values.candidate_romano_wolf_probability);
        base.full_precision_statistics_complete = CompletenessV1::Complete;
        validate_evidence_values_v2(&base).map_err(AdmissionEvidenceRefusalV3::Comparison)?;
        Ok(Self {
            values: base,
            statistics: *statistics,
            walk_forward,
        })
    }

    /// Exact fixed comparison values after joining both sources.
    #[must_use]
    pub const fn values(self) -> AdmissionEvidenceValuesV1 {
        self.values
    }

    /// Exact validated Statistics V3 fields.
    #[must_use]
    pub const fn statistics(self) -> AdmissionStatisticsFieldsV3 {
        self.statistics
    }

    /// Exact independently derived anchored walk authority.
    #[must_use]
    pub const fn walk_forward(self) -> AnchoredWalkForwardAuthorityV3 {
        self.walk_forward
    }

    /// Canonical fixed-size V3 bytes.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V3] {
        canonical_evidence_bytes_v3(&self)
    }

    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV3> {
        decode_evidence_v3(bytes)
    }

    /// Domain-separated identity of the exact V3 evidence record.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }
}

fn require_authority_slots_unmeasured_v3(
    values: &AdmissionEvidenceValuesV1,
) -> Result<(), AdmissionEvidenceRefusalV3> {
    for (value, field) in [
        (
            values.wilson_win_rate_ppm,
            AdmissionAuthorityOwnedFieldV3::WilsonWinRate,
        ),
        (values.pbo_ppm, AdmissionAuthorityOwnedFieldV3::Pbo),
        (
            values.fwer_p_value_ppm,
            AdmissionAuthorityOwnedFieldV3::Fwer,
        ),
        (values.spa_p_value_ppm, AdmissionAuthorityOwnedFieldV3::Spa),
        (
            values.decided_folds,
            AdmissionAuthorityOwnedFieldV3::DecidedFolds,
        ),
        (
            values.bootstrap_draws,
            AdmissionAuthorityOwnedFieldV3::BootstrapDraws,
        ),
        (
            values.bootstrap_strategies,
            AdmissionAuthorityOwnedFieldV3::BootstrapStrategies,
        ),
        (
            values.bootstrap_periods,
            AdmissionAuthorityOwnedFieldV3::BootstrapPeriods,
        ),
        (
            values.pbo_contributing_folds,
            AdmissionAuthorityOwnedFieldV3::PboContributingSplits,
        ),
        (
            values.pbo_unrankable_folds,
            AdmissionAuthorityOwnedFieldV3::PboUnrankableSplits,
        ),
        (
            values.profitable_oos_folds,
            AdmissionAuthorityOwnedFieldV3::ProfitableOosFolds,
        ),
        (
            values.white_reality_p_value_ppm,
            AdmissionAuthorityOwnedFieldV3::WhiteRealityPValue,
        ),
        (
            values.romano_wolf_p_value_ppm,
            AdmissionAuthorityOwnedFieldV3::CandidateRomanoWolfPValue,
        ),
    ] {
        if value != ObservedU64V1::Unmeasured {
            return Err(AdmissionEvidenceRefusalV3::AuthorityFieldAlreadySet(field));
        }
    }
    if values.oos_pessimistic_return_paisa != ObservedI64V1::Unmeasured {
        return Err(AdmissionEvidenceRefusalV3::AuthorityFieldAlreadySet(
            AdmissionAuthorityOwnedFieldV3::OosPessimisticReturn,
        ));
    }
    for (value, field) in [
        (
            values.white_reality_decision,
            AdmissionAuthorityOwnedFieldV3::WhiteRealityDecision,
        ),
        (
            values.romano_wolf_decision,
            AdmissionAuthorityOwnedFieldV3::CandidateRomanoWolfDecision,
        ),
    ] {
        if value != HypothesisDecisionV1::Unmeasured {
            return Err(AdmissionEvidenceRefusalV3::AuthorityFieldAlreadySet(field));
        }
    }
    if values.full_precision_statistics_complete != CompletenessV1::Unmeasured {
        return Err(AdmissionEvidenceRefusalV3::AuthorityFieldAlreadySet(
            AdmissionAuthorityOwnedFieldV3::FullPrecisionStatisticsCompleteness,
        ));
    }
    Ok(())
}

/// Admission V2 canonical record named by a decode refusal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionCanonicalRecordV2 {
    /// Two-authority evidence record.
    Evidence = EVIDENCE_DOMAIN_V1,
    /// Policy/evidence/verdict decision record.
    Decision = DECISION_DOMAIN_V1,
}

/// Exact reason canonical Admission V2 bytes were refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionCanonicalRefusalV2 {
    /// Complete record length differed from the fixed V2 length.
    Length {
        /// Record whose length differed.
        record: AdmissionCanonicalRecordV2,
        /// Exact V2 length.
        expected: usize,
        /// Supplied length.
        actual: usize,
    },
    /// Canonical magic differed.
    Magic {
        /// Record whose magic differed.
        record: AdmissionCanonicalRecordV2,
    },
    /// Canonical domain differed.
    Domain {
        /// Decoder receiving the bytes.
        record: AdmissionCanonicalRecordV2,
        /// Supplied domain.
        actual: u8,
    },
    /// A reserved byte or payload was nonzero.
    Reserved {
        /// Record containing the byte.
        record: AdmissionCanonicalRecordV2,
        /// Absolute byte offset.
        offset: usize,
    },
    /// Canonical record version was not V2.
    Version {
        /// Record carrying the version.
        record: AdmissionCanonicalRecordV2,
        /// Supplied version.
        actual: u16,
    },
    /// Header payload length differed from the fixed V2 layout.
    PayloadLength {
        /// Record carrying the declaration.
        record: AdmissionCanonicalRecordV2,
        /// Supplied declaration.
        actual: u32,
    },
    /// An observed/completeness/decision tag was outside its stable domain.
    Tag {
        /// Record carrying the tag.
        record: AdmissionCanonicalRecordV2,
        /// Absolute byte offset.
        offset: usize,
        /// Supplied tag.
        actual: u8,
    },
    /// A Statistics V2 field set was internally inconsistent.
    Statistics(AdmissionStatisticsRefusalV2),
    /// Anchored walk-forward fields were internally inconsistent.
    WalkForward(AnchoredWalkForwardRefusalV2),
    /// The two-authority evidence join refused.
    Evidence(AdmissionEvidenceRefusalV2),
    /// A nested immutable V1 policy or verdict record refused.
    NestedV1(AdmissionCanonicalRefusalV1),
    /// Decoded comparison bytes were not exactly reproduced from authorities.
    AuthorityProjectionMismatch,
    /// A supplied valid V1 verdict differed from V2 recomputation.
    VerdictMismatch,
}

/// Admission V3 canonical record named by a decode refusal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionCanonicalRecordV3 {
    /// Two-authority evidence record.
    Evidence = EVIDENCE_DOMAIN_V1,
    /// Policy/evidence/verdict decision record.
    Decision = DECISION_DOMAIN_V1,
}

/// Exact reason canonical Admission V3 bytes were refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionCanonicalRefusalV3 {
    /// Complete record length differed from the fixed V3 length.
    Length {
        /// Record whose length differed.
        record: AdmissionCanonicalRecordV3,
        /// Exact V3 length.
        expected: usize,
        /// Supplied length.
        actual: usize,
    },
    /// Canonical magic differed.
    Magic {
        /// Record whose magic differed.
        record: AdmissionCanonicalRecordV3,
    },
    /// Canonical domain differed.
    Domain {
        /// Decoder receiving the bytes.
        record: AdmissionCanonicalRecordV3,
        /// Supplied domain.
        actual: u8,
    },
    /// A reserved byte or payload was nonzero.
    Reserved {
        /// Record containing the byte.
        record: AdmissionCanonicalRecordV3,
        /// Absolute byte offset.
        offset: usize,
    },
    /// Canonical record version was not V3.
    Version {
        /// Record carrying the version.
        record: AdmissionCanonicalRecordV3,
        /// Supplied version.
        actual: u16,
    },
    /// Header payload length differed from the fixed V3 layout.
    PayloadLength {
        /// Record carrying the declaration.
        record: AdmissionCanonicalRecordV3,
        /// Supplied declaration.
        actual: u32,
    },
    /// An observed/completeness/decision tag was outside its stable domain.
    Tag {
        /// Record carrying the tag.
        record: AdmissionCanonicalRecordV3,
        /// Absolute byte offset.
        offset: usize,
        /// Supplied tag.
        actual: u8,
    },
    /// A Statistics V3 field set was internally inconsistent.
    Statistics(AdmissionStatisticsRefusalV3),
    /// Anchored walk-forward fields were internally inconsistent.
    WalkForward(AnchoredWalkForwardRefusalV3),
    /// The two-authority evidence join refused.
    Evidence(AdmissionEvidenceRefusalV3),
    /// A nested immutable V1 policy or verdict record refused.
    NestedV1(AdmissionCanonicalRefusalV1),
    /// Decoded comparison bytes were not exactly reproduced from authorities.
    AuthorityProjectionMismatch,
    /// A supplied valid V1 verdict differed from V3 recomputation.
    VerdictMismatch,
}

impl AdmissionPolicyV1 {
    /// Verifies detached V2 arithmetic and returns comparison/verdict bytes.
    ///
    /// This is the pure Runner surface for a future CLI Finalization wrapper:
    /// it validates supplied exact fractions, recomputes Wilson, consumes only
    /// the opaque anchored validation result, and evaluates the unchanged V1
    /// policy. It does **not** reopen any receipt, prove Candidate-Universe
    /// lineage, or create a durable/admissible seal. CLI must bind the retained
    /// Statistics lineage to the internally derived validation policy/family
    /// terms before presenting the arithmetic as authoritative.
    ///
    /// # Errors
    ///
    /// Returns the first Statistics, walk-forward, or evidence-join refusal.
    pub fn evaluate_v2_projection(
        self,
        base: &AdmissionEvidenceValuesV1,
        statistics: &AdmissionStatisticsDraftV2,
        anchored: &crate::validate::AnchoredAdmissionValidationV2,
    ) -> Result<AdmissionV2ArithmeticProjection, AdmissionV2ArithmeticRefusal> {
        let fields = AdmissionStatisticsFieldsV2::verify_detached(statistics)
            .map_err(AdmissionV2ArithmeticRefusal::Statistics)?;
        let walk_forward = AnchoredWalkForwardAuthorityV2::from_opaque(anchored)
            .map_err(AdmissionV2ArithmeticRefusal::WalkForward)?;
        let evidence = AdmissionEvidenceV2::new(*base, &fields, walk_forward)
            .map_err(AdmissionV2ArithmeticRefusal::Evidence)?;
        Ok(AdmissionV2ArithmeticProjection::from_decision(
            &self.evaluate_v2_record(&evidence),
        ))
    }

    /// Evaluates the unchanged fixed policy inside the detached Admission V2 kernel.
    ///
    /// This is crate-private because only a future CLI Finalization authority
    /// may promote freshly reopened cross-crate capabilities into an operator
    /// decision. Self-declared detached fields are not production authority.
    #[must_use]
    pub(crate) fn evaluate_v2(self, evidence: &AdmissionEvidenceV2) -> AdmissionVerdictV1 {
        // Same-module construction is deliberate: public V1 construction still
        // refuses measured PBO fields, while V2 has already validated the two
        // independent authorities that make these values reachable.
        self.evaluate(&AdmissionEvidenceV1 {
            values: evidence.values,
        })
    }

    /// Evaluates one exact V1 policy into a detached V2 kernel record.
    #[must_use]
    fn evaluate_v2_record(self, evidence: &AdmissionEvidenceV2) -> AdmissionDecisionV2 {
        AdmissionDecisionV2::new(&self, evidence)
    }

    /// Verifies detached V3 arithmetic from only the Statistics identities that
    /// its typed source can prove and Runner's opaque anchored authority.
    ///
    /// The returned bytes remain non-authoritative until a downstream typed
    /// persistence layer freshly reopens and joins Candidate, Statistics,
    /// Search, and Base capabilities.  No Search/ranking/Finalization identity
    /// can be supplied through this API.
    ///
    /// # Errors
    ///
    /// Returns the first Statistics, walk-forward, or evidence-join refusal.
    pub fn evaluate_v3_projection(
        self,
        base: &AdmissionEvidenceValuesV1,
        statistics: &AdmissionStatisticsDraftV3,
        anchored: &crate::validate::AnchoredSearchValidationV3,
    ) -> Result<AdmissionV3ArithmeticProjection, AdmissionV3ArithmeticRefusal> {
        let fields = AdmissionStatisticsFieldsV3::verify_detached(statistics)
            .map_err(AdmissionV3ArithmeticRefusal::Statistics)?;
        let walk_forward = AnchoredWalkForwardAuthorityV3::from_opaque(anchored)
            .map_err(AdmissionV3ArithmeticRefusal::WalkForward)?;
        let evidence = AdmissionEvidenceV3::new(*base, &fields, walk_forward)
            .map_err(AdmissionV3ArithmeticRefusal::Evidence)?;
        Ok(AdmissionV3ArithmeticProjection::from_decision(
            &self.evaluate_v3_record(&evidence),
        ))
    }

    /// Verifies the same fixed Admission V3 arithmetic from an opaque causal
    /// exact-grid Search V4 object.
    ///
    /// This is still detached arithmetic, not durable authority. The caller
    /// must additionally persist and freshly reopen Search V4 lineage, compare
    /// its typed Long/Short policy and resolution components with the exact
    /// Candidate universe, and bind Base/Statistics records per candidate.
    /// Omitting that outer join is a production refusal, not a fallback to the
    /// older scalar-rung Search V3 path.
    ///
    /// # Errors
    ///
    /// Returns the first Statistics, exact-grid walk, or evidence-join refusal.
    pub fn evaluate_v3_exact_grid_projection(
        self,
        base: &AdmissionEvidenceValuesV1,
        statistics: &AdmissionStatisticsDraftV3,
        anchored: &crate::validate::AnchoredSearchValidationV4,
    ) -> Result<AdmissionV3ArithmeticProjection, AdmissionV3ArithmeticRefusal> {
        let fields = AdmissionStatisticsFieldsV3::verify_detached(statistics)
            .map_err(AdmissionV3ArithmeticRefusal::Statistics)?;
        let walk_forward = AnchoredWalkForwardAuthorityV3::from_exact_grid_opaque(anchored)
            .map_err(AdmissionV3ArithmeticRefusal::WalkForward)?;
        let evidence = AdmissionEvidenceV3::new(*base, &fields, walk_forward)
            .map_err(AdmissionV3ArithmeticRefusal::Evidence)?;
        Ok(AdmissionV3ArithmeticProjection::from_decision(
            &self.evaluate_v3_record(&evidence),
        ))
    }

    #[must_use]
    fn evaluate_v3(self, evidence: &AdmissionEvidenceV3) -> AdmissionVerdictV1 {
        self.evaluate(&AdmissionEvidenceV1 {
            values: evidence.values,
        })
    }

    #[must_use]
    fn evaluate_v3_record(self, evidence: &AdmissionEvidenceV3) -> AdmissionDecisionV3 {
        AdmissionDecisionV3::new(&self, evidence)
    }
}

/// Version-separated detached decision kernel retaining immutable V1
/// policy/verdict bytes and two-source V2 evidence.
///
/// This record is not a durable admission seal. Its constructors and decoders
/// are crate-private until CLI Finalization can supply freshly reopened opaque
/// capabilities for every cross-crate identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AdmissionDecisionV2 {
    policy: AdmissionPolicyV1,
    evidence: AdmissionEvidenceV2,
    verdict: AdmissionVerdictV1,
}

/// Non-authoritative result of the public Admission V2 arithmetic verifier.
///
/// It intentionally exposes the recomputed comparison and verdict bytes, not a
/// durable decision record or seal. Matching bytes prove deterministic Runner
/// arithmetic only; they do not prove fresh CLI receipt/lineage reopening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionV2ArithmeticProjection {
    policy: AdmissionPolicyV1,
    evidence: AdmissionEvidenceV2,
    verdict: AdmissionVerdictV1,
}

impl AdmissionV2ArithmeticProjection {
    fn from_decision(decision: &AdmissionDecisionV2) -> Self {
        Self {
            policy: decision.policy,
            evidence: decision.evidence,
            verdict: decision.verdict,
        }
    }

    /// Revalidates a detached canonical decision record as arithmetic only.
    ///
    /// This decoder does not authorize the identities named by the bytes. A
    /// later CLI Finalization wrapper must freshly reopen and cross-check them
    /// before using this projection in an operator decision.
    ///
    /// # Errors
    ///
    /// Returns the first canonical, Statistics, walk, join or verdict refusal.
    #[cfg(test)]
    fn verify_decision_record_detached(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV2> {
        AdmissionDecisionV2::from_canonical_bytes(bytes)
            .map(|decision| Self::from_decision(&decision))
    }

    /// Exact fixed comparison values produced by the detached verifier.
    #[must_use]
    pub const fn comparison_values(self) -> AdmissionEvidenceValuesV1 {
        self.evidence.values
    }

    /// Canonical detached evidence bytes for byte-exact CLI comparison.
    #[must_use]
    pub fn evidence_bytes(self) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V2] {
        self.evidence.canonical_bytes()
    }

    /// Canonical recomputed V1 verdict bytes for byte-exact CLI comparison.
    #[must_use]
    pub fn verdict_bytes(self) -> [u8; ADMISSION_VERDICT_CANONICAL_LEN_V1] {
        self.verdict.canonical_bytes()
    }

    /// Terminal classification recomputed by Runner's unchanged fixed policy.
    ///
    /// This status remains a detached arithmetic result.  It becomes usable as
    /// production admission evidence only after CLI has freshly reopened and
    /// cross-checked every identity retained by this projection.
    #[must_use]
    pub const fn status(self) -> AdmissionStatusV1 {
        self.verdict.status()
    }

    /// Complete canonical detached Admission V2 decision bytes.
    ///
    /// Runner, rather than a downstream crate, owns this codec.  Returning the
    /// exact envelope prevents CLI persistence from reimplementing policy,
    /// evidence, or verdict byte layout.  These bytes prove deterministic
    /// arithmetic only; they are not a durable admission seal.
    #[must_use]
    pub fn decision_bytes(self) -> [u8; ADMISSION_DECISION_CANONICAL_LEN_V2] {
        AdmissionDecisionV2 {
            policy: self.policy,
            evidence: self.evidence,
            verdict: self.verdict,
        }
        .canonical_bytes()
    }
}

impl AdmissionDecisionV2 {
    fn new(policy: &AdmissionPolicyV1, evidence: &AdmissionEvidenceV2) -> Self {
        let verdict = policy.evaluate_v2(evidence);
        Self {
            policy: *policy,
            evidence: *evidence,
            verdict,
        }
    }

    /// Reconstructs a V2 decision from exact nested records and recomputes the verdict.
    ///
    /// # Errors
    ///
    /// Refuses an invalid V1 policy/verdict, invalid V2 evidence or any supplied
    /// verdict that differs byte-for-byte from fresh V2 evaluation.
    #[cfg(test)]
    fn from_canonical_parts(
        policy_bytes: &[u8],
        evidence_bytes: &[u8],
        verdict_bytes: &[u8],
    ) -> Result<Self, AdmissionCanonicalRefusalV2> {
        let policy = AdmissionPolicyV1::from_canonical_bytes(policy_bytes)
            .map_err(AdmissionCanonicalRefusalV2::NestedV1)?;
        let evidence = AdmissionEvidenceV2::from_canonical_bytes(evidence_bytes)?;
        let supplied = AdmissionVerdictV1::from_canonical_bytes(verdict_bytes)
            .map_err(AdmissionCanonicalRefusalV2::NestedV1)?;
        let computed = policy.evaluate_v2(&evidence);
        if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
            return Err(AdmissionCanonicalRefusalV2::VerdictMismatch);
        }
        Ok(Self {
            policy,
            evidence,
            verdict: computed,
        })
    }

    /// Decodes a complete V2 decision and reauthorizes its verdict.
    ///
    /// # Errors
    ///
    /// Returns an exact outer, nested, authority, evidence or verdict refusal.
    #[cfg(test)]
    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV2> {
        decode_decision_v2(bytes)
    }

    /// Verdict recomputed through the unchanged fixed policy checks.
    #[cfg(test)]
    #[must_use]
    const fn verdict(self) -> AdmissionVerdictV1 {
        self.verdict
    }

    /// Whether fresh evaluation reproduces the detached verdict.
    #[cfg(test)]
    #[must_use]
    fn recomputes(self) -> bool {
        self.policy.evaluate_v2(&self.evidence) == self.verdict
    }

    /// Canonical V2 decision bytes.
    #[must_use]
    fn canonical_bytes(self) -> [u8; ADMISSION_DECISION_CANONICAL_LEN_V2] {
        let policy = self.policy.canonical_bytes();
        let evidence = self.evidence.canonical_bytes();
        let verdict = self.verdict.canonical_bytes();
        let mut bytes = [0_u8; ADMISSION_DECISION_CANONICAL_LEN_V2];
        let mut writer = CanonicalWriter::new_version(
            &mut bytes,
            DECISION_DOMAIN_V1,
            DECISION_PAYLOAD_LEN_V2,
            ADMISSION_VERSION_V2,
        );
        writer.put_slice(&policy);
        writer.put_slice(&evidence);
        writer.put_slice(&verdict);
        writer.finish();
        bytes
    }
}

/// Detached version-separated Admission V3 decision kernel.
///
/// It is not a durable admission seal; production authority remains the
/// responsibility of the later freshly reopened CLI capability join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AdmissionDecisionV3 {
    policy: AdmissionPolicyV1,
    evidence: AdmissionEvidenceV3,
    verdict: AdmissionVerdictV1,
}

/// Non-authoritative result of Admission V3 arithmetic verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionV3ArithmeticProjection {
    policy: AdmissionPolicyV1,
    evidence: AdmissionEvidenceV3,
    verdict: AdmissionVerdictV1,
}

impl AdmissionV3ArithmeticProjection {
    fn from_decision(decision: &AdmissionDecisionV3) -> Self {
        Self {
            policy: decision.policy,
            evidence: decision.evidence,
            verdict: decision.verdict,
        }
    }

    /// Revalidates a canonical Admission V3 decision as detached arithmetic only.
    ///
    /// The decoder revalidates the outer V3 envelope, immutable V1 policy,
    /// Statistics fields, anchored walk facts, reconstructed comparison values,
    /// and the byte-exact verdict produced by Runner's fixed policy.  The
    /// returned projection grants **no persistence, provenance, Search,
    /// Candidate, ranking, or Finalization authority**.  A caller may use it to
    /// compare arithmetic bytes, but must freshly reopen and join every durable
    /// capability before making an operator-visible admission decision.
    ///
    /// # Errors
    ///
    /// Returns the first canonical, Statistics, walk, evidence-join, nested V1,
    /// authority-projection, or recomputed-verdict refusal.
    pub fn verify_decision_record_detached(
        bytes: &[u8],
    ) -> Result<Self, AdmissionCanonicalRefusalV3> {
        AdmissionDecisionV3::from_canonical_bytes(bytes)
            .map(|decision| Self::from_decision(&decision))
    }

    /// Exact fixed comparison values produced by Runner.
    #[must_use]
    pub const fn comparison_values(self) -> AdmissionEvidenceValuesV1 {
        self.evidence.values
    }

    /// Canonical detached V3 evidence bytes.
    #[must_use]
    pub fn evidence_bytes(self) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V3] {
        self.evidence.canonical_bytes()
    }

    /// Canonical recomputed V1 verdict bytes.
    #[must_use]
    pub fn verdict_bytes(self) -> [u8; ADMISSION_VERDICT_CANONICAL_LEN_V1] {
        self.verdict.canonical_bytes()
    }

    /// Typed verdict recomputed by Runner's fixed Admission policy.
    ///
    /// This exposes the exact reason partitions without asking a downstream
    /// crate to decode canonical bytes.  It remains detached arithmetic only:
    /// the value carries no persistence, provenance, Search, Candidate,
    /// ranking, or Finalization authority.
    #[must_use]
    pub const fn verdict(self) -> AdmissionVerdictV1 {
        self.verdict
    }

    /// Terminal classification recomputed by Runner's fixed policy.
    #[must_use]
    pub const fn status(self) -> AdmissionStatusV1 {
        self.verdict.status()
    }

    /// Complete canonical detached Admission V3 decision bytes.
    #[must_use]
    pub fn decision_bytes(self) -> [u8; ADMISSION_DECISION_CANONICAL_LEN_V3] {
        AdmissionDecisionV3 {
            policy: self.policy,
            evidence: self.evidence,
            verdict: self.verdict,
        }
        .canonical_bytes()
    }
}

impl AdmissionDecisionV3 {
    fn new(policy: &AdmissionPolicyV1, evidence: &AdmissionEvidenceV3) -> Self {
        let verdict = policy.evaluate_v3(evidence);
        Self {
            policy: *policy,
            evidence: *evidence,
            verdict,
        }
    }

    fn from_canonical_parts(
        policy_bytes: &[u8],
        evidence_bytes: &[u8],
        verdict_bytes: &[u8],
    ) -> Result<Self, AdmissionCanonicalRefusalV3> {
        let policy = AdmissionPolicyV1::from_canonical_bytes(policy_bytes)
            .map_err(AdmissionCanonicalRefusalV3::NestedV1)?;
        let evidence = AdmissionEvidenceV3::from_canonical_bytes(evidence_bytes)?;
        let supplied = AdmissionVerdictV1::from_canonical_bytes(verdict_bytes)
            .map_err(AdmissionCanonicalRefusalV3::NestedV1)?;
        let computed = policy.evaluate_v3(&evidence);
        if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
            return Err(AdmissionCanonicalRefusalV3::VerdictMismatch);
        }
        Ok(Self {
            policy,
            evidence,
            verdict: computed,
        })
    }

    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, AdmissionCanonicalRefusalV3> {
        decode_decision_v3(bytes)
    }

    #[cfg(test)]
    #[must_use]
    const fn verdict(self) -> AdmissionVerdictV1 {
        self.verdict
    }

    #[cfg(test)]
    #[must_use]
    fn recomputes(self) -> bool {
        self.policy.evaluate_v3(&self.evidence) == self.verdict
    }

    #[must_use]
    fn canonical_bytes(self) -> [u8; ADMISSION_DECISION_CANONICAL_LEN_V3] {
        let policy = self.policy.canonical_bytes();
        let evidence = self.evidence.canonical_bytes();
        let verdict = self.verdict.canonical_bytes();
        let mut bytes = [0_u8; ADMISSION_DECISION_CANONICAL_LEN_V3];
        let mut writer = CanonicalWriter::new_version(
            &mut bytes,
            DECISION_DOMAIN_V1,
            DECISION_PAYLOAD_LEN_V3,
            ADMISSION_VERSION_V3,
        );
        writer.put_slice(&policy);
        writer.put_slice(&evidence);
        writer.put_slice(&verdict);
        writer.finish();
        bytes
    }
}

fn canonical_evidence_bytes_v2(
    evidence: &AdmissionEvidenceV2,
) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V2] {
    let mut bytes = [0_u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V2];
    let mut writer = CanonicalWriter::new_version(
        &mut bytes,
        EVIDENCE_DOMAIN_V1,
        EVIDENCE_PAYLOAD_LEN_V2,
        ADMISSION_VERSION_V2,
    );
    put_evidence_values(&mut writer, &evidence.values);
    let statistics = evidence.statistics.values();
    put_statistics_fields_v2(&mut writer, &statistics);
    put_walk_forward_v2(&mut writer, evidence.walk_forward);
    writer.finish();
    bytes
}

fn canonical_evidence_bytes_v3(
    evidence: &AdmissionEvidenceV3,
) -> [u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V3] {
    let mut bytes = [0_u8; ADMISSION_EVIDENCE_CANONICAL_LEN_V3];
    let mut writer = CanonicalWriter::new_version(
        &mut bytes,
        EVIDENCE_DOMAIN_V1,
        EVIDENCE_PAYLOAD_LEN_V3,
        ADMISSION_VERSION_V3,
    );
    put_evidence_values(&mut writer, &evidence.values);
    let statistics = evidence.statistics.values();
    put_statistics_fields_v3(&mut writer, &statistics);
    put_walk_forward_v3(&mut writer, evidence.walk_forward);
    writer.finish();
    bytes
}

fn put_statistics_fields_v2(writer: &mut CanonicalWriter<'_>, values: &AdmissionStatisticsDraftV2) {
    for identity in [
        values.candidate_semantic_id,
        values.population_search_id,
        values.ranking_validation_policy_digest,
        values.source_policy_digest,
        values.finalization_family_digest,
        values.statistics_audit_id,
        values.statistics_completion_digest,
        values.observation_statistics_link_id,
        values.cscv_policy_digest,
        values.cscv_split_family_digest,
        values.white_family_digest,
        values.spa_family_digest,
        values.romano_wolf_family_digest,
    ] {
        writer.put_slice(&identity);
    }
    for value in [
        values.trades,
        values.wins,
        values.wilson_lower_bits,
        values.wilson_win_rate_ppm,
        values.cscv_split_count,
        values.pbo_contributing_splits,
        values.pbo_unrankable_splits,
    ] {
        writer.put_u64(value);
    }
    for probability in [
        values.pbo_probability,
        values.white_probability,
        values.spa_probability,
        values.familywise_romano_wolf_probability,
        values.candidate_romano_wolf_probability,
    ] {
        writer.put_u64(probability.numerator());
        writer.put_u64(probability.denominator());
    }
    for value in [
        values.bootstrap_draws,
        values.bootstrap_strategies,
        values.bootstrap_periods,
    ] {
        writer.put_u64(value);
    }
}

fn put_statistics_fields_v3(writer: &mut CanonicalWriter<'_>, values: &AdmissionStatisticsDraftV3) {
    for identity in [
        values.candidate_semantic_id,
        values.statistics_audit_id,
        values.statistics_completion_digest,
        values.observation_statistics_link_id,
        values.cscv_policy_digest,
        values.cscv_split_family_digest,
        values.white_family_digest,
        values.spa_family_digest,
        values.romano_wolf_family_digest,
    ] {
        writer.put_slice(&identity);
    }
    for value in [
        values.trades,
        values.wins,
        values.wilson_lower_bits,
        values.wilson_win_rate_ppm,
        values.cscv_split_count,
        values.pbo_contributing_splits,
        values.pbo_unrankable_splits,
    ] {
        writer.put_u64(value);
    }
    for probability in [
        values.pbo_probability,
        values.white_probability,
        values.spa_probability,
        values.familywise_romano_wolf_probability,
        values.candidate_romano_wolf_probability,
    ] {
        writer.put_u64(probability.numerator());
        writer.put_u64(probability.denominator());
    }
    for value in [
        values.bootstrap_draws,
        values.bootstrap_strategies,
        values.bootstrap_periods,
    ] {
        writer.put_u64(value);
    }
}

fn put_walk_forward_v2(writer: &mut CanonicalWriter<'_>, value: AnchoredWalkForwardAuthorityV2) {
    for identity in [
        value.validation_policy_digest,
        value.validation_family_digest,
        value.walk_facts_digest,
        value.authority_id,
    ] {
        writer.put_slice(&identity);
    }
    writer.put_u64(value.fold_count);
    writer.put_u64(value.decided_folds);
    writer.put_u64(value.profitable_oos_folds);
    writer.put_i64(value.aggregate_oos_paisa);
}

fn put_walk_forward_v3(writer: &mut CanonicalWriter<'_>, value: AnchoredWalkForwardAuthorityV3) {
    for identity in [
        value.validation_policy_digest,
        value.validation_family_digest,
        value.walk_facts_digest,
        value.authority_id,
    ] {
        writer.put_slice(&identity);
    }
    writer.put_u64(value.fold_count);
    writer.put_u64(value.decided_folds);
    writer.put_u64(value.profitable_oos_folds);
    writer.put_i64(value.aggregate_oos_paisa);
}

#[cfg(test)]
struct CanonicalReaderV2<'a> {
    bytes: &'a [u8],
    offset: usize,
    record: AdmissionCanonicalRecordV2,
    expected_len: usize,
}

#[cfg(test)]
impl<'a> CanonicalReaderV2<'a> {
    fn new(
        bytes: &'a [u8],
        record: AdmissionCanonicalRecordV2,
        expected_len: usize,
        expected_domain: u8,
        expected_payload_len: u32,
    ) -> Result<Self, AdmissionCanonicalRefusalV2> {
        if bytes.len() != expected_len {
            return Err(AdmissionCanonicalRefusalV2::Length {
                record,
                expected: expected_len,
                actual: bytes.len(),
            });
        }
        if !bytes.starts_with(b"BADM") {
            return Err(AdmissionCanonicalRefusalV2::Magic { record });
        }
        let domain = bytes.get(4).copied().unwrap_or_default();
        if domain != expected_domain {
            return Err(AdmissionCanonicalRefusalV2::Domain {
                record,
                actual: domain,
            });
        }
        if bytes.get(5).copied().unwrap_or_default() != 0 {
            return Err(AdmissionCanonicalRefusalV2::Reserved { record, offset: 5 });
        }
        let mut version_bytes = [0_u8; 2];
        if let Some(source) = bytes.get(6..8) {
            version_bytes.copy_from_slice(source);
        }
        let version = u16::from_le_bytes(version_bytes);
        if version != ADMISSION_VERSION_V2 {
            return Err(AdmissionCanonicalRefusalV2::Version {
                record,
                actual: version,
            });
        }
        let mut payload_bytes = [0_u8; 4];
        if let Some(source) = bytes.get(8..12) {
            payload_bytes.copy_from_slice(source);
        }
        let payload_len = u32::from_le_bytes(payload_bytes);
        if payload_len != expected_payload_len {
            return Err(AdmissionCanonicalRefusalV2::PayloadLength {
                record,
                actual: payload_len,
            });
        }
        Ok(Self {
            bytes,
            offset: CANONICAL_HEADER_LEN,
            record,
            expected_len,
        })
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], AdmissionCanonicalRefusalV2> {
        let Some(end) = self.offset.checked_add(N) else {
            return Err(AdmissionCanonicalRefusalV2::Length {
                record: self.record,
                expected: usize::MAX,
                actual: self.expected_len,
            });
        };
        let Some(source) = self.bytes.get(self.offset..end) else {
            return Err(AdmissionCanonicalRefusalV2::Length {
                record: self.record,
                expected: end,
                actual: self.expected_len,
            });
        };
        let mut value = [0_u8; N];
        value.copy_from_slice(source);
        self.offset = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, AdmissionCanonicalRefusalV2> {
        Ok(self.take::<1>()?[0])
    }

    fn read_u64(&mut self) -> Result<u64, AdmissionCanonicalRefusalV2> {
        Ok(u64::from_le_bytes(self.take::<8>()?))
    }

    fn read_i64(&mut self) -> Result<i64, AdmissionCanonicalRefusalV2> {
        Ok(i64::from_le_bytes(self.take::<8>()?))
    }

    fn read_observed_u64(&mut self) -> Result<ObservedU64V1, AdmissionCanonicalRefusalV2> {
        let tag_offset = self.offset;
        let tag = self.read_u8()?;
        let payload_offset = self.offset;
        let payload = self.take::<8>()?;
        match tag {
            0 => Ok(ObservedU64V1::Measured(u64::from_le_bytes(payload))),
            1 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedU64V1::Unmeasured)
            }
            2 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedU64V1::Refused)
            }
            actual => Err(AdmissionCanonicalRefusalV2::Tag {
                record: self.record,
                offset: tag_offset,
                actual,
            }),
        }
    }

    fn read_observed_i64(&mut self) -> Result<ObservedI64V1, AdmissionCanonicalRefusalV2> {
        let tag_offset = self.offset;
        let tag = self.read_u8()?;
        let payload_offset = self.offset;
        let payload = self.take::<8>()?;
        match tag {
            0 => Ok(ObservedI64V1::Measured(i64::from_le_bytes(payload))),
            1 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedI64V1::Unmeasured)
            }
            2 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedI64V1::Refused)
            }
            actual => Err(AdmissionCanonicalRefusalV2::Tag {
                record: self.record,
                offset: tag_offset,
                actual,
            }),
        }
    }

    fn read_completeness(&mut self) -> Result<CompletenessV1, AdmissionCanonicalRefusalV2> {
        let offset = self.offset;
        match self.read_u8()? {
            0 => Ok(CompletenessV1::Complete),
            1 => Ok(CompletenessV1::Incomplete),
            2 => Ok(CompletenessV1::Unmeasured),
            3 => Ok(CompletenessV1::Refused),
            actual => Err(AdmissionCanonicalRefusalV2::Tag {
                record: self.record,
                offset,
                actual,
            }),
        }
    }

    fn read_hypothesis_decision(
        &mut self,
    ) -> Result<HypothesisDecisionV1, AdmissionCanonicalRefusalV2> {
        let offset = self.offset;
        match self.read_u8()? {
            0 => Ok(HypothesisDecisionV1::RejectedNull),
            1 => Ok(HypothesisDecisionV1::DidNotReject),
            2 => Ok(HypothesisDecisionV1::Unmeasured),
            3 => Ok(HypothesisDecisionV1::Refused),
            actual => Err(AdmissionCanonicalRefusalV2::Tag {
                record: self.record,
                offset,
                actual,
            }),
        }
    }

    fn require_zero(&self, bytes: &[u8], start: usize) -> Result<(), AdmissionCanonicalRefusalV2> {
        if let Some((relative, _)) = bytes.iter().enumerate().find(|(_, byte)| **byte != 0) {
            return Err(AdmissionCanonicalRefusalV2::Reserved {
                record: self.record,
                offset: start.saturating_add(relative),
            });
        }
        Ok(())
    }

    fn finish(self) {
        debug_assert_eq!(self.offset, self.bytes.len());
    }
}

#[cfg(test)]
fn read_evidence_values_v2(
    reader: &mut CanonicalReaderV2<'_>,
) -> Result<AdmissionEvidenceValuesV1, AdmissionCanonicalRefusalV2> {
    Ok(AdmissionEvidenceValuesV1 {
        support_hits: reader.read_observed_u64()?,
        independent_sessions: reader.read_observed_u64()?,
        trades: reader.read_observed_u64()?,
        max_mae_paisa: reader.read_observed_u64()?,
        worst_reward_risk_ppm: reader.read_observed_u64()?,
        win_rate_ppm: reader.read_observed_u64()?,
        wilson_win_rate_ppm: reader.read_observed_u64()?,
        return_drawdown_ppm: reader.read_observed_u64()?,
        weakest_period_return_paisa: reader.read_observed_i64()?,
        pbo_ppm: reader.read_observed_u64()?,
        fwer_p_value_ppm: reader.read_observed_u64()?,
        spa_p_value_ppm: reader.read_observed_u64()?,
        decided_folds: reader.read_observed_u64()?,
        ambiguous_fill_rate_ppm: reader.read_observed_u64()?,
        gap_affected_rate_ppm: reader.read_observed_u64()?,
        session_concentration_ppm: reader.read_observed_u64()?,
        largest_trade_profit_share_ppm: reader.read_observed_u64()?,
        execution_complete: reader.read_completeness()?,
        data_complete: reader.read_completeness()?,
        calendar_complete: reader.read_completeness()?,
        population_complete: reader.read_completeness()?,
        drawdown_paisa: reader.read_observed_u64()?,
        worst_trade_loss_paisa: reader.read_observed_u64()?,
        losing_trade_rate_ppm: reader.read_observed_u64()?,
        losing_trades: reader.read_observed_u64()?,
        pessimistic_profit_paisa: reader.read_observed_i64()?,
        winning_trades: reader.read_observed_u64()?,
        average_win_paisa: reader.read_observed_u64()?,
        average_loss_paisa: reader.read_observed_u64()?,
        profit_factor_ppm: reader.read_observed_u64()?,
        consecutive_losing_streak: reader.read_observed_u64()?,
        consecutive_winning_streak: reader.read_observed_u64()?,
        bootstrap_draws: reader.read_observed_u64()?,
        bootstrap_strategies: reader.read_observed_u64()?,
        bootstrap_periods: reader.read_observed_u64()?,
        pbo_contributing_folds: reader.read_observed_u64()?,
        pbo_unrankable_folds: reader.read_observed_u64()?,
        profitable_oos_folds: reader.read_observed_u64()?,
        oos_pessimistic_return_paisa: reader.read_observed_i64()?,
        white_reality_p_value_ppm: reader.read_observed_u64()?,
        romano_wolf_p_value_ppm: reader.read_observed_u64()?,
        white_reality_decision: reader.read_hypothesis_decision()?,
        romano_wolf_decision: reader.read_hypothesis_decision()?,
        full_precision_statistics_complete: reader.read_completeness()?,
    })
}

#[cfg(test)]
fn read_probability_v2(
    reader: &mut CanonicalReaderV2<'_>,
    field: AdmissionStatisticsProbabilityV2,
) -> Result<AdmissionExactProbabilityV2, AdmissionCanonicalRefusalV2> {
    AdmissionExactProbabilityV2::new(reader.read_u64()?, reader.read_u64()?).map_err(|kind| {
        AdmissionCanonicalRefusalV2::Statistics(AdmissionStatisticsRefusalV2::Probability {
            field,
            kind,
        })
    })
}

#[cfg(test)]
fn read_statistics_fields_v2(
    reader: &mut CanonicalReaderV2<'_>,
) -> Result<AdmissionStatisticsFieldsV2, AdmissionCanonicalRefusalV2> {
    let values = AdmissionStatisticsDraftV2 {
        candidate_semantic_id: reader.take::<32>()?,
        population_search_id: reader.take::<32>()?,
        ranking_validation_policy_digest: reader.take::<32>()?,
        source_policy_digest: reader.take::<32>()?,
        finalization_family_digest: reader.take::<32>()?,
        statistics_audit_id: reader.take::<32>()?,
        statistics_completion_digest: reader.take::<32>()?,
        observation_statistics_link_id: reader.take::<32>()?,
        cscv_policy_digest: reader.take::<32>()?,
        cscv_split_family_digest: reader.take::<32>()?,
        white_family_digest: reader.take::<32>()?,
        spa_family_digest: reader.take::<32>()?,
        romano_wolf_family_digest: reader.take::<32>()?,
        trades: reader.read_u64()?,
        wins: reader.read_u64()?,
        wilson_lower_bits: reader.read_u64()?,
        wilson_win_rate_ppm: reader.read_u64()?,
        cscv_split_count: reader.read_u64()?,
        pbo_contributing_splits: reader.read_u64()?,
        pbo_unrankable_splits: reader.read_u64()?,
        pbo_probability: read_probability_v2(reader, AdmissionStatisticsProbabilityV2::CscvPbo)?,
        white_probability: read_probability_v2(
            reader,
            AdmissionStatisticsProbabilityV2::WhiteReality,
        )?,
        spa_probability: read_probability_v2(reader, AdmissionStatisticsProbabilityV2::Spa)?,
        familywise_romano_wolf_probability: read_probability_v2(
            reader,
            AdmissionStatisticsProbabilityV2::FamilywiseRomanoWolf,
        )?,
        candidate_romano_wolf_probability: read_probability_v2(
            reader,
            AdmissionStatisticsProbabilityV2::CandidateRomanoWolf,
        )?,
        bootstrap_draws: reader.read_u64()?,
        bootstrap_strategies: reader.read_u64()?,
        bootstrap_periods: reader.read_u64()?,
    };
    AdmissionStatisticsFieldsV2::verify_detached(&values)
        .map_err(AdmissionCanonicalRefusalV2::Statistics)
}

#[cfg(test)]
fn read_walk_forward_v2(
    reader: &mut CanonicalReaderV2<'_>,
) -> Result<AnchoredWalkForwardAuthorityV2, AdmissionCanonicalRefusalV2> {
    AnchoredWalkForwardAuthorityV2::from_canonical_fields(AnchoredWalkForwardAuthorityV2 {
        validation_policy_digest: reader.take::<32>()?,
        validation_family_digest: reader.take::<32>()?,
        walk_facts_digest: reader.take::<32>()?,
        authority_id: reader.take::<32>()?,
        fold_count: reader.read_u64()?,
        decided_folds: reader.read_u64()?,
        profitable_oos_folds: reader.read_u64()?,
        aggregate_oos_paisa: reader.read_i64()?,
    })
    .map_err(AdmissionCanonicalRefusalV2::WalkForward)
}

#[cfg(test)]
fn decode_evidence_v2(bytes: &[u8]) -> Result<AdmissionEvidenceV2, AdmissionCanonicalRefusalV2> {
    let mut reader = CanonicalReaderV2::new(
        bytes,
        AdmissionCanonicalRecordV2::Evidence,
        ADMISSION_EVIDENCE_CANONICAL_LEN_V2,
        EVIDENCE_DOMAIN_V1,
        EVIDENCE_PAYLOAD_LEN_V2,
    )?;
    let decoded_values = read_evidence_values_v2(&mut reader)?;
    let statistics = read_statistics_fields_v2(&mut reader)?;
    let walk_forward = read_walk_forward_v2(&mut reader)?;
    reader.finish();
    let mut base = decoded_values;
    clear_authority_slots_v2(&mut base);
    let evidence = AdmissionEvidenceV2::new(base, &statistics, walk_forward)
        .map_err(AdmissionCanonicalRefusalV2::Evidence)?;
    if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
        return Err(AdmissionCanonicalRefusalV2::AuthorityProjectionMismatch);
    }
    Ok(evidence)
}

#[cfg(test)]
fn decode_decision_v2(bytes: &[u8]) -> Result<AdmissionDecisionV2, AdmissionCanonicalRefusalV2> {
    let mut reader = CanonicalReaderV2::new(
        bytes,
        AdmissionCanonicalRecordV2::Decision,
        ADMISSION_DECISION_CANONICAL_LEN_V2,
        DECISION_DOMAIN_V1,
        DECISION_PAYLOAD_LEN_V2,
    )?;
    let policy = reader.take::<ADMISSION_POLICY_CANONICAL_LEN_V1>()?;
    let evidence = reader.take::<ADMISSION_EVIDENCE_CANONICAL_LEN_V2>()?;
    let verdict = reader.take::<ADMISSION_VERDICT_CANONICAL_LEN_V1>()?;
    reader.finish();
    AdmissionDecisionV2::from_canonical_parts(&policy, &evidence, &verdict)
}

struct CanonicalReaderV3<'a> {
    bytes: &'a [u8],
    offset: usize,
    record: AdmissionCanonicalRecordV3,
    expected_len: usize,
}

impl<'a> CanonicalReaderV3<'a> {
    fn new(
        bytes: &'a [u8],
        record: AdmissionCanonicalRecordV3,
        expected_len: usize,
        expected_domain: u8,
        expected_payload_len: u32,
    ) -> Result<Self, AdmissionCanonicalRefusalV3> {
        if bytes.len() != expected_len {
            return Err(AdmissionCanonicalRefusalV3::Length {
                record,
                expected: expected_len,
                actual: bytes.len(),
            });
        }
        if !bytes.starts_with(b"BADM") {
            return Err(AdmissionCanonicalRefusalV3::Magic { record });
        }
        let domain = bytes.get(4).copied().unwrap_or_default();
        if domain != expected_domain {
            return Err(AdmissionCanonicalRefusalV3::Domain {
                record,
                actual: domain,
            });
        }
        if bytes.get(5).copied().unwrap_or_default() != 0 {
            return Err(AdmissionCanonicalRefusalV3::Reserved { record, offset: 5 });
        }
        let mut version_bytes = [0_u8; 2];
        if let Some(source) = bytes.get(6..8) {
            version_bytes.copy_from_slice(source);
        }
        let version = u16::from_le_bytes(version_bytes);
        if version != ADMISSION_VERSION_V3 {
            return Err(AdmissionCanonicalRefusalV3::Version {
                record,
                actual: version,
            });
        }
        let mut payload_bytes = [0_u8; 4];
        if let Some(source) = bytes.get(8..12) {
            payload_bytes.copy_from_slice(source);
        }
        let payload_len = u32::from_le_bytes(payload_bytes);
        if payload_len != expected_payload_len {
            return Err(AdmissionCanonicalRefusalV3::PayloadLength {
                record,
                actual: payload_len,
            });
        }
        Ok(Self {
            bytes,
            offset: CANONICAL_HEADER_LEN,
            record,
            expected_len,
        })
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], AdmissionCanonicalRefusalV3> {
        let Some(end) = self.offset.checked_add(N) else {
            return Err(AdmissionCanonicalRefusalV3::Length {
                record: self.record,
                expected: usize::MAX,
                actual: self.expected_len,
            });
        };
        let Some(source) = self.bytes.get(self.offset..end) else {
            return Err(AdmissionCanonicalRefusalV3::Length {
                record: self.record,
                expected: end,
                actual: self.expected_len,
            });
        };
        let mut value = [0_u8; N];
        value.copy_from_slice(source);
        self.offset = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, AdmissionCanonicalRefusalV3> {
        Ok(self.take::<1>()?[0])
    }

    fn read_u64(&mut self) -> Result<u64, AdmissionCanonicalRefusalV3> {
        Ok(u64::from_le_bytes(self.take::<8>()?))
    }

    fn read_i64(&mut self) -> Result<i64, AdmissionCanonicalRefusalV3> {
        Ok(i64::from_le_bytes(self.take::<8>()?))
    }

    fn read_observed_u64(&mut self) -> Result<ObservedU64V1, AdmissionCanonicalRefusalV3> {
        let tag_offset = self.offset;
        let tag = self.read_u8()?;
        let payload_offset = self.offset;
        let payload = self.take::<8>()?;
        match tag {
            0 => Ok(ObservedU64V1::Measured(u64::from_le_bytes(payload))),
            1 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedU64V1::Unmeasured)
            }
            2 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedU64V1::Refused)
            }
            actual => Err(AdmissionCanonicalRefusalV3::Tag {
                record: self.record,
                offset: tag_offset,
                actual,
            }),
        }
    }

    fn read_observed_i64(&mut self) -> Result<ObservedI64V1, AdmissionCanonicalRefusalV3> {
        let tag_offset = self.offset;
        let tag = self.read_u8()?;
        let payload_offset = self.offset;
        let payload = self.take::<8>()?;
        match tag {
            0 => Ok(ObservedI64V1::Measured(i64::from_le_bytes(payload))),
            1 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedI64V1::Unmeasured)
            }
            2 => {
                self.require_zero(&payload, payload_offset)?;
                Ok(ObservedI64V1::Refused)
            }
            actual => Err(AdmissionCanonicalRefusalV3::Tag {
                record: self.record,
                offset: tag_offset,
                actual,
            }),
        }
    }

    fn read_completeness(&mut self) -> Result<CompletenessV1, AdmissionCanonicalRefusalV3> {
        let offset = self.offset;
        match self.read_u8()? {
            0 => Ok(CompletenessV1::Complete),
            1 => Ok(CompletenessV1::Incomplete),
            2 => Ok(CompletenessV1::Unmeasured),
            3 => Ok(CompletenessV1::Refused),
            actual => Err(AdmissionCanonicalRefusalV3::Tag {
                record: self.record,
                offset,
                actual,
            }),
        }
    }

    fn read_hypothesis_decision(
        &mut self,
    ) -> Result<HypothesisDecisionV1, AdmissionCanonicalRefusalV3> {
        let offset = self.offset;
        match self.read_u8()? {
            0 => Ok(HypothesisDecisionV1::RejectedNull),
            1 => Ok(HypothesisDecisionV1::DidNotReject),
            2 => Ok(HypothesisDecisionV1::Unmeasured),
            3 => Ok(HypothesisDecisionV1::Refused),
            actual => Err(AdmissionCanonicalRefusalV3::Tag {
                record: self.record,
                offset,
                actual,
            }),
        }
    }

    fn require_zero(&self, bytes: &[u8], start: usize) -> Result<(), AdmissionCanonicalRefusalV3> {
        if let Some((relative, _)) = bytes.iter().enumerate().find(|(_, byte)| **byte != 0) {
            return Err(AdmissionCanonicalRefusalV3::Reserved {
                record: self.record,
                offset: start.saturating_add(relative),
            });
        }
        Ok(())
    }

    fn finish(self) {
        debug_assert_eq!(self.offset, self.bytes.len());
    }
}

fn read_evidence_values_v3(
    reader: &mut CanonicalReaderV3<'_>,
) -> Result<AdmissionEvidenceValuesV1, AdmissionCanonicalRefusalV3> {
    Ok(AdmissionEvidenceValuesV1 {
        support_hits: reader.read_observed_u64()?,
        independent_sessions: reader.read_observed_u64()?,
        trades: reader.read_observed_u64()?,
        max_mae_paisa: reader.read_observed_u64()?,
        worst_reward_risk_ppm: reader.read_observed_u64()?,
        win_rate_ppm: reader.read_observed_u64()?,
        wilson_win_rate_ppm: reader.read_observed_u64()?,
        return_drawdown_ppm: reader.read_observed_u64()?,
        weakest_period_return_paisa: reader.read_observed_i64()?,
        pbo_ppm: reader.read_observed_u64()?,
        fwer_p_value_ppm: reader.read_observed_u64()?,
        spa_p_value_ppm: reader.read_observed_u64()?,
        decided_folds: reader.read_observed_u64()?,
        ambiguous_fill_rate_ppm: reader.read_observed_u64()?,
        gap_affected_rate_ppm: reader.read_observed_u64()?,
        session_concentration_ppm: reader.read_observed_u64()?,
        largest_trade_profit_share_ppm: reader.read_observed_u64()?,
        execution_complete: reader.read_completeness()?,
        data_complete: reader.read_completeness()?,
        calendar_complete: reader.read_completeness()?,
        population_complete: reader.read_completeness()?,
        drawdown_paisa: reader.read_observed_u64()?,
        worst_trade_loss_paisa: reader.read_observed_u64()?,
        losing_trade_rate_ppm: reader.read_observed_u64()?,
        losing_trades: reader.read_observed_u64()?,
        pessimistic_profit_paisa: reader.read_observed_i64()?,
        winning_trades: reader.read_observed_u64()?,
        average_win_paisa: reader.read_observed_u64()?,
        average_loss_paisa: reader.read_observed_u64()?,
        profit_factor_ppm: reader.read_observed_u64()?,
        consecutive_losing_streak: reader.read_observed_u64()?,
        consecutive_winning_streak: reader.read_observed_u64()?,
        bootstrap_draws: reader.read_observed_u64()?,
        bootstrap_strategies: reader.read_observed_u64()?,
        bootstrap_periods: reader.read_observed_u64()?,
        pbo_contributing_folds: reader.read_observed_u64()?,
        pbo_unrankable_folds: reader.read_observed_u64()?,
        profitable_oos_folds: reader.read_observed_u64()?,
        oos_pessimistic_return_paisa: reader.read_observed_i64()?,
        white_reality_p_value_ppm: reader.read_observed_u64()?,
        romano_wolf_p_value_ppm: reader.read_observed_u64()?,
        white_reality_decision: reader.read_hypothesis_decision()?,
        romano_wolf_decision: reader.read_hypothesis_decision()?,
        full_precision_statistics_complete: reader.read_completeness()?,
    })
}

fn read_probability_v3(
    reader: &mut CanonicalReaderV3<'_>,
    field: AdmissionStatisticsProbabilityV2,
) -> Result<AdmissionExactProbabilityV2, AdmissionCanonicalRefusalV3> {
    AdmissionExactProbabilityV2::new(reader.read_u64()?, reader.read_u64()?).map_err(|kind| {
        AdmissionCanonicalRefusalV3::Statistics(AdmissionStatisticsRefusalV3::Probability {
            field,
            kind,
        })
    })
}

fn read_statistics_fields_v3(
    reader: &mut CanonicalReaderV3<'_>,
) -> Result<AdmissionStatisticsFieldsV3, AdmissionCanonicalRefusalV3> {
    let values = AdmissionStatisticsDraftV3 {
        candidate_semantic_id: reader.take::<32>()?,
        statistics_audit_id: reader.take::<32>()?,
        statistics_completion_digest: reader.take::<32>()?,
        observation_statistics_link_id: reader.take::<32>()?,
        cscv_policy_digest: reader.take::<32>()?,
        cscv_split_family_digest: reader.take::<32>()?,
        white_family_digest: reader.take::<32>()?,
        spa_family_digest: reader.take::<32>()?,
        romano_wolf_family_digest: reader.take::<32>()?,
        trades: reader.read_u64()?,
        wins: reader.read_u64()?,
        wilson_lower_bits: reader.read_u64()?,
        wilson_win_rate_ppm: reader.read_u64()?,
        cscv_split_count: reader.read_u64()?,
        pbo_contributing_splits: reader.read_u64()?,
        pbo_unrankable_splits: reader.read_u64()?,
        pbo_probability: read_probability_v3(reader, AdmissionStatisticsProbabilityV2::CscvPbo)?,
        white_probability: read_probability_v3(
            reader,
            AdmissionStatisticsProbabilityV2::WhiteReality,
        )?,
        spa_probability: read_probability_v3(reader, AdmissionStatisticsProbabilityV2::Spa)?,
        familywise_romano_wolf_probability: read_probability_v3(
            reader,
            AdmissionStatisticsProbabilityV2::FamilywiseRomanoWolf,
        )?,
        candidate_romano_wolf_probability: read_probability_v3(
            reader,
            AdmissionStatisticsProbabilityV2::CandidateRomanoWolf,
        )?,
        bootstrap_draws: reader.read_u64()?,
        bootstrap_strategies: reader.read_u64()?,
        bootstrap_periods: reader.read_u64()?,
    };
    AdmissionStatisticsFieldsV3::verify_detached(&values)
        .map_err(AdmissionCanonicalRefusalV3::Statistics)
}

fn read_walk_forward_v3(
    reader: &mut CanonicalReaderV3<'_>,
) -> Result<AnchoredWalkForwardAuthorityV3, AdmissionCanonicalRefusalV3> {
    AnchoredWalkForwardAuthorityV3::from_canonical_fields(AnchoredWalkForwardAuthorityV3 {
        validation_policy_digest: reader.take::<32>()?,
        validation_family_digest: reader.take::<32>()?,
        walk_facts_digest: reader.take::<32>()?,
        authority_id: reader.take::<32>()?,
        fold_count: reader.read_u64()?,
        decided_folds: reader.read_u64()?,
        profitable_oos_folds: reader.read_u64()?,
        aggregate_oos_paisa: reader.read_i64()?,
    })
    .map_err(AdmissionCanonicalRefusalV3::WalkForward)
}

fn decode_evidence_v3(bytes: &[u8]) -> Result<AdmissionEvidenceV3, AdmissionCanonicalRefusalV3> {
    let mut reader = CanonicalReaderV3::new(
        bytes,
        AdmissionCanonicalRecordV3::Evidence,
        ADMISSION_EVIDENCE_CANONICAL_LEN_V3,
        EVIDENCE_DOMAIN_V1,
        EVIDENCE_PAYLOAD_LEN_V3,
    )?;
    let decoded_values = read_evidence_values_v3(&mut reader)?;
    let statistics = read_statistics_fields_v3(&mut reader)?;
    let walk_forward = read_walk_forward_v3(&mut reader)?;
    reader.finish();
    let mut base = decoded_values;
    clear_authority_slots_v2(&mut base);
    let evidence = AdmissionEvidenceV3::new(base, &statistics, walk_forward)
        .map_err(AdmissionCanonicalRefusalV3::Evidence)?;
    if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
        return Err(AdmissionCanonicalRefusalV3::AuthorityProjectionMismatch);
    }
    Ok(evidence)
}

fn decode_decision_v3(bytes: &[u8]) -> Result<AdmissionDecisionV3, AdmissionCanonicalRefusalV3> {
    let mut reader = CanonicalReaderV3::new(
        bytes,
        AdmissionCanonicalRecordV3::Decision,
        ADMISSION_DECISION_CANONICAL_LEN_V3,
        DECISION_DOMAIN_V1,
        DECISION_PAYLOAD_LEN_V3,
    )?;
    let policy = reader.take::<ADMISSION_POLICY_CANONICAL_LEN_V1>()?;
    let evidence = reader.take::<ADMISSION_EVIDENCE_CANONICAL_LEN_V3>()?;
    let verdict = reader.take::<ADMISSION_VERDICT_CANONICAL_LEN_V1>()?;
    reader.finish();
    AdmissionDecisionV3::from_canonical_parts(&policy, &evidence, &verdict)
}

// In-memory footprint assertions only. They protect the bounded comparison
// kernel from accidental heap-bearing growth. They are NOT a byte codec, ABI,
// durable schema or digest; the canonical codecs above are that boundary.
const _: [(); 304] = [(); core::mem::size_of::<AdmissionPolicyV1>()];
const _: [(); 608] = [(); core::mem::size_of::<AdmissionEvidenceV1>()];
const _: [(); 8] = [(); core::mem::size_of::<ReasonBits>()];
const _: [(); 40] = [(); core::mem::size_of::<AdmissionVerdictV1>()];

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "tests construct deliberately valid fixed-layout fixtures and must fail loudly on an exhaustive mismatch"
)]
mod tests {
    use super::{
        ADMISSION_DECISION_CANONICAL_LEN_V1, ADMISSION_DECISION_CANONICAL_LEN_V2,
        ADMISSION_DECISION_CANONICAL_LEN_V3, ADMISSION_EVIDENCE_CANONICAL_LEN_V1,
        ADMISSION_EVIDENCE_CANONICAL_LEN_V2, ADMISSION_EVIDENCE_CANONICAL_LEN_V3,
        ADMISSION_POLICY_CANONICAL_LEN_V1, ADMISSION_VERDICT_CANONICAL_LEN_V1,
        ADMISSION_VERSION_V1, ADMISSION_VERSION_V2, ADMISSION_VERSION_V3,
        AdmissionCanonicalRecordV1, AdmissionCanonicalRecordV2, AdmissionCanonicalRecordV3,
        AdmissionCanonicalRefusalV1, AdmissionCanonicalRefusalV2, AdmissionCanonicalRefusalV3,
        AdmissionDecisionSealV1, AdmissionDecisionV2, AdmissionDecisionV3,
        AdmissionEvidenceFieldV1, AdmissionEvidenceRefusalKindV1, AdmissionEvidenceV1,
        AdmissionEvidenceV2, AdmissionEvidenceV3, AdmissionEvidenceValuesV1,
        AdmissionExactProbabilityV2, AdmissionFieldV1, AdmissionPolicyDraftV1,
        AdmissionPolicyRefusalKindV1, AdmissionPolicyV1, AdmissionPolicyValuesV1,
        AdmissionReasonV1, AdmissionStatisticsDraftV2, AdmissionStatisticsDraftV3,
        AdmissionStatisticsFieldsV2, AdmissionStatisticsFieldsV3, AdmissionStatisticsIdentityV3,
        AdmissionStatisticsRefusalV2, AdmissionStatisticsRefusalV3, AdmissionStatusV1,
        AdmissionV2ArithmeticProjection, AdmissionV3ArithmeticProjection,
        AdmissionVerdictPartitionV1, AdmissionVerdictV1, AnchoredWalkForwardAuthorityV2,
        AnchoredWalkForwardAuthorityV3, AnchoredWalkForwardRefusalV2, AnchoredWalkForwardRefusalV3,
        CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1, PPM, ReasonBits,
        canonical_wilson_projection_v2, hypothesis_decision_v2,
    };
    use costs::fill::Direction;
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use std::sync::OnceLock;

    use crate::Sweeper;
    use crate::outcome::Horizon;
    use crate::validate::{
        AnchoredAdmissionValidationV2, AnchoredSearchValidationV3, DEFAULT_RUNGS, ExecutionSeries,
        walk_forward_projected_prepared_anchored_admission_v2,
        walk_forward_projected_prepared_anchored_search_v3,
    };

    type PolicySetter = fn(&mut AdmissionPolicyDraftV1);
    type PolicyValueSetter = fn(&mut AdmissionPolicyValuesV1);
    type EvidenceSetter = fn(&mut AdmissionEvidenceValuesV1);
    type VerdictSetter = fn(&mut AdmissionVerdictV1);

    fn draft() -> AdmissionPolicyDraftV1 {
        AdmissionPolicyDraftV1 {
            min_support_hits: Some(100),
            min_independent_sessions: Some(20),
            min_trades: Some(30),
            max_mae_paisa: Some(500),
            min_worst_reward_risk_ppm: Some(2_000_000),
            min_win_rate_ppm: Some(600_000),
            min_wilson_win_rate_ppm: Some(550_000),
            min_return_drawdown_ppm: Some(3_000_000),
            min_weakest_period_return_paisa: Some(0),
            max_pbo_ppm: Some(100_000),
            max_fwer_p_value_ppm: Some(50_000),
            max_spa_p_value_ppm: Some(50_000),
            min_decided_folds: Some(10),
            max_ambiguous_fill_rate_ppm: Some(100_000),
            max_gap_affected_rate_ppm: Some(100_000),
            max_session_concentration_ppm: Some(300_000),
            max_largest_trade_profit_share_ppm: Some(200_000),
            max_drawdown_paisa: Some(10_000),
            max_worst_trade_loss_paisa: Some(2_000),
            max_losing_trade_rate_ppm: Some(400_000),
            max_losing_trades: Some(20),
            min_pessimistic_profit_paisa: Some(1_000),
            min_winning_trades: Some(30),
            min_average_win_paisa: Some(300),
            max_average_loss_paisa: Some(150),
            min_profit_factor_ppm: Some(2_000_000),
            max_consecutive_losing_streak: Some(3),
            min_consecutive_winning_streak: Some(3),
            min_bootstrap_draws: Some(1_000),
            min_bootstrap_strategies: Some(100),
            min_bootstrap_periods: Some(300),
            min_pbo_contributing_folds: Some(10),
            max_pbo_unrankable_folds: Some(2),
            min_profitable_oos_folds: Some(8),
            min_oos_pessimistic_return_paisa: Some(1_000),
            max_white_reality_p_value_ppm: Some(50_000),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(50_000),
            require_romano_wolf_rejection: Some(true),
        }
    }

    fn policy() -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(draft()).expect("complete policy is valid")
    }

    fn policy_for_failure(reason: AdmissionReasonV1) -> AdmissionPolicyV1 {
        let mut values = draft();
        match reason {
            AdmissionReasonV1::Trades => values.min_winning_trades = Some(0),
            AdmissionReasonV1::WinRate => {
                values.min_winning_trades = Some(0);
                values.max_losing_trades = Some(u64::MAX);
                values.max_losing_trade_rate_ppm = Some(PPM);
            }
            AdmissionReasonV1::LosingTradeRate => {
                values.min_win_rate_ppm = Some(0);
                values.min_winning_trades = Some(0);
                values.max_losing_trades = Some(u64::MAX);
            }
            AdmissionReasonV1::LosingTrades => {
                values.min_win_rate_ppm = Some(0);
                values.max_losing_trade_rate_ppm = Some(PPM);
            }
            _ => {}
        }
        AdmissionPolicyV1::new(values).expect("reason-specific policy remains valid")
    }

    const fn trade_linked(reason: AdmissionReasonV1) -> bool {
        matches!(
            reason,
            AdmissionReasonV1::Trades
                | AdmissionReasonV1::WinRate
                | AdmissionReasonV1::LosingTradeRate
                | AdmissionReasonV1::LosingTrades
                | AdmissionReasonV1::WinningTrades
        )
    }

    fn fail_all_trade_metrics(values: &mut AdmissionEvidenceValuesV1) {
        values.trades = ObservedU64V1::Measured(29);
        values.winning_trades = ObservedU64V1::Measured(8);
        values.losing_trades = ObservedU64V1::Measured(21);
        values.win_rate_ppm = ObservedU64V1::Measured(275_862);
        values.losing_trade_rate_ppm = ObservedU64V1::Measured(724_137);
    }

    fn passing_values() -> AdmissionEvidenceValuesV1 {
        // Every measured value is deliberately EXACTLY on its policy boundary.
        // The three measured PBO-named fields make this a comparator-only
        // hypothetical: public V1 construction refuses them because V1 has no
        // genuine CSCV authority. `evidence` below is the test module's private
        // way to exercise every policy branch; constructor/decoder tests use
        // `constructible_values` instead.
        AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(100),
            independent_sessions: ObservedU64V1::Measured(20),
            trades: ObservedU64V1::Measured(50),
            max_mae_paisa: ObservedU64V1::Measured(500),
            worst_reward_risk_ppm: ObservedU64V1::Measured(2_000_000),
            win_rate_ppm: ObservedU64V1::Measured(600_000),
            wilson_win_rate_ppm: ObservedU64V1::Measured(550_000),
            return_drawdown_ppm: ObservedU64V1::Measured(3_000_000),
            weakest_period_return_paisa: ObservedI64V1::Measured(0),
            pbo_ppm: ObservedU64V1::Measured(100_000),
            fwer_p_value_ppm: ObservedU64V1::Measured(50_000),
            spa_p_value_ppm: ObservedU64V1::Measured(50_000),
            decided_folds: ObservedU64V1::Measured(10),
            ambiguous_fill_rate_ppm: ObservedU64V1::Measured(100_000),
            gap_affected_rate_ppm: ObservedU64V1::Measured(100_000),
            session_concentration_ppm: ObservedU64V1::Measured(300_000),
            largest_trade_profit_share_ppm: ObservedU64V1::Measured(200_000),
            execution_complete: CompletenessV1::Complete,
            data_complete: CompletenessV1::Complete,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Measured(10_000),
            worst_trade_loss_paisa: ObservedU64V1::Measured(2_000),
            losing_trade_rate_ppm: ObservedU64V1::Measured(400_000),
            losing_trades: ObservedU64V1::Measured(20),
            pessimistic_profit_paisa: ObservedI64V1::Measured(1_000),
            winning_trades: ObservedU64V1::Measured(30),
            average_win_paisa: ObservedU64V1::Measured(300),
            average_loss_paisa: ObservedU64V1::Measured(150),
            profit_factor_ppm: ObservedU64V1::Measured(2_000_000),
            consecutive_losing_streak: ObservedU64V1::Measured(3),
            consecutive_winning_streak: ObservedU64V1::Measured(3),
            bootstrap_draws: ObservedU64V1::Measured(1_000),
            bootstrap_strategies: ObservedU64V1::Measured(100),
            bootstrap_periods: ObservedU64V1::Measured(300),
            pbo_contributing_folds: ObservedU64V1::Measured(10),
            pbo_unrankable_folds: ObservedU64V1::Measured(2),
            profitable_oos_folds: ObservedU64V1::Measured(8),
            oos_pessimistic_return_paisa: ObservedI64V1::Measured(1_000),
            white_reality_p_value_ppm: ObservedU64V1::Measured(50_000),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(50_000),
            white_reality_decision: HypothesisDecisionV1::RejectedNull,
            romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
            full_precision_statistics_complete: CompletenessV1::Complete,
        }
    }

    fn v2_base_values() -> AdmissionEvidenceValuesV1 {
        let mut values = passing_values();
        values.wilson_win_rate_ppm = ObservedU64V1::Unmeasured;
        values.pbo_ppm = ObservedU64V1::Unmeasured;
        values.fwer_p_value_ppm = ObservedU64V1::Unmeasured;
        values.spa_p_value_ppm = ObservedU64V1::Unmeasured;
        values.decided_folds = ObservedU64V1::Unmeasured;
        values.bootstrap_draws = ObservedU64V1::Unmeasured;
        values.bootstrap_strategies = ObservedU64V1::Unmeasured;
        values.bootstrap_periods = ObservedU64V1::Unmeasured;
        values.pbo_contributing_folds = ObservedU64V1::Unmeasured;
        values.pbo_unrankable_folds = ObservedU64V1::Unmeasured;
        values.profitable_oos_folds = ObservedU64V1::Unmeasured;
        values.oos_pessimistic_return_paisa = ObservedI64V1::Unmeasured;
        values.white_reality_p_value_ppm = ObservedU64V1::Unmeasured;
        values.romano_wolf_p_value_ppm = ObservedU64V1::Unmeasured;
        values.white_reality_decision = HypothesisDecisionV1::Unmeasured;
        values.romano_wolf_decision = HypothesisDecisionV1::Unmeasured;
        values.full_precision_statistics_complete = CompletenessV1::Unmeasured;
        values
    }

    fn v2_policy() -> AdmissionPolicyV1 {
        let mut values = draft();
        values.min_wilson_win_rate_ppm = Some(0);
        values.min_decided_folds = Some(1);
        values.min_profitable_oos_folds = Some(1);
        values.min_oos_pessimistic_return_paisa = Some(i64::MIN);
        AdmissionPolicyV1::new(values).expect("V2 fixture policy is valid")
    }

    fn evaluator_v2() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn anchored_v2() -> AnchoredAdmissionValidationV2 {
        static FIXTURE: OnceLock<AnchoredAdmissionValidationV2> = OnceLock::new();
        FIXTURE
            .get_or_init(|| {
                let bars = crate::synthetic::sessions(12);
                let sweeper = Sweeper::new(Ladder::with_min_hits(1_200).with_ceiling(20_000));
                let mut builder = |slice: &[indicators::Candle]| {
                    let mut evaluator = evaluator_v2();
                    Ok(Column::build(slice, &mut evaluator))
                };
                walk_forward_projected_prepared_anchored_admission_v2(
                    &bars,
                    ExecutionSeries {
                        bars: &bars,
                        signal_length_micros: 60_000_000,
                    },
                    Horizon::bars(15).expect("fixture horizon is nonzero"),
                    3,
                    Direction::Long,
                    &sweeper,
                    &mut builder,
                    DEFAULT_RUNGS,
                )
                .expect("genuine anchored fixture issues opaque provenance")
            })
            .clone()
    }

    fn anchored_v3() -> AnchoredSearchValidationV3 {
        static FIXTURE: OnceLock<AnchoredSearchValidationV3> = OnceLock::new();
        FIXTURE
            .get_or_init(|| {
                let bars = crate::synthetic::sessions(12);
                let sweeper = Sweeper::new(Ladder::with_min_hits(1_200).with_ceiling(20_000));
                let mut builder = |slice: &[indicators::Candle]| {
                    let mut evaluator = evaluator_v2();
                    Ok(Column::build(slice, &mut evaluator))
                };
                walk_forward_projected_prepared_anchored_search_v3(
                    &bars,
                    ExecutionSeries {
                        bars: &bars,
                        signal_length_micros: 60_000_000,
                    },
                    Horizon::bars(15).expect("fixture horizon is nonzero"),
                    3,
                    &sweeper,
                    &mut builder,
                    DEFAULT_RUNGS,
                )
                .expect("genuine bilateral fixture issues opaque V3 provenance")
            })
            .clone()
    }

    fn statistics_v2(candidate_tag: u8) -> AdmissionStatisticsDraftV2 {
        let (wilson_lower_bits, wilson_win_rate_ppm) = canonical_wilson_projection_v2(30, 50);
        AdmissionStatisticsDraftV2 {
            candidate_semantic_id: [candidate_tag; 32],
            population_search_id: [2; 32],
            ranking_validation_policy_digest: [13; 32],
            source_policy_digest: [3; 32],
            finalization_family_digest: [4; 32],
            statistics_audit_id: [5; 32],
            statistics_completion_digest: [6; 32],
            observation_statistics_link_id: [7; 32],
            cscv_policy_digest: [8; 32],
            cscv_split_family_digest: [9; 32],
            white_family_digest: [10; 32],
            spa_family_digest: [11; 32],
            romano_wolf_family_digest: [12; 32],
            trades: 50,
            wins: 30,
            wilson_lower_bits,
            wilson_win_rate_ppm,
            cscv_split_count: 12,
            pbo_contributing_splits: 10,
            pbo_unrankable_splits: 2,
            pbo_probability: AdmissionExactProbabilityV2::new(1, 10).expect("fixture PBO is exact"),
            white_probability: AdmissionExactProbabilityV2::new(50, 1_001)
                .expect("fixture White probability is exact"),
            spa_probability: AdmissionExactProbabilityV2::new(49, 1_001)
                .expect("fixture SPA probability is exact"),
            familywise_romano_wolf_probability: AdmissionExactProbabilityV2::new(49, 1_001)
                .expect("fixture family probability is exact"),
            candidate_romano_wolf_probability: AdmissionExactProbabilityV2::new(50, 1_001)
                .expect("fixture candidate probability is exact"),
            bootstrap_draws: 1_000,
            bootstrap_strategies: 100,
            bootstrap_periods: 300,
        }
    }

    fn statistics_v3(candidate_tag: u8) -> AdmissionStatisticsDraftV3 {
        let legacy = statistics_v2(candidate_tag);
        AdmissionStatisticsDraftV3 {
            candidate_semantic_id: legacy.candidate_semantic_id,
            statistics_audit_id: legacy.statistics_audit_id,
            statistics_completion_digest: legacy.statistics_completion_digest,
            observation_statistics_link_id: legacy.observation_statistics_link_id,
            cscv_policy_digest: legacy.cscv_policy_digest,
            cscv_split_family_digest: legacy.cscv_split_family_digest,
            white_family_digest: legacy.white_family_digest,
            spa_family_digest: legacy.spa_family_digest,
            romano_wolf_family_digest: legacy.romano_wolf_family_digest,
            trades: legacy.trades,
            wins: legacy.wins,
            wilson_lower_bits: legacy.wilson_lower_bits,
            wilson_win_rate_ppm: legacy.wilson_win_rate_ppm,
            cscv_split_count: legacy.cscv_split_count,
            pbo_contributing_splits: legacy.pbo_contributing_splits,
            pbo_unrankable_splits: legacy.pbo_unrankable_splits,
            pbo_probability: legacy.pbo_probability,
            white_probability: legacy.white_probability,
            spa_probability: legacy.spa_probability,
            familywise_romano_wolf_probability: legacy.familywise_romano_wolf_probability,
            candidate_romano_wolf_probability: legacy.candidate_romano_wolf_probability,
            bootstrap_draws: legacy.bootstrap_draws,
            bootstrap_strategies: legacy.bootstrap_strategies,
            bootstrap_periods: legacy.bootstrap_periods,
        }
    }

    fn v2_parts() -> (
        AdmissionPolicyV1,
        AdmissionEvidenceValuesV1,
        AnchoredAdmissionValidationV2,
        AdmissionStatisticsDraftV2,
    ) {
        let anchored = anchored_v2();
        let statistics = statistics_v2(1);
        (v2_policy(), v2_base_values(), anchored, statistics)
    }

    fn v2_evidence(
        base: &AdmissionEvidenceValuesV1,
        anchored: &AnchoredAdmissionValidationV2,
        statistics: &AdmissionStatisticsDraftV2,
    ) -> AdmissionEvidenceV2 {
        let fields = AdmissionStatisticsFieldsV2::verify_detached(statistics)
            .expect("fixture statistics reconcile");
        let walk_forward =
            AnchoredWalkForwardAuthorityV2::from_opaque(anchored).expect("fixture walk reconciles");
        AdmissionEvidenceV2::new(*base, &fields, walk_forward).expect("fixture authorities join")
    }

    fn v3_evidence(
        base: &AdmissionEvidenceValuesV1,
        anchored: &AnchoredSearchValidationV3,
        statistics: &AdmissionStatisticsDraftV3,
    ) -> AdmissionEvidenceV3 {
        let fields = AdmissionStatisticsFieldsV3::verify_detached(statistics)
            .expect("fixture V3 statistics reconcile");
        let walk_forward = AnchoredWalkForwardAuthorityV3::from_opaque(anchored)
            .expect("fixture bilateral V3 walk reconciles");
        AdmissionEvidenceV3::new(*base, &fields, walk_forward).expect("fixture V3 authorities join")
    }

    fn constructible_values() -> AdmissionEvidenceValuesV1 {
        let mut values = passing_values();
        values.pbo_ppm = ObservedU64V1::Unmeasured;
        values.pbo_contributing_folds = ObservedU64V1::Unmeasured;
        values.pbo_unrankable_folds = ObservedU64V1::Unmeasured;
        values
    }

    fn constructible_evidence() -> AdmissionEvidenceV1 {
        AdmissionEvidenceV1::new(constructible_values())
            .expect("explicitly non-authoritative PBO states are valid V1 evidence")
    }

    fn evidence(values: &AdmissionEvidenceValuesV1) -> AdmissionEvidenceV1 {
        // Policy comparison remains testable against a hypothetical future
        // fully sourced PBO value, but no production caller can take this path:
        // `values` is private and the public constructor/decoder reject it.
        AdmissionEvidenceV1 { values: *values }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive test fixture keeps every stable reason beside the one evidence mutation intended to trigger it"
    )]
    fn fail(values: &mut AdmissionEvidenceValuesV1, reason: AdmissionReasonV1) {
        match reason {
            AdmissionReasonV1::Support => values.support_hits = ObservedU64V1::Measured(99),
            AdmissionReasonV1::SessionIndependence => {
                values.independent_sessions = ObservedU64V1::Measured(19);
            }
            AdmissionReasonV1::Trades => {
                values.trades = ObservedU64V1::Measured(29);
                values.winning_trades = ObservedU64V1::Measured(18);
                values.losing_trades = ObservedU64V1::Measured(11);
                values.win_rate_ppm = ObservedU64V1::Measured(620_689);
                values.losing_trade_rate_ppm = ObservedU64V1::Measured(379_310);
            }
            AdmissionReasonV1::MaxMae => {
                values.max_mae_paisa = ObservedU64V1::Measured(501);
            }
            AdmissionReasonV1::WorstRewardRisk => {
                values.worst_reward_risk_ppm = ObservedU64V1::Measured(1_999_999);
            }
            AdmissionReasonV1::WinRate | AdmissionReasonV1::LosingTradeRate => {
                values.winning_trades = ObservedU64V1::Measured(29);
                values.losing_trades = ObservedU64V1::Measured(21);
                values.win_rate_ppm = ObservedU64V1::Measured(580_000);
                values.losing_trade_rate_ppm = ObservedU64V1::Measured(420_000);
            }
            AdmissionReasonV1::WilsonWinRate => {
                values.wilson_win_rate_ppm = ObservedU64V1::Measured(549_999);
            }
            AdmissionReasonV1::ReturnDrawdown => {
                values.return_drawdown_ppm = ObservedU64V1::Measured(2_999_999);
            }
            AdmissionReasonV1::WeakestPeriod => {
                values.weakest_period_return_paisa = ObservedI64V1::Measured(-1);
            }
            AdmissionReasonV1::Pbo => values.pbo_ppm = ObservedU64V1::Measured(100_001),
            AdmissionReasonV1::Fwer => {
                values.fwer_p_value_ppm = ObservedU64V1::Measured(50_001);
            }
            AdmissionReasonV1::Spa => {
                values.spa_p_value_ppm = ObservedU64V1::Measured(50_001);
            }
            AdmissionReasonV1::DecidedFolds => {
                values.decided_folds = ObservedU64V1::Measured(8);
            }
            AdmissionReasonV1::AmbiguousFills => {
                values.ambiguous_fill_rate_ppm = ObservedU64V1::Measured(100_001);
            }
            AdmissionReasonV1::GapAffected => {
                values.gap_affected_rate_ppm = ObservedU64V1::Measured(100_001);
            }
            AdmissionReasonV1::SessionConcentration => {
                values.session_concentration_ppm = ObservedU64V1::Measured(300_001);
            }
            AdmissionReasonV1::LargestTradeProfitShare => {
                values.largest_trade_profit_share_ppm = ObservedU64V1::Measured(200_001);
            }
            AdmissionReasonV1::ExecutionCompleteness => {
                values.execution_complete = CompletenessV1::Incomplete;
            }
            AdmissionReasonV1::DataCompleteness => {
                values.data_complete = CompletenessV1::Incomplete;
            }
            AdmissionReasonV1::CalendarCompleteness => {
                values.calendar_complete = CompletenessV1::Incomplete;
            }
            AdmissionReasonV1::PopulationCompleteness => {
                values.population_complete = CompletenessV1::Incomplete;
            }
            AdmissionReasonV1::Drawdown => {
                values.drawdown_paisa = ObservedU64V1::Measured(10_001);
            }
            AdmissionReasonV1::WorstTradeLoss => {
                values.worst_trade_loss_paisa = ObservedU64V1::Measured(2_001);
            }
            AdmissionReasonV1::LosingTrades => {
                values.trades = ObservedU64V1::Measured(51);
                values.winning_trades = ObservedU64V1::Measured(30);
                values.losing_trades = ObservedU64V1::Measured(21);
                values.win_rate_ppm = ObservedU64V1::Measured(588_235);
                values.losing_trade_rate_ppm = ObservedU64V1::Measured(411_764);
            }
            AdmissionReasonV1::PessimisticProfit => {
                values.pessimistic_profit_paisa = ObservedI64V1::Measured(999);
            }
            AdmissionReasonV1::WinningTrades => {
                values.winning_trades = ObservedU64V1::Measured(29);
                values.losing_trades = ObservedU64V1::Measured(19);
                values.trades = ObservedU64V1::Measured(48);
                values.win_rate_ppm = ObservedU64V1::Measured(604_166);
                values.losing_trade_rate_ppm = ObservedU64V1::Measured(395_833);
            }
            AdmissionReasonV1::AverageWin => {
                values.average_win_paisa = ObservedU64V1::Measured(299);
            }
            AdmissionReasonV1::AverageLoss => {
                values.average_loss_paisa = ObservedU64V1::Measured(151);
            }
            AdmissionReasonV1::ProfitFactor => {
                values.profit_factor_ppm = ObservedU64V1::Measured(1_999_999);
            }
            AdmissionReasonV1::ConsecutiveLosingStreak => {
                values.consecutive_losing_streak = ObservedU64V1::Measured(4);
            }
            AdmissionReasonV1::ConsecutiveWinningStreak => {
                values.consecutive_winning_streak = ObservedU64V1::Measured(2);
            }
            AdmissionReasonV1::BootstrapDraws => {
                values.bootstrap_draws = ObservedU64V1::Measured(999);
            }
            AdmissionReasonV1::BootstrapStrategies => {
                values.bootstrap_strategies = ObservedU64V1::Measured(99);
            }
            AdmissionReasonV1::BootstrapPeriods => {
                values.bootstrap_periods = ObservedU64V1::Measured(299);
            }
            AdmissionReasonV1::PboContributingFolds => {
                values.pbo_contributing_folds = ObservedU64V1::Measured(9);
            }
            AdmissionReasonV1::PboUnrankableFolds => {
                values.pbo_unrankable_folds = ObservedU64V1::Measured(3);
            }
            AdmissionReasonV1::ProfitableOosFolds => {
                values.profitable_oos_folds = ObservedU64V1::Measured(7);
            }
            AdmissionReasonV1::OosPessimisticReturn => {
                values.oos_pessimistic_return_paisa = ObservedI64V1::Measured(999);
            }
            AdmissionReasonV1::WhiteRealityPValue => {
                values.white_reality_p_value_ppm = ObservedU64V1::Measured(50_001);
            }
            AdmissionReasonV1::WhiteRealityDecision => {
                values.white_reality_decision = HypothesisDecisionV1::DidNotReject;
            }
            AdmissionReasonV1::RomanoWolfPValue => {
                values.romano_wolf_p_value_ppm = ObservedU64V1::Measured(50_001);
            }
            AdmissionReasonV1::RomanoWolfDecision => {
                values.romano_wolf_decision = HypothesisDecisionV1::DidNotReject;
            }
            AdmissionReasonV1::FullPrecisionStatisticsCompleteness => {
                values.full_precision_statistics_complete = CompletenessV1::Incomplete;
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive test fixture maps every stable reason to its explicit unmeasured state"
    )]
    fn set_unmeasured(values: &mut AdmissionEvidenceValuesV1, reason: AdmissionReasonV1) {
        match reason {
            AdmissionReasonV1::Support => values.support_hits = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::SessionIndependence => {
                values.independent_sessions = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::Trades => values.trades = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::MaxMae => values.max_mae_paisa = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::WorstRewardRisk => {
                values.worst_reward_risk_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::WinRate => values.win_rate_ppm = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::WilsonWinRate => {
                values.wilson_win_rate_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::ReturnDrawdown => {
                values.return_drawdown_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::WeakestPeriod => {
                values.weakest_period_return_paisa = ObservedI64V1::Unmeasured;
            }
            AdmissionReasonV1::Pbo => values.pbo_ppm = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::Fwer => values.fwer_p_value_ppm = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::Spa => values.spa_p_value_ppm = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::DecidedFolds => {
                values.decided_folds = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::AmbiguousFills => {
                values.ambiguous_fill_rate_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::GapAffected => {
                values.gap_affected_rate_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::SessionConcentration => {
                values.session_concentration_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::LargestTradeProfitShare => {
                values.largest_trade_profit_share_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::ExecutionCompleteness => {
                values.execution_complete = CompletenessV1::Unmeasured;
            }
            AdmissionReasonV1::DataCompleteness => {
                values.data_complete = CompletenessV1::Unmeasured;
            }
            AdmissionReasonV1::CalendarCompleteness => {
                values.calendar_complete = CompletenessV1::Unmeasured;
            }
            AdmissionReasonV1::PopulationCompleteness => {
                values.population_complete = CompletenessV1::Unmeasured;
            }
            AdmissionReasonV1::Drawdown => values.drawdown_paisa = ObservedU64V1::Unmeasured,
            AdmissionReasonV1::WorstTradeLoss => {
                values.worst_trade_loss_paisa = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::LosingTradeRate => {
                values.losing_trade_rate_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::LosingTrades => {
                values.losing_trades = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::PessimisticProfit => {
                values.pessimistic_profit_paisa = ObservedI64V1::Unmeasured;
            }
            AdmissionReasonV1::WinningTrades => {
                values.winning_trades = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::AverageWin => {
                values.average_win_paisa = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::AverageLoss => {
                values.average_loss_paisa = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::ProfitFactor => {
                values.profit_factor_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::ConsecutiveLosingStreak => {
                values.consecutive_losing_streak = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::ConsecutiveWinningStreak => {
                values.consecutive_winning_streak = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::BootstrapDraws => {
                values.bootstrap_draws = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::BootstrapStrategies => {
                values.bootstrap_strategies = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::BootstrapPeriods => {
                values.bootstrap_periods = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::PboContributingFolds => {
                values.pbo_contributing_folds = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::PboUnrankableFolds => {
                values.pbo_unrankable_folds = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::ProfitableOosFolds => {
                values.profitable_oos_folds = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::OosPessimisticReturn => {
                values.oos_pessimistic_return_paisa = ObservedI64V1::Unmeasured;
            }
            AdmissionReasonV1::WhiteRealityPValue => {
                values.white_reality_p_value_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::WhiteRealityDecision => {
                values.white_reality_decision = HypothesisDecisionV1::Unmeasured;
            }
            AdmissionReasonV1::RomanoWolfPValue => {
                values.romano_wolf_p_value_ppm = ObservedU64V1::Unmeasured;
            }
            AdmissionReasonV1::RomanoWolfDecision => {
                values.romano_wolf_decision = HypothesisDecisionV1::Unmeasured;
            }
            AdmissionReasonV1::FullPrecisionStatisticsCompleteness => {
                values.full_precision_statistics_complete = CompletenessV1::Unmeasured;
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive test fixture maps every stable reason to its explicit upstream-refused state"
    )]
    fn set_refused(values: &mut AdmissionEvidenceValuesV1, reason: AdmissionReasonV1) {
        match reason {
            AdmissionReasonV1::Support => values.support_hits = ObservedU64V1::Refused,
            AdmissionReasonV1::SessionIndependence => {
                values.independent_sessions = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::Trades => values.trades = ObservedU64V1::Refused,
            AdmissionReasonV1::MaxMae => values.max_mae_paisa = ObservedU64V1::Refused,
            AdmissionReasonV1::WorstRewardRisk => {
                values.worst_reward_risk_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::WinRate => values.win_rate_ppm = ObservedU64V1::Refused,
            AdmissionReasonV1::WilsonWinRate => {
                values.wilson_win_rate_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::ReturnDrawdown => {
                values.return_drawdown_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::WeakestPeriod => {
                values.weakest_period_return_paisa = ObservedI64V1::Refused;
            }
            AdmissionReasonV1::Pbo => values.pbo_ppm = ObservedU64V1::Refused,
            AdmissionReasonV1::Fwer => values.fwer_p_value_ppm = ObservedU64V1::Refused,
            AdmissionReasonV1::Spa => values.spa_p_value_ppm = ObservedU64V1::Refused,
            AdmissionReasonV1::DecidedFolds => values.decided_folds = ObservedU64V1::Refused,
            AdmissionReasonV1::AmbiguousFills => {
                values.ambiguous_fill_rate_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::GapAffected => {
                values.gap_affected_rate_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::SessionConcentration => {
                values.session_concentration_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::LargestTradeProfitShare => {
                values.largest_trade_profit_share_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::ExecutionCompleteness => {
                values.execution_complete = CompletenessV1::Refused;
            }
            AdmissionReasonV1::DataCompleteness => {
                values.data_complete = CompletenessV1::Refused;
            }
            AdmissionReasonV1::CalendarCompleteness => {
                values.calendar_complete = CompletenessV1::Refused;
            }
            AdmissionReasonV1::PopulationCompleteness => {
                values.population_complete = CompletenessV1::Refused;
            }
            AdmissionReasonV1::Drawdown => values.drawdown_paisa = ObservedU64V1::Refused,
            AdmissionReasonV1::WorstTradeLoss => {
                values.worst_trade_loss_paisa = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::LosingTradeRate => {
                values.losing_trade_rate_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::LosingTrades => {
                values.losing_trades = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::PessimisticProfit => {
                values.pessimistic_profit_paisa = ObservedI64V1::Refused;
            }
            AdmissionReasonV1::WinningTrades => {
                values.winning_trades = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::AverageWin => {
                values.average_win_paisa = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::AverageLoss => {
                values.average_loss_paisa = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::ProfitFactor => {
                values.profit_factor_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::ConsecutiveLosingStreak => {
                values.consecutive_losing_streak = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::ConsecutiveWinningStreak => {
                values.consecutive_winning_streak = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::BootstrapDraws => {
                values.bootstrap_draws = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::BootstrapStrategies => {
                values.bootstrap_strategies = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::BootstrapPeriods => {
                values.bootstrap_periods = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::PboContributingFolds => {
                values.pbo_contributing_folds = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::PboUnrankableFolds => {
                values.pbo_unrankable_folds = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::ProfitableOosFolds => {
                values.profitable_oos_folds = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::OosPessimisticReturn => {
                values.oos_pessimistic_return_paisa = ObservedI64V1::Refused;
            }
            AdmissionReasonV1::WhiteRealityPValue => {
                values.white_reality_p_value_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::WhiteRealityDecision => {
                values.white_reality_decision = HypothesisDecisionV1::Refused;
            }
            AdmissionReasonV1::RomanoWolfPValue => {
                values.romano_wolf_p_value_ppm = ObservedU64V1::Refused;
            }
            AdmissionReasonV1::RomanoWolfDecision => {
                values.romano_wolf_decision = HypothesisDecisionV1::Refused;
            }
            AdmissionReasonV1::FullPrecisionStatisticsCompleteness => {
                values.full_precision_statistics_complete = CompletenessV1::Refused;
            }
        }
    }

    fn clear(draft: &mut AdmissionPolicyDraftV1, field: AdmissionFieldV1) {
        match field {
            AdmissionFieldV1::MinSupportHits => draft.min_support_hits = None,
            AdmissionFieldV1::MinIndependentSessions => draft.min_independent_sessions = None,
            AdmissionFieldV1::MinTrades => draft.min_trades = None,
            AdmissionFieldV1::MaxMaePaisa => draft.max_mae_paisa = None,
            AdmissionFieldV1::MinWorstRewardRiskPpm => {
                draft.min_worst_reward_risk_ppm = None;
            }
            AdmissionFieldV1::MinWinRatePpm => draft.min_win_rate_ppm = None,
            AdmissionFieldV1::MinWilsonWinRatePpm => draft.min_wilson_win_rate_ppm = None,
            AdmissionFieldV1::MinReturnDrawdownPpm => {
                draft.min_return_drawdown_ppm = None;
            }
            AdmissionFieldV1::MinWeakestPeriodReturnPaisa => {
                draft.min_weakest_period_return_paisa = None;
            }
            AdmissionFieldV1::MaxPboPpm => draft.max_pbo_ppm = None,
            AdmissionFieldV1::MaxFwerPValuePpm => draft.max_fwer_p_value_ppm = None,
            AdmissionFieldV1::MaxSpaPValuePpm => draft.max_spa_p_value_ppm = None,
            AdmissionFieldV1::MinDecidedFolds => draft.min_decided_folds = None,
            AdmissionFieldV1::MaxAmbiguousFillRatePpm => {
                draft.max_ambiguous_fill_rate_ppm = None;
            }
            AdmissionFieldV1::MaxGapAffectedRatePpm => {
                draft.max_gap_affected_rate_ppm = None;
            }
            AdmissionFieldV1::MaxSessionConcentrationPpm => {
                draft.max_session_concentration_ppm = None;
            }
            AdmissionFieldV1::MaxLargestTradeProfitSharePpm => {
                draft.max_largest_trade_profit_share_ppm = None;
            }
            AdmissionFieldV1::MaxDrawdownPaisa => draft.max_drawdown_paisa = None,
            AdmissionFieldV1::MaxWorstTradeLossPaisa => draft.max_worst_trade_loss_paisa = None,
            AdmissionFieldV1::MaxLosingTradeRatePpm => draft.max_losing_trade_rate_ppm = None,
            AdmissionFieldV1::MaxLosingTrades => draft.max_losing_trades = None,
            AdmissionFieldV1::MinPessimisticProfitPaisa => {
                draft.min_pessimistic_profit_paisa = None;
            }
            AdmissionFieldV1::MinWinningTrades => draft.min_winning_trades = None,
            AdmissionFieldV1::MinAverageWinPaisa => draft.min_average_win_paisa = None,
            AdmissionFieldV1::MaxAverageLossPaisa => draft.max_average_loss_paisa = None,
            AdmissionFieldV1::MinProfitFactorPpm => draft.min_profit_factor_ppm = None,
            AdmissionFieldV1::MaxConsecutiveLosingStreak => {
                draft.max_consecutive_losing_streak = None;
            }
            AdmissionFieldV1::MinConsecutiveWinningStreak => {
                draft.min_consecutive_winning_streak = None;
            }
            AdmissionFieldV1::MinBootstrapDraws => draft.min_bootstrap_draws = None,
            AdmissionFieldV1::MinBootstrapStrategies => draft.min_bootstrap_strategies = None,
            AdmissionFieldV1::MinBootstrapPeriods => draft.min_bootstrap_periods = None,
            AdmissionFieldV1::MinPboContributingFolds => {
                draft.min_pbo_contributing_folds = None;
            }
            AdmissionFieldV1::MaxPboUnrankableFolds => {
                draft.max_pbo_unrankable_folds = None;
            }
            AdmissionFieldV1::MinProfitableOosFolds => {
                draft.min_profitable_oos_folds = None;
            }
            AdmissionFieldV1::MinOosPessimisticReturnPaisa => {
                draft.min_oos_pessimistic_return_paisa = None;
            }
            AdmissionFieldV1::MaxWhiteRealityPValuePpm => {
                draft.max_white_reality_p_value_ppm = None;
            }
            AdmissionFieldV1::RequireWhiteRealityRejection => {
                draft.require_white_reality_rejection = None;
            }
            AdmissionFieldV1::MaxRomanoWolfPValuePpm => {
                draft.max_romano_wolf_p_value_ppm = None;
            }
            AdmissionFieldV1::RequireRomanoWolfRejection => {
                draft.require_romano_wolf_rejection = None;
            }
        }
    }

    #[test]
    fn exact_boundaries_pass_and_admission_is_exactly_an_empty_reason_set() {
        let verdict = policy().evaluate(&evidence(&passing_values()));
        assert_eq!(verdict.status(), AdmissionStatusV1::Admitted);
        assert!(verdict.is_admitted());
        assert_eq!(verdict.reasons(), ReasonBits::EMPTY);
        assert!(verdict.reconciles());
    }

    #[test]
    fn every_absent_runtime_parameter_is_refused_by_name() {
        for field in AdmissionFieldV1::ALL {
            let mut incomplete = draft();
            clear(&mut incomplete, field);
            let refusal = AdmissionPolicyV1::new(incomplete).expect_err("missing must refuse");
            assert_eq!(refusal.field, field);
            assert_eq!(refusal.kind, AdmissionPolicyRefusalKindV1::Absent);
        }
    }

    #[test]
    fn zero_count_floors_and_only_count_floors_are_refused() {
        let setters: [fn(&mut AdmissionPolicyDraftV1); 9] = [
            |d| d.min_support_hits = Some(0),
            |d| d.min_independent_sessions = Some(0),
            |d| d.min_trades = Some(0),
            |d| d.min_decided_folds = Some(0),
            |d| d.min_bootstrap_draws = Some(0),
            |d| d.min_bootstrap_strategies = Some(0),
            |d| d.min_bootstrap_periods = Some(0),
            |d| d.min_pbo_contributing_folds = Some(0),
            |d| d.min_profitable_oos_folds = Some(0),
        ];
        let fields = [
            AdmissionFieldV1::MinSupportHits,
            AdmissionFieldV1::MinIndependentSessions,
            AdmissionFieldV1::MinTrades,
            AdmissionFieldV1::MinDecidedFolds,
            AdmissionFieldV1::MinBootstrapDraws,
            AdmissionFieldV1::MinBootstrapStrategies,
            AdmissionFieldV1::MinBootstrapPeriods,
            AdmissionFieldV1::MinPboContributingFolds,
            AdmissionFieldV1::MinProfitableOosFolds,
        ];
        for (set, field) in setters.into_iter().zip(fields) {
            let mut invalid = draft();
            set(&mut invalid);
            let refusal = AdmissionPolicyV1::new(invalid).expect_err("zero floor must refuse");
            assert_eq!(refusal.field, field);
            assert_eq!(refusal.kind, AdmissionPolicyRefusalKindV1::MustBePositive);
        }

        let mut zeros_are_real_limits = draft();
        zeros_are_real_limits.max_mae_paisa = Some(0);
        zeros_are_real_limits.min_worst_reward_risk_ppm = Some(0);
        zeros_are_real_limits.min_return_drawdown_ppm = Some(0);
        zeros_are_real_limits.min_consecutive_winning_streak = Some(0);
        zeros_are_real_limits.max_pbo_unrankable_folds = Some(0);
        assert!(AdmissionPolicyV1::new(zeros_are_real_limits).is_ok());
    }

    #[test]
    fn every_bounded_policy_rate_refuses_above_one_million() {
        let setters: [(AdmissionFieldV1, PolicySetter); 12] = [
            (AdmissionFieldV1::MinWinRatePpm, |d| {
                d.min_win_rate_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MinWilsonWinRatePpm, |d| {
                d.min_wilson_win_rate_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxPboPpm, |d| {
                d.max_pbo_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxFwerPValuePpm, |d| {
                d.max_fwer_p_value_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxSpaPValuePpm, |d| {
                d.max_spa_p_value_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxAmbiguousFillRatePpm, |d| {
                d.max_ambiguous_fill_rate_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxGapAffectedRatePpm, |d| {
                d.max_gap_affected_rate_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxSessionConcentrationPpm, |d| {
                d.max_session_concentration_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxLargestTradeProfitSharePpm, |d| {
                d.max_largest_trade_profit_share_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxLosingTradeRatePpm, |d| {
                d.max_losing_trade_rate_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxWhiteRealityPValuePpm, |d| {
                d.max_white_reality_p_value_ppm = Some(PPM + 1);
            }),
            (AdmissionFieldV1::MaxRomanoWolfPValuePpm, |d| {
                d.max_romano_wolf_p_value_ppm = Some(PPM + 1);
            }),
        ];
        for (field, set) in setters {
            let mut invalid = draft();
            set(&mut invalid);
            let refusal = AdmissionPolicyV1::new(invalid).expect_err("ppm overflow must refuse");
            assert_eq!(refusal.field, field);
            assert_eq!(refusal.kind, AdmissionPolicyRefusalKindV1::AboveOneMillion);
        }
    }

    #[test]
    fn every_measured_failure_has_its_own_stable_reason() {
        for reason in AdmissionReasonV1::ALL {
            let mut values = passing_values();
            fail(&mut values, reason);
            let verdict = policy_for_failure(reason).evaluate(&evidence(&values));
            assert_eq!(
                verdict.status(),
                AdmissionStatusV1::Rejected,
                "{}",
                reason.name()
            );
            assert_eq!(
                verdict.failed(),
                ReasonBits::one(reason),
                "{}",
                reason.name()
            );
            assert_eq!(
                verdict.reasons(),
                ReasonBits::one(reason),
                "{}",
                reason.name()
            );
            assert!(verdict.unmeasured().is_empty());
            assert!(verdict.refused().is_empty());
            assert!(verdict.reconciles());
        }
    }

    #[test]
    fn all_pairwise_failure_permutations_and_the_full_set_are_collected() {
        // Every ordered position can coexist with every later position. This
        // catches an early return at every boundary without attempting an
        // intractable power-set test. Count/rate-linked reasons are covered by
        // their reconciled joint case below rather than contradictory tuples.
        for (left_index, left) in AdmissionReasonV1::ALL.into_iter().enumerate() {
            if trade_linked(left) {
                continue;
            }
            for right in AdmissionReasonV1::ALL.into_iter().skip(left_index) {
                if trade_linked(right) {
                    continue;
                }
                let mut values = passing_values();
                fail(&mut values, left);
                fail(&mut values, right);
                let expected = ReasonBits::one(left).union(ReasonBits::one(right));
                let verdict = policy().evaluate(&evidence(&values));
                assert_eq!(
                    verdict.failed(),
                    expected,
                    "{} + {}",
                    left.name(),
                    right.name()
                );
                assert_eq!(verdict.reasons(), expected);
                assert!(verdict.reconciles());
            }
        }

        let mut every_failure = passing_values();
        for reason in AdmissionReasonV1::ALL {
            if !trade_linked(reason) {
                fail(&mut every_failure, reason);
            }
        }
        fail_all_trade_metrics(&mut every_failure);
        let verdict = policy().evaluate(&evidence(&every_failure));
        assert_eq!(verdict.failed(), ReasonBits::KNOWN);
        assert_eq!(verdict.reasons(), ReasonBits::KNOWN);
        assert_eq!(verdict.reasons().count(), 44);
        assert!(verdict.reconciles());
    }

    #[test]
    fn every_check_has_distinct_unmeasured_and_refused_shapes() {
        for reason in AdmissionReasonV1::ALL {
            if matches!(
                reason,
                AdmissionReasonV1::Trades
                    | AdmissionReasonV1::WinningTrades
                    | AdmissionReasonV1::LosingTrades
            ) {
                continue;
            }
            let mut missing = passing_values();
            set_unmeasured(&mut missing, reason);
            let unmeasured = policy().evaluate(&evidence(&missing));
            assert_eq!(unmeasured.status(), AdmissionStatusV1::Unmeasured);
            assert_eq!(unmeasured.unmeasured(), ReasonBits::one(reason));
            assert!(unmeasured.failed().is_empty());
            assert!(unmeasured.refused().is_empty());
            assert!(unmeasured.reconciles());

            let mut denied = passing_values();
            set_refused(&mut denied, reason);
            let refused = policy().evaluate(&evidence(&denied));
            assert_eq!(refused.status(), AdmissionStatusV1::Refused);
            assert_eq!(refused.refused(), ReasonBits::one(reason));
            assert!(refused.failed().is_empty());
            assert!(refused.unmeasured().is_empty());
            assert!(refused.reconciles());
        }
    }

    #[test]
    fn unavailable_trade_tuple_keeps_every_affected_reason_explicit() {
        let affected = ReasonBits::one(AdmissionReasonV1::Trades)
            .union(ReasonBits::one(AdmissionReasonV1::WinRate))
            .union(ReasonBits::one(AdmissionReasonV1::LosingTradeRate))
            .union(ReasonBits::one(AdmissionReasonV1::WinningTrades))
            .union(ReasonBits::one(AdmissionReasonV1::LosingTrades));

        let mut missing = passing_values();
        missing.trades = ObservedU64V1::Unmeasured;
        missing.winning_trades = ObservedU64V1::Unmeasured;
        missing.losing_trades = ObservedU64V1::Unmeasured;
        missing.win_rate_ppm = ObservedU64V1::Unmeasured;
        missing.losing_trade_rate_ppm = ObservedU64V1::Unmeasured;
        let verdict = policy().evaluate(&evidence(&missing));
        assert_eq!(verdict.status(), AdmissionStatusV1::Unmeasured);
        assert_eq!(verdict.unmeasured(), affected);
        assert!(verdict.reconciles());

        let mut refused = passing_values();
        refused.trades = ObservedU64V1::Refused;
        refused.winning_trades = ObservedU64V1::Refused;
        refused.losing_trades = ObservedU64V1::Refused;
        refused.win_rate_ppm = ObservedU64V1::Refused;
        refused.losing_trade_rate_ppm = ObservedU64V1::Refused;
        let verdict = policy().evaluate(&evidence(&refused));
        assert_eq!(verdict.status(), AdmissionStatusV1::Refused);
        assert_eq!(verdict.refused(), affected);
        assert!(verdict.reconciles());
    }

    #[test]
    fn refused_precedes_unmeasured_but_no_other_reason_is_hidden() {
        let mut values = passing_values();
        fail(&mut values, AdmissionReasonV1::Support);
        set_unmeasured(&mut values, AdmissionReasonV1::WilsonWinRate);
        set_refused(&mut values, AdmissionReasonV1::PopulationCompleteness);
        let verdict = policy().evaluate(&evidence(&values));
        assert_eq!(verdict.status(), AdmissionStatusV1::Refused);
        assert_eq!(
            verdict.failed(),
            ReasonBits::one(AdmissionReasonV1::Support)
        );
        assert_eq!(
            verdict.unmeasured(),
            ReasonBits::one(AdmissionReasonV1::WilsonWinRate)
        );
        assert_eq!(
            verdict.refused(),
            ReasonBits::one(AdmissionReasonV1::PopulationCompleteness)
        );
        assert_eq!(verdict.reasons().count(), 3);
        assert!(verdict.reconciles());
    }

    #[test]
    fn runtime_decision_policy_can_decline_rejection_without_hiding_test_absence() {
        let mut relaxed = draft();
        relaxed.require_white_reality_rejection = Some(false);
        relaxed.require_romano_wolf_rejection = Some(false);
        let relaxed = AdmissionPolicyV1::new(relaxed).expect("explicit false is a valid policy");
        let mut values = passing_values();
        values.white_reality_decision = HypothesisDecisionV1::DidNotReject;
        values.romano_wolf_decision = HypothesisDecisionV1::DidNotReject;
        assert!(relaxed.evaluate(&evidence(&values)).is_admitted());

        values.white_reality_decision = HypothesisDecisionV1::Unmeasured;
        let verdict = relaxed.evaluate(&evidence(&values));
        assert_eq!(verdict.status(), AdmissionStatusV1::Unmeasured);
        assert_eq!(
            verdict.unmeasured(),
            ReasonBits::one(AdmissionReasonV1::WhiteRealityDecision)
        );
    }

    #[test]
    fn measured_percentage_evidence_above_the_domain_is_refused_before_evaluation() {
        let setters: [(AdmissionEvidenceFieldV1, EvidenceSetter); 11] = [
            (AdmissionEvidenceFieldV1::WinRate, |e| {
                e.win_rate_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::WilsonWinRate, |e| {
                e.wilson_win_rate_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::FwerPValue, |e| {
                e.fwer_p_value_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::SpaPValue, |e| {
                e.spa_p_value_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::AmbiguousFillRate, |e| {
                e.ambiguous_fill_rate_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::GapAffectedRate, |e| {
                e.gap_affected_rate_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::SessionConcentration, |e| {
                e.session_concentration_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::LargestTradeProfitShare, |e| {
                e.largest_trade_profit_share_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::LosingTradeRate, |e| {
                e.losing_trade_rate_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::WhiteRealityPValue, |e| {
                e.white_reality_p_value_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
            (AdmissionEvidenceFieldV1::RomanoWolfPValue, |e| {
                e.romano_wolf_p_value_ppm = ObservedU64V1::Measured(PPM + 1);
            }),
        ];
        for (field, set) in setters {
            let mut invalid = constructible_values();
            set(&mut invalid);
            let refusal = AdmissionEvidenceV1::new(invalid).expect_err("invalid rate must refuse");
            assert_eq!(refusal.field, field);
            assert_eq!(
                refusal.kind,
                AdmissionEvidenceRefusalKindV1::AboveOneMillion
            );
        }
    }

    #[test]
    fn caller_crafted_measured_pbo_named_v1_evidence_is_refused_before_policy_evaluation() {
        let cases: [(AdmissionEvidenceFieldV1, EvidenceSetter); 3] = [
            (AdmissionEvidenceFieldV1::Pbo, |values| {
                values.pbo_ppm = ObservedU64V1::Measured(0);
            }),
            (AdmissionEvidenceFieldV1::PboContributingFolds, |values| {
                values.pbo_contributing_folds = ObservedU64V1::Measured(0);
            }),
            (AdmissionEvidenceFieldV1::PboUnrankableFolds, |values| {
                values.pbo_unrankable_folds = ObservedU64V1::Measured(0);
            }),
        ];
        for (field, set) in cases {
            let mut unsupported = constructible_values();
            set(&mut unsupported);
            assert_eq!(
                AdmissionEvidenceV1::new(unsupported),
                Err(super::AdmissionEvidenceRefusalV1 {
                    field,
                    kind: AdmissionEvidenceRefusalKindV1::UnsupportedMeasuredPboV1,
                }),
                "a measured {field:?} V1 slot must not reach the policy comparator"
            );
        }
    }

    #[test]
    fn reopened_legacy_measured_pbo_named_v1_bytes_refuse_loudly() {
        let cases: [(AdmissionEvidenceFieldV1, EvidenceSetter); 3] = [
            (AdmissionEvidenceFieldV1::Pbo, |values| {
                values.pbo_ppm = ObservedU64V1::Measured(100_000);
            }),
            (AdmissionEvidenceFieldV1::PboContributingFolds, |values| {
                values.pbo_contributing_folds = ObservedU64V1::Measured(10);
            }),
            (AdmissionEvidenceFieldV1::PboUnrankableFolds, |values| {
                values.pbo_unrankable_folds = ObservedU64V1::Measured(2);
            }),
        ];
        for (field, set) in cases {
            let mut legacy = constructible_values();
            set(&mut legacy);
            // A private literal is intentional: it models bytes written by the
            // pre-D-0467 constructor, which the new public constructor can no
            // longer produce.
            let legacy_evidence = AdmissionEvidenceV1 { values: legacy };
            let bytes = legacy_evidence.canonical_bytes();
            let decision_bytes = policy().evaluate_sealed(&legacy_evidence).canonical_bytes();
            let expected =
                AdmissionCanonicalRefusalV1::Evidence(super::AdmissionEvidenceRefusalV1 {
                    field,
                    kind: AdmissionEvidenceRefusalKindV1::UnsupportedMeasuredPboV1,
                });
            assert_eq!(
                AdmissionEvidenceV1::from_canonical_bytes(&bytes),
                Err(expected),
                "legacy measured {field:?} bytes must never reopen as authority"
            );
            assert_eq!(
                AdmissionDecisionSealV1::from_canonical_bytes(&decision_bytes),
                Err(expected),
                "a sealed legacy measured {field:?} record must fail at its nested evidence"
            );
        }
    }

    #[test]
    fn unmeasured_and_refused_pbo_named_v1_bytes_remain_readable() {
        for state in [ObservedU64V1::Unmeasured, ObservedU64V1::Refused] {
            let mut values = constructible_values();
            values.pbo_ppm = state;
            values.pbo_contributing_folds = state;
            values.pbo_unrankable_folds = state;
            let evidence = AdmissionEvidenceV1::new(values)
                .expect("a non-authoritative PBO state remains valid V1 evidence");
            assert_eq!(
                AdmissionEvidenceV1::from_canonical_bytes(&evidence.canonical_bytes()),
                Ok(evidence)
            );
        }
    }

    #[test]
    fn publicly_constructible_and_reopened_v1_evidence_cannot_authorize_pbo() {
        let expected = ReasonBits::one(AdmissionReasonV1::Pbo)
            .union(ReasonBits::one(AdmissionReasonV1::PboContributingFolds))
            .union(ReasonBits::one(AdmissionReasonV1::PboUnrankableFolds));
        let evidence = constructible_evidence();
        let reopened = AdmissionEvidenceV1::from_canonical_bytes(&evidence.canonical_bytes())
            .expect("non-authoritative PBO states remain auditable after reopen");

        for candidate in [evidence, reopened] {
            let verdict = policy().evaluate(&candidate);
            assert_eq!(verdict.status(), AdmissionStatusV1::Unmeasured);
            assert_eq!(verdict.unmeasured(), expected);
            assert!(verdict.failed().is_empty());
            assert!(verdict.refused().is_empty());
            assert!(!verdict.is_admitted());
            assert!(verdict.reconciles());
        }
    }

    #[test]
    fn trade_count_and_rate_tampering_is_refused_by_exact_field_and_kind() {
        let mut mismatch = constructible_values();
        mismatch.winning_trades = ObservedU64V1::Measured(31);
        let refusal = AdmissionEvidenceV1::new(mismatch).expect_err("31 + 20 is not 50");
        assert_eq!(refusal.field, AdmissionEvidenceFieldV1::TradeCounts);
        assert_eq!(
            refusal.kind,
            AdmissionEvidenceRefusalKindV1::TradeCountMismatch
        );

        let mut overflow = constructible_values();
        overflow.trades = ObservedU64V1::Measured(u64::MAX);
        overflow.winning_trades = ObservedU64V1::Measured(u64::MAX);
        overflow.losing_trades = ObservedU64V1::Measured(1);
        let refusal = AdmissionEvidenceV1::new(overflow).expect_err("count sum overflows");
        assert_eq!(refusal.field, AdmissionEvidenceFieldV1::TradeCounts);
        assert_eq!(
            refusal.kind,
            AdmissionEvidenceRefusalKindV1::TradeCountOverflow
        );

        let mut win_rate = constructible_values();
        win_rate.win_rate_ppm = ObservedU64V1::Measured(599_999);
        let refusal = AdmissionEvidenceV1::new(win_rate).expect_err("win rate was hand-edited");
        assert_eq!(refusal.field, AdmissionEvidenceFieldV1::WinRate);
        assert_eq!(
            refusal.kind,
            AdmissionEvidenceRefusalKindV1::RateCountMismatch
        );

        let mut loss_rate = constructible_values();
        loss_rate.losing_trade_rate_ppm = ObservedU64V1::Measured(399_999);
        let refusal = AdmissionEvidenceV1::new(loss_rate).expect_err("loss rate was hand-edited");
        assert_eq!(refusal.field, AdmissionEvidenceFieldV1::LosingTradeRate);
        assert_eq!(
            refusal.kind,
            AdmissionEvidenceRefusalKindV1::RateCountMismatch
        );

        let mut missing_count = constructible_values();
        missing_count.winning_trades = ObservedU64V1::Unmeasured;
        let refusal = AdmissionEvidenceV1::new(missing_count)
            .expect_err("a measured rate cannot outlive its count");
        assert_eq!(refusal.field, AdmissionEvidenceFieldV1::WinRate);
        assert_eq!(
            refusal.kind,
            AdmissionEvidenceRefusalKindV1::RateWithoutCounts
        );
    }

    #[test]
    fn every_measured_subset_is_bounded_by_its_measured_total() {
        let cases: [(AdmissionEvidenceFieldV1, EvidenceSetter); 6] = [
            (AdmissionEvidenceFieldV1::SupportSessions, |values| {
                values.independent_sessions = ObservedU64V1::Measured(101);
            }),
            (AdmissionEvidenceFieldV1::TradeCounts, |values| {
                values.winning_trades = ObservedU64V1::Measured(51);
                values.losing_trades = ObservedU64V1::Unmeasured;
                values.win_rate_ppm = ObservedU64V1::Unmeasured;
                values.losing_trade_rate_ppm = ObservedU64V1::Unmeasured;
            }),
            (AdmissionEvidenceFieldV1::TradeCounts, |values| {
                values.losing_trades = ObservedU64V1::Measured(51);
                values.winning_trades = ObservedU64V1::Unmeasured;
                values.win_rate_ppm = ObservedU64V1::Unmeasured;
                values.losing_trade_rate_ppm = ObservedU64V1::Unmeasured;
            }),
            (AdmissionEvidenceFieldV1::WinningStreak, |values| {
                values.consecutive_winning_streak = ObservedU64V1::Measured(31);
            }),
            (AdmissionEvidenceFieldV1::LosingStreak, |values| {
                values.consecutive_losing_streak = ObservedU64V1::Measured(21);
            }),
            (AdmissionEvidenceFieldV1::OosFolds, |values| {
                values.profitable_oos_folds = ObservedU64V1::Measured(11);
            }),
        ];
        for (field, change) in cases {
            let mut invalid = constructible_values();
            change(&mut invalid);
            let refusal = AdmissionEvidenceV1::new(invalid)
                .expect_err("a measured subset cannot exceed its total");
            assert_eq!(refusal.field, field);
            assert_eq!(
                refusal.kind,
                AdmissionEvidenceRefusalKindV1::SubsetExceedsTotal
            );
        }
    }

    #[test]
    fn zero_trades_has_explicit_undefined_rates_and_never_a_fabricated_zero_rate() {
        let mut zero = constructible_values();
        zero.trades = ObservedU64V1::Measured(0);
        zero.winning_trades = ObservedU64V1::Measured(0);
        zero.losing_trades = ObservedU64V1::Measured(0);
        zero.win_rate_ppm = ObservedU64V1::Unmeasured;
        zero.losing_trade_rate_ppm = ObservedU64V1::Unmeasured;
        zero.consecutive_winning_streak = ObservedU64V1::Measured(0);
        zero.consecutive_losing_streak = ObservedU64V1::Measured(0);
        assert!(AdmissionEvidenceV1::new(zero).is_ok());

        zero.win_rate_ppm = ObservedU64V1::Measured(0);
        let refusal = AdmissionEvidenceV1::new(zero)
            .expect_err("zero trades cannot define a measured win rate");
        assert_eq!(refusal.field, AdmissionEvidenceFieldV1::WinRate);
        assert_eq!(
            refusal.kind,
            AdmissionEvidenceRefusalKindV1::RateWithZeroDenominator
        );
    }

    #[test]
    fn extreme_integer_values_do_not_wrap_or_invent_a_limit() {
        let extreme_policy = AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(u64::MAX),
            min_independent_sessions: Some(u64::MAX),
            min_trades: Some(u64::MAX),
            max_mae_paisa: Some(u64::MAX),
            min_worst_reward_risk_ppm: Some(u64::MAX),
            min_win_rate_ppm: Some(PPM),
            min_wilson_win_rate_ppm: Some(PPM),
            min_return_drawdown_ppm: Some(u64::MAX),
            min_weakest_period_return_paisa: Some(i64::MIN),
            max_pbo_ppm: Some(PPM),
            max_fwer_p_value_ppm: Some(PPM),
            max_spa_p_value_ppm: Some(PPM),
            min_decided_folds: Some(u64::MAX),
            max_ambiguous_fill_rate_ppm: Some(PPM),
            max_gap_affected_rate_ppm: Some(PPM),
            max_session_concentration_ppm: Some(PPM),
            max_largest_trade_profit_share_ppm: Some(PPM),
            max_drawdown_paisa: Some(u64::MAX),
            max_worst_trade_loss_paisa: Some(u64::MAX),
            max_losing_trade_rate_ppm: Some(PPM),
            max_losing_trades: Some(u64::MAX),
            min_pessimistic_profit_paisa: Some(i64::MIN),
            min_winning_trades: Some(u64::MAX),
            min_average_win_paisa: Some(u64::MAX),
            max_average_loss_paisa: Some(u64::MAX),
            min_profit_factor_ppm: Some(u64::MAX),
            max_consecutive_losing_streak: Some(u64::MAX),
            min_consecutive_winning_streak: Some(u64::MAX),
            min_bootstrap_draws: Some(u64::MAX),
            min_bootstrap_strategies: Some(u64::MAX),
            min_bootstrap_periods: Some(u64::MAX),
            min_pbo_contributing_folds: Some(u64::MAX),
            max_pbo_unrankable_folds: Some(u64::MAX),
            min_profitable_oos_folds: Some(u64::MAX),
            min_oos_pessimistic_return_paisa: Some(i64::MIN),
            max_white_reality_p_value_ppm: Some(PPM),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(PPM),
            require_romano_wolf_rejection: Some(true),
        })
        .expect("ratios and magnitudes may span their full integer domain");
        let extreme_evidence = evidence(&AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(u64::MAX),
            independent_sessions: ObservedU64V1::Measured(u64::MAX),
            trades: ObservedU64V1::Measured(u64::MAX),
            max_mae_paisa: ObservedU64V1::Measured(u64::MAX),
            worst_reward_risk_ppm: ObservedU64V1::Measured(u64::MAX),
            win_rate_ppm: ObservedU64V1::Measured(PPM),
            wilson_win_rate_ppm: ObservedU64V1::Measured(PPM),
            return_drawdown_ppm: ObservedU64V1::Measured(u64::MAX),
            weakest_period_return_paisa: ObservedI64V1::Measured(i64::MIN),
            pbo_ppm: ObservedU64V1::Measured(PPM),
            fwer_p_value_ppm: ObservedU64V1::Measured(PPM),
            spa_p_value_ppm: ObservedU64V1::Measured(PPM),
            decided_folds: ObservedU64V1::Measured(u64::MAX),
            ambiguous_fill_rate_ppm: ObservedU64V1::Measured(PPM),
            gap_affected_rate_ppm: ObservedU64V1::Measured(PPM),
            session_concentration_ppm: ObservedU64V1::Measured(PPM),
            largest_trade_profit_share_ppm: ObservedU64V1::Measured(PPM),
            execution_complete: CompletenessV1::Complete,
            data_complete: CompletenessV1::Complete,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Measured(u64::MAX),
            worst_trade_loss_paisa: ObservedU64V1::Measured(u64::MAX),
            losing_trade_rate_ppm: ObservedU64V1::Measured(0),
            losing_trades: ObservedU64V1::Measured(0),
            pessimistic_profit_paisa: ObservedI64V1::Measured(i64::MIN),
            winning_trades: ObservedU64V1::Measured(u64::MAX),
            average_win_paisa: ObservedU64V1::Measured(u64::MAX),
            average_loss_paisa: ObservedU64V1::Measured(u64::MAX),
            profit_factor_ppm: ObservedU64V1::Measured(u64::MAX),
            consecutive_losing_streak: ObservedU64V1::Measured(0),
            consecutive_winning_streak: ObservedU64V1::Measured(u64::MAX),
            bootstrap_draws: ObservedU64V1::Measured(u64::MAX),
            bootstrap_strategies: ObservedU64V1::Measured(u64::MAX),
            bootstrap_periods: ObservedU64V1::Measured(u64::MAX),
            pbo_contributing_folds: ObservedU64V1::Measured(u64::MAX),
            pbo_unrankable_folds: ObservedU64V1::Measured(u64::MAX),
            profitable_oos_folds: ObservedU64V1::Measured(u64::MAX),
            oos_pessimistic_return_paisa: ObservedI64V1::Measured(i64::MIN),
            white_reality_p_value_ppm: ObservedU64V1::Measured(PPM),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(PPM),
            white_reality_decision: HypothesisDecisionV1::RejectedNull,
            romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
            full_precision_statistics_complete: CompletenessV1::Complete,
        });
        assert!(extreme_policy.evaluate(&extreme_evidence).is_admitted());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one adversarial fixture must show all numeric extrema and the exact excluded completeness/decision set together"
    )]
    fn opposite_integer_extremes_fail_every_numeric_check_without_overflow() {
        let extreme_policy = AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(u64::MAX),
            min_independent_sessions: Some(u64::MAX),
            min_trades: Some(u64::MAX),
            max_mae_paisa: Some(0),
            min_worst_reward_risk_ppm: Some(u64::MAX),
            min_win_rate_ppm: Some(PPM),
            min_wilson_win_rate_ppm: Some(PPM),
            min_return_drawdown_ppm: Some(u64::MAX),
            min_weakest_period_return_paisa: Some(i64::MAX),
            max_pbo_ppm: Some(0),
            max_fwer_p_value_ppm: Some(0),
            max_spa_p_value_ppm: Some(0),
            min_decided_folds: Some(u64::MAX),
            max_ambiguous_fill_rate_ppm: Some(0),
            max_gap_affected_rate_ppm: Some(0),
            max_session_concentration_ppm: Some(0),
            max_largest_trade_profit_share_ppm: Some(0),
            max_drawdown_paisa: Some(0),
            max_worst_trade_loss_paisa: Some(0),
            max_losing_trade_rate_ppm: Some(0),
            max_losing_trades: Some(0),
            min_pessimistic_profit_paisa: Some(i64::MAX),
            min_winning_trades: Some(u64::MAX),
            min_average_win_paisa: Some(u64::MAX),
            max_average_loss_paisa: Some(0),
            min_profit_factor_ppm: Some(u64::MAX),
            max_consecutive_losing_streak: Some(0),
            min_consecutive_winning_streak: Some(u64::MAX),
            min_bootstrap_draws: Some(u64::MAX),
            min_bootstrap_strategies: Some(u64::MAX),
            min_bootstrap_periods: Some(u64::MAX),
            min_pbo_contributing_folds: Some(u64::MAX),
            max_pbo_unrankable_folds: Some(0),
            min_profitable_oos_folds: Some(u64::MAX),
            min_oos_pessimistic_return_paisa: Some(i64::MAX),
            max_white_reality_p_value_ppm: Some(0),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(0),
            require_romano_wolf_rejection: Some(true),
        })
        .expect("every extreme remains in its declared domain");
        let extreme_evidence = evidence(&AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(0),
            independent_sessions: ObservedU64V1::Measured(0),
            trades: ObservedU64V1::Measured(u64::MAX - 1),
            max_mae_paisa: ObservedU64V1::Measured(u64::MAX),
            worst_reward_risk_ppm: ObservedU64V1::Measured(0),
            win_rate_ppm: ObservedU64V1::Measured(0),
            wilson_win_rate_ppm: ObservedU64V1::Measured(0),
            return_drawdown_ppm: ObservedU64V1::Measured(0),
            weakest_period_return_paisa: ObservedI64V1::Measured(i64::MIN),
            pbo_ppm: ObservedU64V1::Measured(PPM),
            fwer_p_value_ppm: ObservedU64V1::Measured(PPM),
            spa_p_value_ppm: ObservedU64V1::Measured(PPM),
            decided_folds: ObservedU64V1::Measured(0),
            ambiguous_fill_rate_ppm: ObservedU64V1::Measured(PPM),
            gap_affected_rate_ppm: ObservedU64V1::Measured(PPM),
            session_concentration_ppm: ObservedU64V1::Measured(PPM),
            largest_trade_profit_share_ppm: ObservedU64V1::Measured(PPM),
            execution_complete: CompletenessV1::Complete,
            data_complete: CompletenessV1::Complete,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Measured(u64::MAX),
            worst_trade_loss_paisa: ObservedU64V1::Measured(u64::MAX),
            losing_trade_rate_ppm: ObservedU64V1::Measured(PPM),
            losing_trades: ObservedU64V1::Measured(u64::MAX - 1),
            pessimistic_profit_paisa: ObservedI64V1::Measured(i64::MIN),
            winning_trades: ObservedU64V1::Measured(0),
            average_win_paisa: ObservedU64V1::Measured(0),
            average_loss_paisa: ObservedU64V1::Measured(u64::MAX),
            profit_factor_ppm: ObservedU64V1::Measured(0),
            consecutive_losing_streak: ObservedU64V1::Measured(u64::MAX - 1),
            consecutive_winning_streak: ObservedU64V1::Measured(0),
            bootstrap_draws: ObservedU64V1::Measured(0),
            bootstrap_strategies: ObservedU64V1::Measured(0),
            bootstrap_periods: ObservedU64V1::Measured(0),
            pbo_contributing_folds: ObservedU64V1::Measured(0),
            pbo_unrankable_folds: ObservedU64V1::Measured(u64::MAX),
            profitable_oos_folds: ObservedU64V1::Measured(0),
            oos_pessimistic_return_paisa: ObservedI64V1::Measured(i64::MIN),
            white_reality_p_value_ppm: ObservedU64V1::Measured(PPM),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(PPM),
            white_reality_decision: HypothesisDecisionV1::RejectedNull,
            romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
            full_precision_statistics_complete: CompletenessV1::Complete,
        });
        let completeness = ReasonBits::one(AdmissionReasonV1::ExecutionCompleteness)
            .union(ReasonBits::one(AdmissionReasonV1::DataCompleteness))
            .union(ReasonBits::one(AdmissionReasonV1::CalendarCompleteness))
            .union(ReasonBits::one(AdmissionReasonV1::PopulationCompleteness))
            .union(ReasonBits::one(
                AdmissionReasonV1::FullPrecisionStatisticsCompleteness,
            ))
            .union(ReasonBits::one(AdmissionReasonV1::WhiteRealityDecision))
            .union(ReasonBits::one(AdmissionReasonV1::RomanoWolfDecision));
        let expected = ReasonBits::from_bits(ReasonBits::KNOWN.bits() & !completeness.bits())
            .expect("removing known bits remains known");
        let verdict = extreme_policy.evaluate(&extreme_evidence);
        assert_eq!(verdict.failed(), expected);
        assert_eq!(verdict.reasons().count(), 37);
        assert!(verdict.reconciles());
    }

    #[test]
    fn every_policy_field_is_load_bearing_in_canonical_bytes_and_digest() {
        let mutations: [PolicyValueSetter; 39] = [
            |v| v.min_support_hits ^= 1,
            |v| v.min_independent_sessions ^= 1,
            |v| v.min_trades ^= 1,
            |v| v.max_mae_paisa ^= 1,
            |v| v.min_worst_reward_risk_ppm ^= 1,
            |v| v.min_win_rate_ppm ^= 1,
            |v| v.min_wilson_win_rate_ppm ^= 1,
            |v| v.min_return_drawdown_ppm ^= 1,
            |v| v.min_weakest_period_return_paisa ^= 1,
            |v| v.max_pbo_ppm ^= 1,
            |v| v.max_fwer_p_value_ppm ^= 1,
            |v| v.max_spa_p_value_ppm ^= 1,
            |v| v.min_decided_folds ^= 1,
            |v| v.max_ambiguous_fill_rate_ppm ^= 1,
            |v| v.max_gap_affected_rate_ppm ^= 1,
            |v| v.max_session_concentration_ppm ^= 1,
            |v| v.max_largest_trade_profit_share_ppm ^= 1,
            |v| v.max_drawdown_paisa ^= 1,
            |v| v.max_worst_trade_loss_paisa ^= 1,
            |v| v.max_losing_trade_rate_ppm ^= 1,
            |v| v.max_losing_trades ^= 1,
            |v| v.min_pessimistic_profit_paisa ^= 1,
            |v| v.min_winning_trades ^= 1,
            |v| v.min_average_win_paisa ^= 1,
            |v| v.max_average_loss_paisa ^= 1,
            |v| v.min_profit_factor_ppm ^= 1,
            |v| v.max_consecutive_losing_streak ^= 1,
            |v| v.min_consecutive_winning_streak ^= 1,
            |v| v.min_bootstrap_draws ^= 1,
            |v| v.min_bootstrap_strategies ^= 1,
            |v| v.min_bootstrap_periods ^= 1,
            |v| v.min_pbo_contributing_folds ^= 1,
            |v| v.max_pbo_unrankable_folds ^= 1,
            |v| v.min_profitable_oos_folds ^= 1,
            |v| v.min_oos_pessimistic_return_paisa ^= 1,
            |v| v.max_white_reality_p_value_ppm ^= 1,
            |v| v.require_white_reality_rejection = !v.require_white_reality_rejection,
            |v| v.max_romano_wolf_p_value_ppm ^= 1,
            |v| v.require_romano_wolf_rejection = !v.require_romano_wolf_rejection,
        ];
        assert_eq!(mutations.len(), AdmissionFieldV1::ALL.len());

        let original = policy();
        for mutate in mutations {
            let mut values = original.values();
            mutate(&mut values);
            let changed = AdmissionPolicyV1 { values };
            assert_ne!(changed.canonical_bytes(), original.canonical_bytes());
            assert_ne!(changed.digest(), original.digest());
        }
    }

    #[test]
    fn every_evidence_field_is_load_bearing_in_canonical_bytes_and_digest() {
        let mutations: [EvidenceSetter; 44] = [
            |v| v.support_hits = ObservedU64V1::Unmeasured,
            |v| v.independent_sessions = ObservedU64V1::Unmeasured,
            |v| v.trades = ObservedU64V1::Unmeasured,
            |v| v.max_mae_paisa = ObservedU64V1::Unmeasured,
            |v| v.worst_reward_risk_ppm = ObservedU64V1::Unmeasured,
            |v| v.win_rate_ppm = ObservedU64V1::Unmeasured,
            |v| v.wilson_win_rate_ppm = ObservedU64V1::Unmeasured,
            |v| v.return_drawdown_ppm = ObservedU64V1::Unmeasured,
            |v| v.weakest_period_return_paisa = ObservedI64V1::Unmeasured,
            |v| v.pbo_ppm = ObservedU64V1::Unmeasured,
            |v| v.fwer_p_value_ppm = ObservedU64V1::Unmeasured,
            |v| v.spa_p_value_ppm = ObservedU64V1::Unmeasured,
            |v| v.decided_folds = ObservedU64V1::Unmeasured,
            |v| v.ambiguous_fill_rate_ppm = ObservedU64V1::Unmeasured,
            |v| v.gap_affected_rate_ppm = ObservedU64V1::Unmeasured,
            |v| v.session_concentration_ppm = ObservedU64V1::Unmeasured,
            |v| v.largest_trade_profit_share_ppm = ObservedU64V1::Unmeasured,
            |v| v.execution_complete = CompletenessV1::Incomplete,
            |v| v.data_complete = CompletenessV1::Incomplete,
            |v| v.calendar_complete = CompletenessV1::Incomplete,
            |v| v.population_complete = CompletenessV1::Incomplete,
            |v| v.drawdown_paisa = ObservedU64V1::Unmeasured,
            |v| v.worst_trade_loss_paisa = ObservedU64V1::Unmeasured,
            |v| v.losing_trade_rate_ppm = ObservedU64V1::Unmeasured,
            |v| v.losing_trades = ObservedU64V1::Unmeasured,
            |v| v.pessimistic_profit_paisa = ObservedI64V1::Unmeasured,
            |v| v.winning_trades = ObservedU64V1::Unmeasured,
            |v| v.average_win_paisa = ObservedU64V1::Unmeasured,
            |v| v.average_loss_paisa = ObservedU64V1::Unmeasured,
            |v| v.profit_factor_ppm = ObservedU64V1::Unmeasured,
            |v| v.consecutive_losing_streak = ObservedU64V1::Unmeasured,
            |v| v.consecutive_winning_streak = ObservedU64V1::Unmeasured,
            |v| v.bootstrap_draws = ObservedU64V1::Unmeasured,
            |v| v.bootstrap_strategies = ObservedU64V1::Unmeasured,
            |v| v.bootstrap_periods = ObservedU64V1::Unmeasured,
            |v| v.pbo_contributing_folds = ObservedU64V1::Unmeasured,
            |v| v.pbo_unrankable_folds = ObservedU64V1::Unmeasured,
            |v| v.profitable_oos_folds = ObservedU64V1::Unmeasured,
            |v| v.oos_pessimistic_return_paisa = ObservedI64V1::Unmeasured,
            |v| v.white_reality_p_value_ppm = ObservedU64V1::Unmeasured,
            |v| v.romano_wolf_p_value_ppm = ObservedU64V1::Unmeasured,
            |v| v.white_reality_decision = HypothesisDecisionV1::DidNotReject,
            |v| v.romano_wolf_decision = HypothesisDecisionV1::DidNotReject,
            |v| v.full_precision_statistics_complete = CompletenessV1::Incomplete,
        ];
        assert_eq!(mutations.len(), AdmissionReasonV1::ALL.len());

        let original = evidence(&passing_values());
        for mutate in mutations {
            let mut values = original.values();
            mutate(&mut values);
            let changed = AdmissionEvidenceV1 { values };
            assert_ne!(changed.canonical_bytes(), original.canonical_bytes());
            assert_ne!(changed.digest(), original.digest());
        }
    }

    #[test]
    fn every_verdict_field_is_load_bearing_and_seal_corruption_is_detected() {
        let mutations: [VerdictSetter; 5] = [
            |v| v.reasons = ReasonBits::one(AdmissionReasonV1::Support),
            |v| v.failed = ReasonBits::one(AdmissionReasonV1::Support),
            |v| v.unmeasured = ReasonBits::one(AdmissionReasonV1::Support),
            |v| v.refused = ReasonBits::one(AdmissionReasonV1::Support),
            |v| v.status = AdmissionStatusV1::Rejected,
        ];
        let original = policy().evaluate(&evidence(&passing_values()));
        for mutate in mutations {
            let mut changed = original;
            mutate(&mut changed);
            assert_ne!(changed.canonical_bytes(), original.canonical_bytes());
            assert_ne!(changed.digest(), original.digest());
        }

        let seal = policy().evaluate_sealed(&evidence(&passing_values()));
        assert!(seal.recomputes());
        assert_eq!(seal.verdict(), original);
        assert_eq!(seal.policy(), policy());
        assert_eq!(seal.evidence(), evidence(&passing_values()));
        let mut corrupted = seal;
        corrupted.verdict.status = AdmissionStatusV1::Rejected;
        assert!(!corrupted.recomputes());
        assert_ne!(corrupted.digest(), seal.digest());
    }

    #[test]
    fn undefined_refused_and_numeric_sentinels_are_unambiguous() {
        let mut base = passing_values();
        base.independent_sessions = ObservedU64V1::Measured(0);

        let mut measured_zero = base;
        measured_zero.support_hits = ObservedU64V1::Measured(0);
        let mut unmeasured = base;
        unmeasured.support_hits = ObservedU64V1::Unmeasured;
        let mut refused = base;
        refused.support_hits = ObservedU64V1::Refused;
        let [unsigned_measured, unsigned_unmeasured, unsigned_refused] = [
            AdmissionEvidenceV1 {
                values: measured_zero,
            }
            .digest(),
            AdmissionEvidenceV1 { values: unmeasured }.digest(),
            AdmissionEvidenceV1 { values: refused }.digest(),
        ];
        assert_ne!(unsigned_measured, unsigned_unmeasured);
        assert_ne!(unsigned_measured, unsigned_refused);
        assert_ne!(unsigned_unmeasured, unsigned_refused);

        let mut signed_min = base;
        signed_min.weakest_period_return_paisa = ObservedI64V1::Measured(i64::MIN);
        let mut signed_unmeasured = base;
        signed_unmeasured.weakest_period_return_paisa = ObservedI64V1::Unmeasured;
        let mut signed_refused = base;
        signed_refused.weakest_period_return_paisa = ObservedI64V1::Refused;
        let [signed_measured, signed_unmeasured, signed_refused] = [
            AdmissionEvidenceV1 { values: signed_min }.digest(),
            AdmissionEvidenceV1 {
                values: signed_unmeasured,
            }
            .digest(),
            AdmissionEvidenceV1 {
                values: signed_refused,
            }
            .digest(),
        ];
        assert_ne!(signed_measured, signed_unmeasured);
        assert_ne!(signed_measured, signed_refused);
        assert_ne!(signed_unmeasured, signed_refused);

        let completeness = [
            CompletenessV1::Complete,
            CompletenessV1::Incomplete,
            CompletenessV1::Unmeasured,
            CompletenessV1::Refused,
        ];
        let decisions = [
            HypothesisDecisionV1::RejectedNull,
            HypothesisDecisionV1::DidNotReject,
            HypothesisDecisionV1::Unmeasured,
            HypothesisDecisionV1::Refused,
        ];
        for (left_index, left) in completeness.into_iter().enumerate() {
            for right in completeness.into_iter().skip(left_index + 1) {
                let mut left_values = base;
                let mut right_values = base;
                left_values.execution_complete = left;
                right_values.execution_complete = right;
                assert_ne!(
                    AdmissionEvidenceV1 {
                        values: left_values
                    }
                    .digest(),
                    AdmissionEvidenceV1 {
                        values: right_values
                    }
                    .digest()
                );
            }
        }
        for (left_index, left) in decisions.into_iter().enumerate() {
            for right in decisions.into_iter().skip(left_index + 1) {
                let mut left_values = base;
                let mut right_values = base;
                left_values.white_reality_decision = left;
                right_values.white_reality_decision = right;
                assert_ne!(
                    AdmissionEvidenceV1 {
                        values: left_values
                    }
                    .digest(),
                    AdmissionEvidenceV1 {
                        values: right_values
                    }
                    .digest()
                );
            }
        }
    }

    #[test]
    fn canonical_decoders_round_trip_every_valid_record() {
        let policy = policy();
        let evidence = constructible_evidence();
        let seal = policy.evaluate_sealed(&evidence);
        let policy_bytes = policy.canonical_bytes();
        let evidence_bytes = evidence.canonical_bytes();
        let verdict_bytes = seal.verdict().canonical_bytes();
        let decision_bytes = seal.canonical_bytes();

        assert_eq!(
            AdmissionPolicyV1::from_canonical_bytes(&policy_bytes),
            Ok(policy)
        );
        assert_eq!(
            AdmissionEvidenceV1::from_canonical_bytes(&evidence_bytes),
            Ok(evidence)
        );
        assert_eq!(
            AdmissionVerdictV1::from_canonical_bytes(&verdict_bytes),
            Ok(seal.verdict())
        );
        assert_eq!(
            AdmissionDecisionSealV1::from_canonical_parts(
                &policy_bytes,
                &evidence_bytes,
                &verdict_bytes,
            ),
            Ok(seal)
        );
        assert_eq!(
            AdmissionDecisionSealV1::from_canonical_bytes(&decision_bytes),
            Ok(seal)
        );
        assert_eq!(
            AdmissionDecisionSealV1::from_canonical_bytes(&decision_bytes)
                .expect("canonical decision is valid")
                .canonical_bytes(),
            decision_bytes
        );
    }

    #[test]
    fn canonical_headers_refuse_length_magic_domain_reserve_version_and_payload_length() {
        let canonical = policy().canonical_bytes();
        let truncated = canonical
            .get(..canonical.len() - 1)
            .expect("one-byte-short canonical policy slice exists");
        assert_eq!(
            AdmissionPolicyV1::from_canonical_bytes(truncated),
            Err(AdmissionCanonicalRefusalV1::Length {
                record: AdmissionCanonicalRecordV1::Policy,
                expected: ADMISSION_POLICY_CANONICAL_LEN_V1,
                actual: ADMISSION_POLICY_CANONICAL_LEN_V1 - 1,
            })
        );

        let mut wrong = canonical;
        *wrong.get_mut(0).expect("magic byte exists") ^= 1;
        assert_eq!(
            AdmissionPolicyV1::from_canonical_bytes(&wrong),
            Err(AdmissionCanonicalRefusalV1::Magic {
                record: AdmissionCanonicalRecordV1::Policy,
            })
        );

        let mut wrong = canonical;
        *wrong.get_mut(4).expect("domain byte exists") = 2;
        assert_eq!(
            AdmissionPolicyV1::from_canonical_bytes(&wrong),
            Err(AdmissionCanonicalRefusalV1::Domain {
                record: AdmissionCanonicalRecordV1::Policy,
                actual: 2,
            })
        );

        let mut wrong = canonical;
        *wrong.get_mut(5).expect("reserve byte exists") = 1;
        assert_eq!(
            AdmissionPolicyV1::from_canonical_bytes(&wrong),
            Err(AdmissionCanonicalRefusalV1::Reserved {
                record: AdmissionCanonicalRecordV1::Policy,
                offset: 5,
            })
        );

        let mut wrong = canonical;
        wrong
            .get_mut(6..8)
            .expect("version slot exists")
            .copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            AdmissionPolicyV1::from_canonical_bytes(&wrong),
            Err(AdmissionCanonicalRefusalV1::Version {
                record: AdmissionCanonicalRecordV1::Policy,
                actual: 2,
            })
        );

        let mut wrong = canonical;
        wrong
            .get_mut(8..12)
            .expect("payload-length slot exists")
            .copy_from_slice(&297_u32.to_le_bytes());
        assert_eq!(
            AdmissionPolicyV1::from_canonical_bytes(&wrong),
            Err(AdmissionCanonicalRefusalV1::PayloadLength {
                record: AdmissionCanonicalRecordV1::Policy,
                actual: 297,
            })
        );
    }

    #[test]
    fn policy_decoder_refuses_invalid_boolean_tags_and_invalid_payload_values() {
        const WHITE_REJECTION_TAG_OFFSET: usize = 300;
        const ROMANO_WOLF_REJECTION_TAG_OFFSET: usize = 309;
        let canonical = policy().canonical_bytes();

        for offset in [WHITE_REJECTION_TAG_OFFSET, ROMANO_WOLF_REJECTION_TAG_OFFSET] {
            let mut wrong = canonical;
            *wrong.get_mut(offset).expect("policy boolean tag exists") = 2;
            assert_eq!(
                AdmissionPolicyV1::from_canonical_bytes(&wrong),
                Err(AdmissionCanonicalRefusalV1::Tag {
                    record: AdmissionCanonicalRecordV1::Policy,
                    offset,
                    actual: 2,
                })
            );
        }

        let mut zero_support = canonical;
        zero_support
            .get_mut(12..20)
            .expect("minimum-support slot exists")
            .fill(0);
        let refusal = AdmissionPolicyV1::from_canonical_bytes(&zero_support)
            .expect_err("decoded policy must reuse positive-floor validation");
        assert_eq!(
            refusal,
            AdmissionCanonicalRefusalV1::Policy(super::AdmissionPolicyRefusalV1 {
                field: AdmissionFieldV1::MinSupportHits,
                kind: AdmissionPolicyRefusalKindV1::MustBePositive,
            })
        );
    }

    #[test]
    fn evidence_decoder_refuses_unknown_tags_and_nonzero_payloadless_reserves() {
        const FIRST_OBSERVATION_TAG_OFFSET: usize = 12;
        const FIRST_OBSERVATION_PAYLOAD_OFFSET: usize = 13;
        const EXECUTION_COMPLETENESS_TAG_OFFSET: usize = 165;
        const WHITE_DECISION_TAG_OFFSET: usize = 349;
        let canonical = constructible_evidence().canonical_bytes();

        let mut unknown_observation = canonical;
        *unknown_observation
            .get_mut(FIRST_OBSERVATION_TAG_OFFSET)
            .expect("first observation tag exists") = 3;
        assert_eq!(
            AdmissionEvidenceV1::from_canonical_bytes(&unknown_observation),
            Err(AdmissionCanonicalRefusalV1::Tag {
                record: AdmissionCanonicalRecordV1::Evidence,
                offset: FIRST_OBSERVATION_TAG_OFFSET,
                actual: 3,
            })
        );

        let mut payloadless = canonical;
        *payloadless
            .get_mut(FIRST_OBSERVATION_TAG_OFFSET)
            .expect("first observation tag exists") = 1;
        payloadless
            .get_mut(FIRST_OBSERVATION_PAYLOAD_OFFSET..FIRST_OBSERVATION_PAYLOAD_OFFSET + 8)
            .expect("first observation payload exists")
            .fill(0);
        assert_eq!(
            AdmissionEvidenceV1::from_canonical_bytes(&payloadless)
                .expect("zero-reserved unmeasured observation is canonical")
                .canonical_bytes(),
            payloadless
        );
        *payloadless
            .get_mut(FIRST_OBSERVATION_PAYLOAD_OFFSET)
            .expect("first observation payload byte exists") = 1;
        assert_eq!(
            AdmissionEvidenceV1::from_canonical_bytes(&payloadless),
            Err(AdmissionCanonicalRefusalV1::Reserved {
                record: AdmissionCanonicalRecordV1::Evidence,
                offset: FIRST_OBSERVATION_PAYLOAD_OFFSET,
            })
        );

        for offset in [EXECUTION_COMPLETENESS_TAG_OFFSET, WHITE_DECISION_TAG_OFFSET] {
            let mut wrong = canonical;
            *wrong.get_mut(offset).expect("evidence enum tag exists") = 4;
            assert_eq!(
                AdmissionEvidenceV1::from_canonical_bytes(&wrong),
                Err(AdmissionCanonicalRefusalV1::Tag {
                    record: AdmissionCanonicalRecordV1::Evidence,
                    offset,
                    actual: 4,
                })
            );
        }
    }

    #[test]
    fn verdict_decoder_refuses_unknown_reasons_tags_and_redundancy_forgery() {
        const REASONS_OFFSET: usize = 12;
        const STATUS_OFFSET: usize = 44;
        let canonical = policy()
            .evaluate(&evidence(&passing_values()))
            .canonical_bytes();

        let mut future_reason = canonical;
        *future_reason
            .get_mut(REASONS_OFFSET + 7)
            .expect("highest reasons byte exists") = 0x80;
        assert_eq!(
            AdmissionVerdictV1::from_canonical_bytes(&future_reason),
            Err(AdmissionCanonicalRefusalV1::UnknownReasonBits {
                partition: AdmissionVerdictPartitionV1::Reasons,
            })
        );

        let mut unknown_status = canonical;
        *unknown_status
            .get_mut(STATUS_OFFSET)
            .expect("verdict status tag exists") = 4;
        assert_eq!(
            AdmissionVerdictV1::from_canonical_bytes(&unknown_status),
            Err(AdmissionCanonicalRefusalV1::Tag {
                record: AdmissionCanonicalRecordV1::Verdict,
                offset: STATUS_OFFSET,
                actual: 4,
            })
        );

        let mut false_union = canonical;
        *false_union
            .get_mut(REASONS_OFFSET)
            .expect("verdict reasons byte exists") = 1;
        assert_eq!(
            AdmissionVerdictV1::from_canonical_bytes(&false_union),
            Err(AdmissionCanonicalRefusalV1::VerdictInconsistent)
        );
    }

    #[test]
    fn recomputed_outer_seal_cannot_authorize_a_forged_valid_verdict() {
        const NESTED_VERDICT_OFFSET: usize =
            12 + ADMISSION_POLICY_CANONICAL_LEN_V1 + ADMISSION_EVIDENCE_CANONICAL_LEN_V1;
        let policy = policy();
        let passing_evidence = constructible_evidence();
        let authoritative = policy.evaluate_sealed(&passing_evidence);

        let mut failing_values = constructible_values();
        failing_values.max_mae_paisa = ObservedU64V1::Measured(501);
        let forged_verdict = policy
            .evaluate(&evidence(&failing_values))
            .canonical_bytes();
        let decoded_forged = AdmissionVerdictV1::from_canonical_bytes(&forged_verdict)
            .expect("forged verdict is valid in isolation");
        assert_eq!(decoded_forged.status(), AdmissionStatusV1::Unmeasured);
        assert!(decoded_forged.failed().contains(AdmissionReasonV1::MaxMae));
        assert!(decoded_forged.unmeasured().contains(AdmissionReasonV1::Pbo));
        assert_eq!(
            AdmissionDecisionSealV1::from_canonical_parts(
                &policy.canonical_bytes(),
                &passing_evidence.canonical_bytes(),
                &forged_verdict,
            ),
            Err(AdmissionCanonicalRefusalV1::VerdictMismatch)
        );

        let mut forged_outer = authoritative.canonical_bytes();
        forged_outer
            .get_mut(NESTED_VERDICT_OFFSET..)
            .expect("nested verdict slot exists")
            .copy_from_slice(&forged_verdict);
        let recomputed_attacker_digest = brutex_core::blake3::hash(&forged_outer);
        assert_ne!(recomputed_attacker_digest, authoritative.digest());
        assert_eq!(
            AdmissionDecisionSealV1::from_canonical_bytes(&forged_outer),
            Err(AdmissionCanonicalRefusalV1::VerdictMismatch)
        );
    }

    #[test]
    fn equivalent_values_have_identical_versioned_bytes_and_digests() {
        let first_policy = policy();
        let second_policy = policy();
        let first_evidence = constructible_evidence();
        let second_evidence = constructible_evidence();
        let first = first_policy.evaluate_sealed(&first_evidence);
        let second = second_policy.evaluate_sealed(&second_evidence);

        assert_eq!(
            first_policy.canonical_bytes(),
            second_policy.canonical_bytes()
        );
        assert_eq!(first_policy.digest(), second_policy.digest());
        assert_eq!(
            first_evidence.canonical_bytes(),
            second_evidence.canonical_bytes()
        );
        assert_eq!(first_evidence.digest(), second_evidence.digest());
        assert_eq!(
            first.verdict().canonical_bytes(),
            second.verdict().canonical_bytes()
        );
        assert_eq!(first.verdict().digest(), second.verdict().digest());
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.digest(), second.digest());
        let policy_bytes = first_policy.canonical_bytes();
        let evidence_bytes = first_evidence.canonical_bytes();
        let verdict_bytes = first.verdict().canonical_bytes();
        let decision_bytes = first.canonical_bytes();
        for bytes in [
            policy_bytes.as_slice(),
            evidence_bytes.as_slice(),
            verdict_bytes.as_slice(),
            decision_bytes.as_slice(),
        ] {
            assert!(bytes.starts_with(b"BADM"));
            assert_eq!(
                bytes.get(6..8),
                Some(ADMISSION_VERSION_V1.to_le_bytes().as_slice())
            );
        }
        assert_eq!(policy_bytes.get(4), Some(&1));
        assert_eq!(evidence_bytes.get(4), Some(&2));
        assert_eq!(verdict_bytes.get(4), Some(&3));
        assert_eq!(decision_bytes.get(4), Some(&4));
        assert_ne!(first_policy.digest(), first_evidence.digest());
        assert_ne!(first_evidence.digest(), first.verdict().digest());
        assert_ne!(first.verdict().digest(), first.digest());
    }

    #[test]
    fn stable_bits_names_version_and_fixed_layout_are_pinned() {
        assert_eq!(ADMISSION_VERSION_V1, 1);
        assert_eq!(AdmissionFieldV1::ALL.len(), 39);
        assert_eq!(AdmissionReasonV1::ALL.len(), 44);
        for (index, reason) in AdmissionReasonV1::ALL.into_iter().enumerate() {
            assert_eq!(ReasonBits::one(reason).bits(), 1_u64 << index);
            assert!(!reason.name().is_empty());
        }
        assert!(ReasonBits::from_bits(ReasonBits::KNOWN.bits()).is_some());
        assert!(ReasonBits::from_bits(1_u64 << 63).is_none());
        assert_eq!(core::mem::size_of::<AdmissionPolicyV1>(), 304);
        assert_eq!(core::mem::size_of::<AdmissionEvidenceV1>(), 608);
        assert_eq!(core::mem::size_of::<ReasonBits>(), 8);
        assert_eq!(core::mem::size_of::<AdmissionVerdictV1>(), 40);
        assert_eq!(
            policy().canonical_bytes().len(),
            ADMISSION_POLICY_CANONICAL_LEN_V1
        );
        assert_eq!(
            evidence(&passing_values()).canonical_bytes().len(),
            ADMISSION_EVIDENCE_CANONICAL_LEN_V1
        );
        assert_eq!(
            policy()
                .evaluate(&evidence(&passing_values()))
                .canonical_bytes()
                .len(),
            ADMISSION_VERDICT_CANONICAL_LEN_V1
        );
        assert_eq!(
            policy()
                .evaluate_sealed(&evidence(&passing_values()))
                .canonical_bytes()
                .len(),
            ADMISSION_DECISION_CANONICAL_LEN_V1
        );
    }

    #[test]
    fn admission_v2_exact_alpha_includes_the_five_percent_boundary() {
        for (numerator, expected) in [(49, true), (50, true), (51, false)] {
            let probability = AdmissionExactProbabilityV2::new(numerator, 1_000)
                .expect("boundary probability is valid");
            assert_eq!(probability.rejects_at_ppm(50_000), expected);
            assert_eq!(
                hypothesis_decision_v2(probability),
                if expected {
                    HypothesisDecisionV1::RejectedNull
                } else {
                    HypothesisDecisionV1::DidNotReject
                }
            );
        }
    }

    #[test]
    fn admission_v2_wilson_is_recomputed_from_exact_trade_counts() {
        let statistics = statistics_v2(1);
        assert!(AdmissionStatisticsFieldsV2::verify_detached(&statistics).is_ok());

        let mut wrong_bits = statistics;
        wrong_bits.wilson_lower_bits = 1.0_f64.to_bits();
        assert_eq!(
            AdmissionStatisticsFieldsV2::verify_detached(&wrong_bits),
            Err(AdmissionStatisticsRefusalV2::WilsonProjection)
        );

        let mut wrong_ppm = statistics;
        wrong_ppm.wilson_win_rate_ppm = wrong_ppm.wilson_win_rate_ppm.saturating_add(1);
        assert_eq!(
            AdmissionStatisticsFieldsV2::verify_detached(&wrong_ppm),
            Err(AdmissionStatisticsRefusalV2::WilsonProjection)
        );

        let mut zero = statistics;
        zero.trades = 0;
        zero.wins = 0;
        (zero.wilson_lower_bits, zero.wilson_win_rate_ppm) = canonical_wilson_projection_v2(0, 0);
        assert!(AdmissionStatisticsFieldsV2::verify_detached(&zero).is_ok());
    }

    #[test]
    fn admission_v2_public_surface_is_a_recomputable_non_authoritative_projection() {
        let (policy, base, anchored, statistics) = v2_parts();
        let walk = AnchoredWalkForwardAuthorityV2::from_opaque(&anchored)
            .expect("opaque fixture revalidates");
        let projection = policy
            .evaluate_v2_projection(&base, &statistics, &anchored)
            .expect("detached fixture arithmetic reconciles");
        let comparison = projection.comparison_values();
        assert_eq!(
            comparison.decided_folds,
            ObservedU64V1::Measured(walk.decided_folds())
        );
        assert_eq!(
            comparison.profitable_oos_folds,
            ObservedU64V1::Measured(walk.profitable_oos_folds())
        );
        assert_eq!(
            comparison.oos_pessimistic_return_paisa,
            ObservedI64V1::Measured(walk.aggregate_oos_paisa())
        );
        assert_eq!(
            comparison.full_precision_statistics_complete,
            CompletenessV1::Complete
        );
        let projected_verdict =
            AdmissionVerdictV1::from_canonical_bytes(&projection.verdict_bytes())
                .expect("projection verdict bytes are canonical");
        assert_eq!(projection.status(), projected_verdict.status());

        let evidence = v2_evidence(&base, &anchored, &statistics);
        let decision = policy.evaluate_v2_record(&evidence);
        assert_eq!(projected_verdict, decision.verdict());
        let decision_bytes = decision.canonical_bytes();
        assert_eq!(projection.decision_bytes(), decision_bytes);
        let decoded =
            AdmissionV2ArithmeticProjection::verify_decision_record_detached(&decision_bytes)
                .expect("detached decision record revalidates");
        assert_eq!(decoded, projection);
        assert!(decision.recomputes());
        assert_eq!(
            projection.evidence_bytes().len(),
            ADMISSION_EVIDENCE_CANONICAL_LEN_V2
        );
        assert_eq!(decision_bytes.len(), ADMISSION_DECISION_CANONICAL_LEN_V2);
        assert_eq!(projection.evidence_bytes().get(4), Some(&2));
        assert_eq!(decision_bytes.get(4), Some(&4));
        assert_eq!(
            projection.evidence_bytes().get(6..8),
            Some(ADMISSION_VERSION_V2.to_le_bytes().as_slice())
        );
        assert_eq!(
            decision_bytes.get(6..8),
            Some(ADMISSION_VERSION_V2.to_le_bytes().as_slice())
        );
        assert_eq!(ADMISSION_VERSION_V1, 1);
    }

    #[test]
    fn admission_v2_walk_never_copies_detached_statistics_lineage() {
        let (_, base, anchored, first_statistics) = v2_parts();
        let first_fields = AdmissionStatisticsFieldsV2::verify_detached(&first_statistics)
            .expect("first statistics reconcile");
        let walk =
            AnchoredWalkForwardAuthorityV2::from_opaque(&anchored).expect("opaque walk reconciles");

        let second_statistics = statistics_v2(2);
        let second_fields = AdmissionStatisticsFieldsV2::verify_detached(&second_statistics)
            .expect("second candidate statistics reconcile");
        let shared = AdmissionEvidenceV2::new(base, &second_fields, walk)
            .expect("search validation is not relabelled as candidate evidence");
        assert_ne!(
            shared.statistics().candidate_semantic_id(),
            first_fields.candidate_semantic_id()
        );

        let mut foreign = second_statistics;
        foreign.population_search_id = [21; 32];
        foreign.ranking_validation_policy_digest = [22; 32];
        foreign.source_policy_digest = [23; 32];
        foreign.finalization_family_digest = [24; 32];
        let foreign_fields = AdmissionStatisticsFieldsV2::verify_detached(&foreign)
            .expect("nonzero detached lineage is arithmetically well formed");
        let foreign_evidence = AdmissionEvidenceV2::new(base, &foreign_fields, walk)
            .expect("Runner retains but does not authenticate CLI lineage");
        assert_eq!(foreign_evidence.walk_forward(), shared.walk_forward());
        assert_ne!(foreign_evidence.canonical_bytes(), shared.canonical_bytes());
    }

    #[test]
    fn admission_v2_canonical_walk_count_and_outcome_hierarchies_fail_closed() {
        let (_, _, anchored, _) = v2_parts();
        let valid = AnchoredWalkForwardAuthorityV2::from_opaque(&anchored)
            .expect("fixture walk reconciles");

        let count = AnchoredWalkForwardAuthorityV2 {
            fold_count: 0,
            authority_id: [0; 32],
            ..valid
        }
        .with_derived_authority_id();
        assert_eq!(
            AnchoredWalkForwardAuthorityV2::from_canonical_fields(count),
            Err(AnchoredWalkForwardRefusalV2::CountHierarchy)
        );

        for impossible in [
            AnchoredWalkForwardAuthorityV2 {
                decided_folds: 0,
                profitable_oos_folds: 0,
                aggregate_oos_paisa: 1,
                authority_id: [0; 32],
                ..valid
            },
            AnchoredWalkForwardAuthorityV2 {
                decided_folds: 1,
                profitable_oos_folds: 0,
                aggregate_oos_paisa: 1,
                authority_id: [0; 32],
                ..valid
            },
            AnchoredWalkForwardAuthorityV2 {
                decided_folds: 1,
                profitable_oos_folds: 1,
                aggregate_oos_paisa: 0,
                authority_id: [0; 32],
                ..valid
            },
        ] {
            assert_eq!(
                AnchoredWalkForwardAuthorityV2::from_canonical_fields(
                    impossible.with_derived_authority_id()
                ),
                Err(AnchoredWalkForwardRefusalV2::OutcomeHierarchy)
            );
        }
    }

    #[test]
    fn admission_v3_walk_authority_refuses_v2_identity_and_invalid_hierarchies() {
        let (_, _, anchored_v2, _) = v2_parts();
        let anchored_v3 = anchored_v3();
        let v2 = AnchoredWalkForwardAuthorityV2::from_opaque(&anchored_v2)
            .expect("fixture V2 walk reconciles");
        let valid = AnchoredWalkForwardAuthorityV3::from_opaque(&anchored_v3)
            .expect("fixture bilateral V3 walk reconciles");
        assert_ne!(valid.authority_id(), v2.authority_id());

        let cross_version = AnchoredWalkForwardAuthorityV3 {
            authority_id: v2.authority_id(),
            ..valid
        };
        assert_eq!(
            AnchoredWalkForwardAuthorityV3::from_canonical_fields(cross_version),
            Err(AnchoredWalkForwardRefusalV3::AuthorityIdentityMismatch)
        );

        let invalid_count = AnchoredWalkForwardAuthorityV3 {
            fold_count: 0,
            authority_id: [0; 32],
            ..valid
        }
        .with_derived_authority_id();
        assert_eq!(
            AnchoredWalkForwardAuthorityV3::from_canonical_fields(invalid_count),
            Err(AnchoredWalkForwardRefusalV3::CountHierarchy)
        );

        let invalid_outcome = AnchoredWalkForwardAuthorityV3 {
            decided_folds: 0,
            profitable_oos_folds: 0,
            aggregate_oos_paisa: 1,
            authority_id: [0; 32],
            ..valid
        }
        .with_derived_authority_id();
        assert_eq!(
            AnchoredWalkForwardAuthorityV3::from_canonical_fields(invalid_outcome),
            Err(AnchoredWalkForwardRefusalV3::OutcomeHierarchy)
        );
    }

    #[test]
    fn admission_v2_codecs_refuse_crosswires_tampering_and_forged_verdicts() {
        const STATS_POPULATION_OFFSET: usize = 12 + 340 + 32;
        const STATS_WILSON_BITS_OFFSET: usize = 12 + 340 + 13 * 32 + 2 * 8;
        const WALK_OFFSET: usize = 12 + 340 + 13 * 32 + 7 * 8 + 5 * 16 + 3 * 8;
        const WALK_FOLD_COUNT_OFFSET: usize = WALK_OFFSET + 4 * 32;
        const WALK_DECIDED_OFFSET: usize = WALK_FOLD_COUNT_OFFSET + 8;
        const WALK_PROFITABLE_OFFSET: usize = WALK_FOLD_COUNT_OFFSET + 2 * 8;
        const WALK_AGGREGATE_OFFSET: usize = WALK_FOLD_COUNT_OFFSET + 3 * 8;

        let (policy, base, anchored, statistics) = v2_parts();
        let evidence = v2_evidence(&base, &anchored, &statistics);
        let decision = policy.evaluate_v2_record(&evidence);
        let evidence_bytes = evidence.canonical_bytes();
        let decision_bytes = decision.canonical_bytes();
        assert_eq!(
            AdmissionEvidenceV2::from_canonical_bytes(&evidence_bytes),
            Ok(evidence)
        );
        assert_eq!(
            AdmissionDecisionV2::from_canonical_bytes(&decision_bytes),
            Ok(decision)
        );
        assert_eq!(
            AdmissionEvidenceV2::from_canonical_bytes(
                evidence_bytes
                    .get(..ADMISSION_EVIDENCE_CANONICAL_LEN_V2 - 1)
                    .expect("fixed evidence has a one-byte-short prefix"),
            ),
            Err(AdmissionCanonicalRefusalV2::Length {
                record: AdmissionCanonicalRecordV2::Evidence,
                expected: ADMISSION_EVIDENCE_CANONICAL_LEN_V2,
                actual: ADMISSION_EVIDENCE_CANONICAL_LEN_V2 - 1,
            })
        );

        let mut wrong_version = decision_bytes;
        wrong_version[6..8].copy_from_slice(&3_u16.to_le_bytes());
        assert_eq!(
            AdmissionDecisionV2::from_canonical_bytes(&wrong_version),
            Err(AdmissionCanonicalRefusalV2::Version {
                record: AdmissionCanonicalRecordV2::Decision,
                actual: 3,
            })
        );

        let mut foreign_population = evidence_bytes;
        foreign_population[STATS_POPULATION_OFFSET] ^= 1;
        let detached = AdmissionEvidenceV2::from_canonical_bytes(&foreign_population)
            .expect("detached Runner codec retains but does not authenticate CLI lineage");
        assert_ne!(detached, evidence);
        assert_eq!(detached.walk_forward(), evidence.walk_forward());

        let mut false_wilson = evidence_bytes;
        false_wilson[STATS_WILSON_BITS_OFFSET] ^= 1;
        assert_eq!(
            AdmissionEvidenceV2::from_canonical_bytes(&false_wilson),
            Err(AdmissionCanonicalRefusalV2::Statistics(
                AdmissionStatisticsRefusalV2::WilsonProjection
            ))
        );

        let mut zero_folds = evidence_bytes;
        zero_folds[WALK_FOLD_COUNT_OFFSET..WALK_FOLD_COUNT_OFFSET + 8]
            .copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            AdmissionEvidenceV2::from_canonical_bytes(&zero_folds),
            Err(AdmissionCanonicalRefusalV2::WalkForward(
                AnchoredWalkForwardRefusalV2::CountHierarchy
            ))
        );

        let mut impossible_outcome = evidence_bytes;
        impossible_outcome[WALK_DECIDED_OFFSET..WALK_DECIDED_OFFSET + 8]
            .copy_from_slice(&0_u64.to_le_bytes());
        impossible_outcome[WALK_PROFITABLE_OFFSET..WALK_PROFITABLE_OFFSET + 8]
            .copy_from_slice(&0_u64.to_le_bytes());
        impossible_outcome[WALK_AGGREGATE_OFFSET..WALK_AGGREGATE_OFFSET + 8]
            .copy_from_slice(&1_i64.to_le_bytes());
        assert_eq!(
            AdmissionEvidenceV2::from_canonical_bytes(&impossible_outcome),
            Err(AdmissionCanonicalRefusalV2::WalkForward(
                AnchoredWalkForwardRefusalV2::OutcomeHierarchy
            ))
        );

        let mut failing_base = base;
        failing_base.max_mae_paisa = ObservedU64V1::Measured(501);
        let failing_evidence = v2_evidence(&failing_base, &anchored, &statistics);
        let forged_verdict = policy.evaluate_v2(&failing_evidence).canonical_bytes();
        assert_eq!(
            AdmissionDecisionV2::from_canonical_parts(
                &policy.canonical_bytes(),
                &evidence_bytes,
                &forged_verdict,
            ),
            Err(AdmissionCanonicalRefusalV2::VerdictMismatch)
        );
    }

    #[test]
    fn admission_v3_public_projection_is_version_separated_and_recomputable() {
        let (policy, base, anchored_v2, statistics_v2) = v2_parts();
        let anchored_v3 = anchored_v3();
        let statistics_v3 = statistics_v3(1);
        let api: fn(
            AdmissionPolicyV1,
            &AdmissionEvidenceValuesV1,
            &AdmissionStatisticsDraftV3,
            &AnchoredSearchValidationV3,
        ) -> Result<
            AdmissionV3ArithmeticProjection,
            super::AdmissionV3ArithmeticRefusal,
        > = AdmissionPolicyV1::evaluate_v3_projection;
        let v2 = policy
            .evaluate_v2_projection(&base, &statistics_v2, &anchored_v2)
            .expect("V2 arithmetic reconciles");
        let v3 =
            api(policy, &base, &statistics_v3, &anchored_v3).expect("V3 arithmetic reconciles");

        assert_eq!(v3.comparison_values().trades, v2.comparison_values().trades);
        assert_eq!(
            v3.comparison_values().winning_trades,
            v2.comparison_values().winning_trades
        );
        assert_eq!(
            v3.evidence_bytes().len(),
            ADMISSION_EVIDENCE_CANONICAL_LEN_V3
        );
        assert_eq!(
            v3.decision_bytes().len(),
            ADMISSION_DECISION_CANONICAL_LEN_V3
        );
        assert_eq!(
            v3.evidence_bytes()[6..8],
            ADMISSION_VERSION_V3.to_le_bytes()
        );
        assert_eq!(
            v3.decision_bytes()[6..8],
            ADMISSION_VERSION_V3.to_le_bytes()
        );
        assert_eq!(
            ADMISSION_EVIDENCE_CANONICAL_LEN_V2 - ADMISSION_EVIDENCE_CANONICAL_LEN_V3,
            128
        );
        assert_eq!(
            ADMISSION_DECISION_CANONICAL_LEN_V2 - ADMISSION_DECISION_CANONICAL_LEN_V3,
            128
        );
        assert_ne!(
            v3.evidence_bytes().as_slice(),
            v2.evidence_bytes().as_slice()
        );

        let evidence_v2 = v2_evidence(&base, &anchored_v2, &statistics_v2);
        let evidence = v3_evidence(&base, &anchored_v3, &statistics_v3);
        assert_ne!(
            evidence.walk_forward().authority_id(),
            evidence_v2.walk_forward().authority_id()
        );
        let decision = policy.evaluate_v3_record(&evidence);
        assert_eq!(decision.verdict(), policy.evaluate_v3(&evidence));
        assert!(decision.recomputes());
        assert_eq!(
            AdmissionV3ArithmeticProjection::verify_decision_record_detached(
                &decision.canonical_bytes()
            ),
            Ok(v3)
        );
    }

    #[test]
    fn admission_v3_public_detached_verifier_round_trips_and_refuses_mutations() {
        const EVIDENCE_STATISTICS_WILSON_BITS_OFFSET: usize = 12 + 340 + 9 * 32 + 2 * 8;
        const DECISION_EVIDENCE_OFFSET: usize = 12 + ADMISSION_POLICY_CANONICAL_LEN_V1;
        const DECISION_VERDICT_OFFSET: usize =
            DECISION_EVIDENCE_OFFSET + ADMISSION_EVIDENCE_CANONICAL_LEN_V3;

        let (policy, base, _, _) = v2_parts();
        let anchored = anchored_v3();
        let statistics = statistics_v3(1);
        let expected = policy
            .evaluate_v3_projection(&base, &statistics, &anchored)
            .expect("valid detached V3 arithmetic");
        let decision_bytes = expected.decision_bytes();
        let verifier_fn: fn(
            &[u8],
        ) -> Result<
            AdmissionV3ArithmeticProjection,
            AdmissionCanonicalRefusalV3,
        > = AdmissionV3ArithmeticProjection::verify_decision_record_detached;

        let reopened = verifier_fn(&decision_bytes).expect("exact V3 bytes revalidate");
        assert_eq!(reopened, expected);
        assert_eq!(reopened.decision_bytes(), decision_bytes);
        assert_eq!(reopened.evidence_bytes(), expected.evidence_bytes());
        assert_eq!(reopened.verdict_bytes(), expected.verdict_bytes());
        assert_eq!(reopened.verdict(), expected.verdict());
        assert_eq!(reopened.comparison_values(), expected.comparison_values());
        assert_eq!(reopened.status(), expected.status());

        let mut decision_mutation = decision_bytes;
        decision_mutation[6..8].copy_from_slice(&ADMISSION_VERSION_V2.to_le_bytes());
        assert_eq!(
            verifier_fn(&decision_mutation),
            Err(AdmissionCanonicalRefusalV3::Version {
                record: AdmissionCanonicalRecordV3::Decision,
                actual: ADMISSION_VERSION_V2,
            })
        );

        let mut evidence_mutation = decision_bytes;
        evidence_mutation[DECISION_EVIDENCE_OFFSET + EVIDENCE_STATISTICS_WILSON_BITS_OFFSET] ^= 1;
        assert_eq!(
            verifier_fn(&evidence_mutation),
            Err(AdmissionCanonicalRefusalV3::Statistics(
                AdmissionStatisticsRefusalV3::WilsonProjection
            ))
        );

        let mut failing_base = base;
        failing_base.max_mae_paisa = ObservedU64V1::Measured(501);
        let different_valid_verdict = policy
            .evaluate_v3(&v3_evidence(&failing_base, &anchored, &statistics))
            .canonical_bytes();
        let mut verdict_mutation = decision_bytes;
        verdict_mutation[DECISION_VERDICT_OFFSET..].copy_from_slice(&different_valid_verdict);
        assert_eq!(
            verifier_fn(&verdict_mutation),
            Err(AdmissionCanonicalRefusalV3::VerdictMismatch)
        );
    }

    #[test]
    fn admission_v3_statistics_codec_contains_exactly_nine_provable_identities() {
        const STATISTICS_OFFSET: usize = 12 + 340;
        let (_, base, _, _) = v2_parts();
        let anchored = anchored_v3();
        let mut statistics = statistics_v3(1);
        for (index, identity) in [
            &mut statistics.candidate_semantic_id,
            &mut statistics.statistics_audit_id,
            &mut statistics.statistics_completion_digest,
            &mut statistics.observation_statistics_link_id,
            &mut statistics.cscv_policy_digest,
            &mut statistics.cscv_split_family_digest,
            &mut statistics.white_family_digest,
            &mut statistics.spa_family_digest,
            &mut statistics.romano_wolf_family_digest,
        ]
        .into_iter()
        .enumerate()
        {
            identity.fill(u8::try_from(index + 1).expect("nine identities fit in u8"));
        }

        let bytes = v3_evidence(&base, &anchored, &statistics).canonical_bytes();
        for index in 0..9 {
            let start = STATISTICS_OFFSET + index * 32;
            assert_eq!(
                bytes.get(start..start + 32),
                Some([u8::try_from(index + 1).expect("nine identities fit in u8"); 32].as_slice())
            );
        }

        let mut second_v2 = statistics_v2(1);
        second_v2.population_search_id = [91; 32];
        second_v2.ranking_validation_policy_digest = [92; 32];
        second_v2.source_policy_digest = [93; 32];
        second_v2.finalization_family_digest = [94; 32];
        let first_v3 = statistics_v3(1);
        let second_v3 = AdmissionStatisticsDraftV3 {
            candidate_semantic_id: second_v2.candidate_semantic_id,
            statistics_audit_id: second_v2.statistics_audit_id,
            statistics_completion_digest: second_v2.statistics_completion_digest,
            observation_statistics_link_id: second_v2.observation_statistics_link_id,
            cscv_policy_digest: second_v2.cscv_policy_digest,
            cscv_split_family_digest: second_v2.cscv_split_family_digest,
            white_family_digest: second_v2.white_family_digest,
            spa_family_digest: second_v2.spa_family_digest,
            romano_wolf_family_digest: second_v2.romano_wolf_family_digest,
            trades: second_v2.trades,
            wins: second_v2.wins,
            wilson_lower_bits: second_v2.wilson_lower_bits,
            wilson_win_rate_ppm: second_v2.wilson_win_rate_ppm,
            cscv_split_count: second_v2.cscv_split_count,
            pbo_contributing_splits: second_v2.pbo_contributing_splits,
            pbo_unrankable_splits: second_v2.pbo_unrankable_splits,
            pbo_probability: second_v2.pbo_probability,
            white_probability: second_v2.white_probability,
            spa_probability: second_v2.spa_probability,
            familywise_romano_wolf_probability: second_v2.familywise_romano_wolf_probability,
            candidate_romano_wolf_probability: second_v2.candidate_romano_wolf_probability,
            bootstrap_draws: second_v2.bootstrap_draws,
            bootstrap_strategies: second_v2.bootstrap_strategies,
            bootstrap_periods: second_v2.bootstrap_periods,
        };
        assert_eq!(first_v3, second_v3);
    }

    #[test]
    fn admission_v3_names_every_zero_identity_and_rejects_numeric_crosswires() {
        type Setter = fn(&mut AdmissionStatisticsDraftV3);
        let cases: [(AdmissionStatisticsIdentityV3, Setter); 9] = [
            (AdmissionStatisticsIdentityV3::CandidateSemantic, |value| {
                value.candidate_semantic_id = [0; 32];
            }),
            (AdmissionStatisticsIdentityV3::StatisticsAudit, |value| {
                value.statistics_audit_id = [0; 32];
            }),
            (
                AdmissionStatisticsIdentityV3::StatisticsCompletion,
                |value| value.statistics_completion_digest = [0; 32],
            ),
            (
                AdmissionStatisticsIdentityV3::ObservationStatisticsLink,
                |value| value.observation_statistics_link_id = [0; 32],
            ),
            (AdmissionStatisticsIdentityV3::CscvPolicy, |value| {
                value.cscv_policy_digest = [0; 32];
            }),
            (AdmissionStatisticsIdentityV3::CscvSplitFamily, |value| {
                value.cscv_split_family_digest = [0; 32];
            }),
            (AdmissionStatisticsIdentityV3::WhiteFamily, |value| {
                value.white_family_digest = [0; 32];
            }),
            (AdmissionStatisticsIdentityV3::SpaFamily, |value| {
                value.spa_family_digest = [0; 32];
            }),
            (AdmissionStatisticsIdentityV3::RomanoWolfFamily, |value| {
                value.romano_wolf_family_digest = [0; 32];
            }),
        ];
        for (identity, mutate) in cases {
            let mut statistics = statistics_v3(1);
            mutate(&mut statistics);
            assert_eq!(
                AdmissionStatisticsFieldsV3::verify_detached(&statistics),
                Err(AdmissionStatisticsRefusalV3::ZeroIdentity(identity))
            );
        }

        let mut statistics = statistics_v3(1);
        statistics.wins = statistics.trades + 1;
        assert_eq!(
            AdmissionStatisticsFieldsV3::verify_detached(&statistics),
            Err(AdmissionStatisticsRefusalV3::WinningTradesExceedTrades)
        );
        let mut statistics = statistics_v3(1);
        statistics.wilson_win_rate_ppm += 1;
        assert_eq!(
            AdmissionStatisticsFieldsV3::verify_detached(&statistics),
            Err(AdmissionStatisticsRefusalV3::WilsonProjection)
        );
        let mut statistics = statistics_v3(1);
        statistics.white_probability =
            AdmissionExactProbabilityV2::new(1, statistics.bootstrap_draws)
                .expect("deliberately wrong but finite denominator");
        assert!(matches!(
            AdmissionStatisticsFieldsV3::verify_detached(&statistics),
            Err(AdmissionStatisticsRefusalV3::FiniteResampleDenominator { .. })
        ));
        let mut statistics = statistics_v3(1);
        statistics.pbo_unrankable_splits += 1;
        assert_eq!(
            AdmissionStatisticsFieldsV3::verify_detached(&statistics),
            Err(AdmissionStatisticsRefusalV3::CscvCounts)
        );
        let mut statistics = statistics_v3(1);
        statistics.familywise_romano_wolf_probability =
            AdmissionExactProbabilityV2::new(51, 1_001).expect("valid exact probability");
        assert_eq!(
            AdmissionStatisticsFieldsV3::verify_detached(&statistics),
            Err(AdmissionStatisticsRefusalV3::RomanoWolfOrdering)
        );
    }

    #[test]
    fn admission_v3_codecs_reject_cross_version_tampering_and_forged_verdicts() {
        const STATS_WILSON_BITS_OFFSET: usize = 12 + 340 + 9 * 32 + 2 * 8;
        const WALK_OFFSET: usize = 12 + 340 + 9 * 32 + 7 * 8 + 5 * 16 + 3 * 8;
        const WALK_FOLD_COUNT_OFFSET: usize = WALK_OFFSET + 4 * 32;

        let (policy, base, anchored_v2, statistics_v2) = v2_parts();
        let anchored_v3 = anchored_v3();
        let statistics = statistics_v3(1);
        let evidence = v3_evidence(&base, &anchored_v3, &statistics);
        let decision = policy.evaluate_v3_record(&evidence);
        let evidence_bytes = evidence.canonical_bytes();
        let decision_bytes = decision.canonical_bytes();
        assert_eq!(
            AdmissionEvidenceV3::from_canonical_bytes(&evidence_bytes),
            Ok(evidence)
        );
        assert_eq!(
            AdmissionDecisionV3::from_canonical_bytes(&decision_bytes),
            Ok(decision)
        );

        assert!(matches!(
            AdmissionEvidenceV2::from_canonical_bytes(&evidence_bytes),
            Err(AdmissionCanonicalRefusalV2::Length { .. })
        ));
        let v2_evidence_bytes = v2_evidence(&base, &anchored_v2, &statistics_v2).canonical_bytes();
        assert!(matches!(
            AdmissionEvidenceV3::from_canonical_bytes(&v2_evidence_bytes),
            Err(AdmissionCanonicalRefusalV3::Length { .. })
        ));

        let mut wrong_version = decision_bytes;
        wrong_version[6..8].copy_from_slice(&ADMISSION_VERSION_V2.to_le_bytes());
        assert_eq!(
            AdmissionDecisionV3::from_canonical_bytes(&wrong_version),
            Err(AdmissionCanonicalRefusalV3::Version {
                record: AdmissionCanonicalRecordV3::Decision,
                actual: ADMISSION_VERSION_V2,
            })
        );
        let mut false_wilson = evidence_bytes;
        false_wilson[STATS_WILSON_BITS_OFFSET] ^= 1;
        assert_eq!(
            AdmissionEvidenceV3::from_canonical_bytes(&false_wilson),
            Err(AdmissionCanonicalRefusalV3::Statistics(
                AdmissionStatisticsRefusalV3::WilsonProjection
            ))
        );
        let mut zero_folds = evidence_bytes;
        zero_folds[WALK_FOLD_COUNT_OFFSET..WALK_FOLD_COUNT_OFFSET + 8]
            .copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            AdmissionEvidenceV3::from_canonical_bytes(&zero_folds),
            Err(AdmissionCanonicalRefusalV3::WalkForward(
                AnchoredWalkForwardRefusalV3::CountHierarchy
            ))
        );

        let mut failing_base = base;
        failing_base.max_mae_paisa = ObservedU64V1::Measured(501);
        let failing_evidence = v3_evidence(&failing_base, &anchored_v3, &statistics);
        let forged_verdict = policy.evaluate_v3(&failing_evidence).canonical_bytes();
        assert_eq!(
            AdmissionDecisionV3::from_canonical_parts(
                &policy.canonical_bytes(),
                &evidence_bytes,
                &forged_verdict,
            ),
            Err(AdmissionCanonicalRefusalV3::VerdictMismatch)
        );
    }
}
