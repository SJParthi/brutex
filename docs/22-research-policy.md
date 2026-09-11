# Runtime research policy and the 37 settings

The operator delegated the missing threshold choices on 7 September 2026.
The supplied [research profile](../config/intraday-research-v1.toml) answers all
37 fields. The common resolver supplies two further fields: the run's explicit
research loss limit and an inherited minimum reward-to-loss ratio. That ratio
defaults to 3,000,000 ppm (3:1), unless
`BRUTEX_ADMIT_MIN_WORST_REWARD_RISK_PPM` overrides it. The effective values and
policy digest, rather than the file alone, identify the applied policy.

For the NIFTY/BANKNIFTY single-stop workflow, this ratio measures the smallest
positive pessimistic trade divided by the largest pessimistic trade loss in
the later evaluation. It does not set the signal-candle stop and does not
follow from the operator's separate 60% winning-day and weekly requirements.
A missing positive winner or a zero loss denominator is unmeasured; it is not
an invented infinite ratio or an automatic pass. The Backtest page displays
the effective gate when the policy is available. No threshold is changed by
this clarification.

Read the [plain-language comparison and source record](research-policy/report-source.md)
or open the [searchable guide](../web/policy-guide/index.html).

`cli policy-check FILE MAX_POINTS` validates and explains one explicit file
without computing on market data. `BRUTEX_ADMISSION_POLICY_FILE` selects that
file for the existing institutional route. A missing selector retains the
explicit-field behavior; a selected but malformed file refuses. Existing named
gate overrides remain visible and take precedence. The resolved policy remains
subject to the runner's complete 39-field validation.

This is a research profile, not institutional certification, trading advice, a
completed campaign, or a claim of universal statistical validity. Intraday-only
execution and the 15:10 IST deadline remain mandatory. Changing the profile
after inspecting winners is further research and cannot count as untouched
confirmation. The live service has not been restarted by this change.
