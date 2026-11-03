# CLOUD-SIZE.md: the kit of session FNO-SIZE (3 Nov 2026; read before CLOUD.md)

This branch, `cloud/fib-kit-size-20261103`, is the first kit branch `cloud/fib-kit-20261103b`
(`fa9022bac962620528735ab97a61d6ab6d3dedbb` = FNO-SEAM's passed tip `1ad6e24ef8e0` plus this folder's first twelve files) plus
three files: this one; `PARTS-FNO-SIZE.md`, your part's design, which binds whole; and `MANIFEST-SIZE.md`, the sha256 of the two
others (check them with `sha256sum` before you rely on them).

Your base is `1ad6e24ef8e0`, as `CLOUD.md` item 4 says: measure the D2 touched set and the coverage against it. Everything in
`CLOUD.md` and `RULES-MAC.md` stands, with these differences for this part.

1. **No slot: seven anchors.** FNO-SEAM's slot lines stop at FNO-WFT in the runner files and at FNO-BAND-FILES in the four
   ledgers, so FNO-SIZE has none. `PARTS-FNO-SIZE.md` section 2 names the exact base line after which each insertion goes (one in
   `crates/runner/src/lib.rs`, two in `crates/runner/benches/ratio.rs`, one in each of `docs/04`, `05`, `06`, `07`). Where
   `RULES-MAC.md` says "directly under its slot", read "directly after its anchor line". Edit no other line of those files.
