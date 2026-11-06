# CLOUD.md: rules for the PSRB preview reference session (Fri 6 Nov 2026)

- Build an INDEPENDENT reference implementation of PREVIEW-SPEC.md (with OPERATOR-ADDENDUM-20261106.md: three rule
  sets VIDEO-ORIGINAL, AUTHOR-V2, VIDEO-SCREEN; stop loss and exit always the Supertrend) from the text alone, on
  SYNTHETIC data only. Never copy the Mac tool; it is not in this repository.
- TEST-PLAN.md names the functions, tests, extreme cases and benches to cover.
- Rust only; never write, run or name the programming language whose six-letter name starts with "Py".
- Exact integers (paise, basis points, i128 where sums grow); no floating point in any decision.
- Tracked files only .rs, .toml and .md; golden files as plain-text tables in .txt.md; never .json.
- No credentials, no private values, no real vendor rows: every price and trade here is generated with a fixed seed.
- Push after every step; before each push compare the OWNER file with your TOKEN; never force-push, merge, or touch
  any branch but your own.
