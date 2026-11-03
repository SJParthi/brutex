# DECISIONS.md: decision ids of the 3 Nov 2026 BASE cloud sessions

Take ids only from your own block, in order. Shards (LS1-LS4, PS1-PS3) and MS1 put decision drafts in their report only; LSA and PSA write the docs entries (docs/04, 05, 06) from those drafts, with the ids below.

| session | ids | source |
|---|---|---|
| BASE-COV-LIBS-LS1 | D-8560, D-8561 | block D-8560..D-8569, registered by the main session in `state/decision-blocks.json` on 3 Nov 2026 for COV-LIBS attempt 5 (D-7320..D-7329 are used up) |
| BASE-COV-LIBS-LS2 | D-8562, D-8563 | the same block |
| BASE-COV-LIBS-LS3 | D-8564, D-8565 | the same block |
| BASE-COV-LIBS-LS4 | D-8566, D-8567 | the same block |
| BASE-COV-LIBS-LSA | D-8568, D-8569 | the same block |
| BASE-COV-PULL-PS1 | D-7335 | COV-PULL's block D-7330..D-7339: D-7330..D-7334 are used at f98a899398f6 (`git grep '^#+ D-73[34][0-9]'`), D-7335..D-7339 are free |
| BASE-COV-PULL-PS2 | D-7336 | the same block |
| BASE-COV-PULL-PS3 | D-7337 | the same block |
| BASE-COV-PULL-PSA | D-7338, D-7339 | the same block |
| BASE-MSUM-BACKFILL-MS1 | D-7430..D-7439 | MSUM's block (none used at 4d7f446f7a81); drafts in your report, not in docs |

An id outside your block, or one already present in the docs at your START, is a STOP: write `STATUS: BLOCKED QUESTION` with the clash.
