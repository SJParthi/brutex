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
 * @typedef {object} Running
 * @property {boolean} in_flight Whether the sweep is still going.
 * @property {string | null} [report] The nine-rung table, on a run that swept.
 * @property {string | null} [refusal] Why nothing was swept, on a run that did not.
 */

/**
 * @typedef {object} SweepState
 * @property {'idle'|'starting'|'running'|'done'|'failed'} phase
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
 * - otherwise — `done`, and the ledger is worth re-reading
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
	if (run.in_flight) return { phase: 'running', run, why: '' };
	if (run.refusal) return { phase: 'failed', run, why: run.refusal };
	return { phase: 'done', run, why: '' };
}