2. **Your section.** FNO-SIZE is in no section of this folder's `PARTS.md` and in no row of `parts.json.md`: `PARTS-FNO-SIZE.md`
   is its section. Its section 3 is the API, section 4 the rules, section 7 the test names, section 8 the benches, section 9 the
   ledger rows. The kit's `PARTS.md` sections it cites (4.1-4.2, 5 "Common to every part", FNO-PORTFOLIO, FNO-NET, FNO-WFT,
   FNO-RUN, READINESS ADDENDUM item 5) and `SPEC.md` 9.6, 11, 13, 14 and 18 are your facts.
   **Two texts it cites are not in this kit, on purpose:** `AMENDMENTS.md` (its R1-R8, T1-T3, P1, W1 and D1) and `SPEC.md`
   section 20 (OP-47 to OP-59). They are the main session's addenda for FNO-RUN, FNO-WFT, FNO-PORTFOLIO, FNO-SEC-WIRE and
   FNO-RECORDS and for the SPEC, appended on the Mac later; they bind those parts, not your code. Your code needs neither:
   wherever `PARTS-FNO-SIZE.md` cites them, its sections 3-9 state the rule your module must keep (the margin limit
   `min(capital, margin_cap)` of 4.3 step 3, for example), and what they ask of other parts (FNO-RUN's queue, OP-56 and OP-59;
   FNO-PORTFOLIO's slot cap) is outside your scope. Do not block on them and do not guess their text: list both under the
   report heading "Evidence the rules cite that this folder lacks", citing this item as the reason.
3. **Decision ids D-8630..D-8639 only.** Code under `crates/` may cite only a D-number that heads an entry of
   `docs/05-decisions.md` on the same tree (`crates/core/tests/decision_citations.rs`). FNO-PORTFOLIO's D-8180..D-8183 and
   FNO-NET's D-8230.. head nothing on your base: never cite them; name the part and the file in words.
4. **Other parts' code is not in your base, and must not be.** FNO-PORTFOLIO, FNO-NET, FNO-LEVERAGE and FNO-WFT are not on
   `1ad6e24ef8e0`. Take every figure they make as a plain integer, exactly as `PARTS-FNO-SIZE.md` section 3 lists; FNO-RUN maps
   their types later. The facts you need from their code are these (both files are public on origin, branches
   `cloud/fib-fno-portfolio-20261103` at `e6a722af1c2e` and `cloud/fib-fno-leverage-20261103` at `7904a8fe813b`):
   - **Tie-break (e)'s seeded key**, `crates/runner/src/fno_portfolio.rs` lines 1413-1431 of that branch, word for word:
     ```rust
     fn seeded(seed: u64, key: &Key) -> u64 {
         let mut state = mix(seed);
         state = mix(state ^ key.start.cast_unsigned());
         state = mix(state ^ u64::from(key.stock.0));
         mix(state ^ u64::from(key.recipe))
     }

     const fn mix(state: u64) -> u64 {
         let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
         z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
         z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
         z ^ (z >> 31)
     }
     ```
     (`key.start` is the order's start second `i64`, `key.stock.0` the stock's `u8` position, `key.recipe` a `u32`.) RANKED's
     fourth key must give the same `u64` for the same (seed, start, stock, recipe), smaller first: then one seed draws one
     permutation in every policy. Write your test vectors from this formula; FNO-RUN's check compares it with the real function.
   - **The leverage**, `crates/pull/src/nse_margin.rs` lines 1189-1202: `L = 1,000,000 / max(200,000, M)` in parts per million,
     reduced. So the margin rate is `max(200,000, M)` ppm, `L = 10^6 / rate`, and a stock whose M is above 100% has L below 1x.
     Your `rate_ppm` input is that rate, the largest M of the readings (the smallest leverage).
   - **FNO-PORTFOLIO's refusal words** are `position_cap`, `trade_cap`, `exposure_cap`, `margin_cap`, `while_open`
     (`fno_portfolio.rs` lines 341-345). Yours are section 3's, and only `while_open` is shared, with the same meaning.
5. **No `.github`, `ci.yml`, `Cargo.toml` or `Cargo.lock` edit, and no new dependency.** Gate 11 (`.github/gates_ledger.rs`,
   rules 1-7) refuses, on a production line outside a `#[cfg(test)]` module: `.sort(` or `.sort_*(`, `select_nth_unstable`,
   `BinaryHeap`, `collect::<BTreeSet` or `collect::<BTreeMap`, `.binary_search`, `.partition_point`, `.iter().find(`,
   `.iter().position(`, `.contains(&`, `f32` or `f64`, `HashMap::new()` or `HashSet::new()`, a panic, a local lint allow or
   expect, a runtime assert. A file earns an exception only by a line in that `.github` file, which this part never edits. So
   RANKED's order is the fixed bitonic network of `PARTS-FNO-SIZE.md` 4.2, never a sort; use `get` and named refusals, never an
   index that can panic. Test modules may sort (the reference order of the network's property test).
6. **The ledger forms** (as the other fib parts wrote them under their slots):
   - `docs/04-invariants.md`: a table `| Id | Invariant | Proof | |` and `|---|---|---|---|`, one row per invariant
     (`FNO-SIZE-I01`..) and per bench id (`C-FNO-SIZE-01`..`07`), each naming its tests by function and file, last cell `✓`.
   - `docs/05-decisions.md`: `### D-8630 — FNO-SIZE: <title> — 2026-11-03`, then paragraphs **Context**, **Decision**, **Why**.
   - `docs/06-limits.md`: a table `| ID | Limit | Why it is not O(1) / not closed | Bound | Package |`, rows `L-FNO-SIZE-01`..
   - `docs/07-plan.md`: a table `| FNO-SIZE (wave 3, F2) | State |` with what was built and what is next.
   - The bench ids are printed by your row fn and written in the row call in `crates/runner/benches/ratio.rs`, because
     `crates/core/tests/cost_invariants.rs` reads them only from the top-level bench file and pairs each with a `docs/04` row
     (the seam's comment at `crates/runner/benches/ratio.rs` lines 590-601).
7. **Your report** (`report-FNO-SIZE.md`) also lists: each anchor used, with the base line you inserted after; the `docs/04` bench
   rows that gate 14's cover table in `ci.yml` needs (the main session adds them at F2's assembly, never you); every test name of
   `PARTS-FNO-SIZE.md` section 7 with its result, and any you added; the Route B figure for `crates/runner/src/fno_size/` or
   UNMEASURED with the reason; NEEDS THE MAC (`PARTS-FNO-SIZE.md` section 11).

The four policies, the account limits for every policy (margin in use at most Rs 20,00,000, or the capital when carried losses
leave less, and open exposure at most Rs 1,00,00,000, checked at every entry on marked positions as `PARTS-FNO-SIZE.md` section 0
item 3 says; each stock's leverage as above; everything net of every charge at worst-print fills) and the rule that the Winner
Gate's numbers never change are all in `PARTS-FNO-SIZE.md`; nothing in this kit changes them. Its section 12 holds one question
for the operator (whether a mark past a limit forces a cut); it changes no code of yours, so build as the section says.
