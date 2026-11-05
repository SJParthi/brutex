# DECISIONS.md: decision ids of the 5 Nov 2026 BASE-COV-API cloud sessions

Take ids only from your own row, in order. Shards (AS1-AS3) put decision drafts in their report only; ASA writes the docs entries (docs/04, 05, 06) from those drafts, with the ids below.

| session | ids | source |
|---|---|---|
| BASE-COV-API-AS1 | D-8660, D-8661 | block D-8660..D-8669, registered by the main session's A5 workflow in `state/decision-blocks.json` on 5 Nov 2026 for BASE-COV-API repair r3 (free_from was D-8660; no ref tip's `docs/` names D-8660..D-8669) |
| BASE-COV-API-AS2 | D-8662, D-8663 | the same block |
| BASE-COV-API-AS3 | D-8664, D-8665 | the same block |
| BASE-COV-API-ASA | D-8666, D-8667 | the same block |
| (a re-planned assembly, BASE-COV-API-ASB) | D-8668, D-8669 | the same block, kept free |

COV-API's own block is D-7340..D-7349: D-7340..D-7347 are used at START (`git grep -n -E '^#+ D-734[0-9]' 6ea46e59c074 -- docs`); D-7348 and D-7349 stay unused in this round. Only an entry heading (`#+ D-...`) counts as used: a sentence that mentions an id is not a clash (for example `docs/05-decisions.md:87545` at START, "D-7346 to D-7349 are unused"). An id outside your row, or one already present as an entry heading in the docs at your START, is a STOP: write `STATUS: BLOCKED QUESTION` with the clash.
