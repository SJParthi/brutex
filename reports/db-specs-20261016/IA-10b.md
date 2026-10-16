# IA-10b: API, verification half: /verify/*, the dayflags verdict key, the pull keep-and-spool hook

Prerequisites (must be merged into feat/db-fix first): IA-10a, IA-6, IA-7, IA-9, IA-16, WP-11c, WP-39, WP-13d, WP-62

## Specification

```text
ROUND 3 ADDITION (12 Oct): Three things not in your section of the rules audit. (1) The run script did not split you: you are also the critic's IA-10c (the Zerodha half, D-4919); read the audit's "IA-10c" section and take its rows. (2) R3-13 owns dayflags.rs before you (the band at the grid's timeframe, a rules module, the pass-through of day fields it does not know): you now wait for it; add your verdict key beside its calls. (3) The open-interest-normalising repair of DB-A08 and DB-A14 is not yours: D-3322 says the stored bytes are not rewritten, and whether they ever are is the operator's decision (the draft gave it to you "only if the operator asks", which is not a goal a builder can meet). ANSWERS: RA-087; RA-193; RA-219
```
