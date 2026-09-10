/**
 * WHAT A FINISHED SWEEP ACTUALLY REPORTED.
 *
 * # The green banner over an empty ledger
 *
 * `/backtest/run.json` answers with one `running` object, and `in_flight` says
 * whether the sweep is still going. `pollSweep` tested that field and NOTHING
 * else, so it had exactly two readings of a payload that carries three:
 *
 * ```js
 * if (run.in_flight) { ...poll again... }
 * sweep = { phase: 'done', run, why: '' };   // <-- everything else
 * ```
 *
 * `cli::range_all` refuses the WHOLE command when every one of the nine rungs
 * refused — an unknown feed word, a span the store holds no month of, a
 * backwards range — and returns that sentence in place of a table. A run that
 * ends that way is not in flight, exactly like a run that swept millions of
 * bars. So the page printed
 *
 * > **Sweep finished** — NIFTY on zerodha, and the ledger below has been re-read.
 *
 * in green, re-read a ledger that had gained no row, and told the operator
 * nothing about why. That is `CLAUDE.md` §4's failure wearing a success's
 * clothes, sitting on the one control this console owns.
 *
 * # The server's invariant is what makes this decidable
 *
 * `api::sweeprun::settle` files `cli`'s answer under `refusal` when it opens
 * with the word `refused` and under `report` otherwise, so **exactly one of the
 * two is set** on any ended run. Asking which one is asking what happened, and
 * no parsing of the text is needed on this side.
 *
 * Split into a module rather than left inline because a `.svelte` component is
 * not importable by `node --test`, and D-0129 already settled that the way to
 * prove a page's arithmetic in this repository is to move it here — SS-07
 * through SS-10 are the precedent.
 */

/**
 * THE `running` MEMBER, AND ALL FOURTEEN FIELDS IT CARRIES.
 *
 * This listed three — `in_flight`, `report`, `refusal` — because three were
 * what `sweepOutcome` read. `crates/api/src/sweeprun.rs` writes fourteen, and
 * the moment `/backtest` reached for a fourth (`run.feed` and
 * `run.underlying`, to ask the frontier route for a top-25) the checker called
 * them properties that do not exist. They exist; this file had never said so.
 *
 * ALL FOURTEEN ARE NAMED EVEN THOUGH THREE ARE READ, which is the rule
 * `/ingest`'s own `PilotFeed` states in as many words: a typedef that lists
 * only what today happens to be read is one that has to be edited before the
 * next field can be looked at — and editing it is the step that gets skipped,
 * leaving a true read reported as an error.
 *
 * ONLY `in_flight` IS REQUIRED. It is the one field every arm of
 * `sweepOutcome` depends on, and the one the tests construct payloads around;
 * the rest are optional so a build that has not yet grown a field is described
 * by this type rather than contradicted by it.
 *
 * @typedef {object} Running
 * @property {boolean} in_flight Whether the sweep is still going.
 * @property {string} [kind] Which sweep this was.
 * @property {string} [feed] The vendor the bars came from.
 * @property {string} [underlying] The instrument swept, as the ledger spells it.
 * @property {number} [from_year]
 * @property {number} [from_month]
 * @property {number} [to_year]
 * @property {number} [to_month]
 * @property {number | null} [support_ppm] The frequent floor, in parts per million.
 * @property {number} [started_micros] Microseconds, the unit `crates/api` writes.
 * @property {number} [attempt] Opaque exact token shared by status and structural events.
 * @property {string} [attempt_key] Exact u64 token; authoritative beyond the safe-number range.
 * @property {number | null} [finished_micros] `null` while the run is in flight.
 * @property {string | null} [report] The rung table, on a run that swept.
 * @property {string | null} [refusal] Why nothing was swept, on a run that did not.
 * @property {string} [status] Explicit external lifecycle status, including unknown.
 * @property {string} [why] Why external execution state cannot be established.
 */

/**
 * @typedef {object} SweepState
 * @property {'idle'|'starting'|'running'|'done'|'failed'|'unknown'} phase
 * @property {Running | null} run
 * @property {string} why
 */

/**
 * The state one `/backtest/run.json` payload puts the page in.
 *
 * Total over every payload the route can send, which is the point: the caller
 * has no fall-through branch left to guess in.
 *
 * - no run at all — the process has started none, so `idle`
 * - `in_flight` — `running`, and the caller schedules the next poll
 * - `refusal` set — `failed`, carrying the server's own sentence
 * - a nonempty completion report — `done`, and the ledger is worth re-reading
 * - missing or unreadable lifecycle evidence — `unknown`, never completed
 *
 * `in_flight` is tested BEFORE `refusal` on purpose. The two are independent
 * fields and a slot that carried a stale refusal into a fresh run would
 * otherwise report the new run as failed before it had done anything.
 *
 * @param {Running | null | undefined} run The `running` member of the payload.
 * @returns {SweepState}
 */
export function sweepOutcome(run) {
	if (!run) return { phase: 'idle', run: null, why: '' };
	if (run.status === 'unknown' || typeof run.in_flight !== 'boolean') {
		return { phase: 'unknown', run, why: run.why || 'The server cannot establish whether this execution is running or finished.' };
	}
	if (run.in_flight) return { phase: 'running', run, why: '' };
	if (run.refusal) return { phase: 'failed', run, why: run.refusal };
	if (typeof run.report === 'string' && run.report.length > 0) return { phase: 'done', run, why: '' };
	return { phase: 'unknown', run, why: run.why || 'Execution is no longer reported in flight, but no completion or refusal receipt was supplied.' };
}

/**
 * Whether a sweep started now could record anything, answered BEFORE it starts.
 *
 * # The hours this exists to save
 *
 * `cli::results::Results::append` refuses outright when the ledger FILE's
 * version is not the version the build writes — `CLAUDE.md` §3 rule 8, *"store
 * format versions are never mutated in place"*. The refusal is correct: a
 * 261-byte record appended to a file addressed every 213 bytes corrupts every
 * record after it, and it still parses.
 *
 * What was missing is that nothing said so in advance. Measured on the
 * operator's own store — `runs.bin` at version 2 against a build writing 3 —
 * pressing Run swept nine rungs over 121 months of one-minute bars, refused the
 * append on every one, turned each into a rung refusal, and refused the whole
 * command. Hours of CPU for a sentence that was knowable from sixteen header
 * bytes read at page load.
 *
 * # `appendable` is the SERVER's answer, not a comparison made here
 *
 * `version` was already on the wire and this page could have compared it to a
 * `3` of its own. That is the hand-kept second copy `CLAUDE.md` §5 refuses for
 * the condition vocabulary, and for the same reason: correct the day it is
 * written, silently wrong the first time the format moves. The rule lives in
 * `api::backtest` beside the constant it depends on.
 *
 * Only an explicit `false` blocks. An absent field is not evidence of a
 * problem, and painting a red banner over a payload that never made the claim
 * would be an alarm nobody can act on.
 *
 * A ledger the reader already REFUSED returns `null`: that has its own banner
 * on the page, and two banners for one fault read as two faults.
 *
 * @param {{version?: number, writes_version?: number, appendable?: boolean,
 *          refusal?: string | null, path?: string | null} | null | undefined} ledger
 * @returns {{version: number | null, writes: number | null, path: string | null} | null}
 *   The two versions and the file, or `null` when a sweep can record normally.
 */
export function ledgerBlock(ledger) {
	if (!ledger || ledger.appendable !== false || ledger.refusal) return null;
	return {
		version: ledger.version ?? null,
		writes: ledger.writes_version ?? null,
		path: ledger.path ?? null
	};
}
