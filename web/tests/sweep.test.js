// A SWEEP THAT REFUSED IS NOT A SWEEP THAT FINISHED.
//
// `pollSweep` tested `in_flight` and nothing else, so every ended run — the one
// that swept 1.4 million bars and the one that refused every rung before
// reading a byte — took the same branch and printed the same green
// "Sweep finished" over a ledger that, in the second case, had gained no row.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { sweepOutcome } from '../src/lib/sweep.js';

/** The sentence `cli::range_all` returns when nothing was read. */
const REFUSAL =
	'refused: every one of the 9 rungs refused. Nothing was read and no row was recorded.\n' +
	'  first reason: `nosuchfeed` is not a feed this engine reads.\n';

/** A report opens with the provenance banner, which is what makes it one. */
const REPORT =
	'THESE BARS ARE REAL MARKET DATA, READ FROM THE STORE\n' +
	'feed zerodha · NIFTY · ALL EIGHT INTRADAY RUNGS\n';

test('a run that refused every rung is failed, not finished', () => {
	// THE REPRODUCED CASE: an unknown feed word. Nine rungs refuse, `range_all`
	// refuses the whole command, `settle` files the sentence under `refusal`,
	// and `in_flight` is false because the run genuinely ended.
	const state = sweepOutcome({ in_flight: false, report: null, refusal: REFUSAL });

	assert.equal(state.phase, 'failed', 'this branch printed the green banner');
	assert.equal(state.why, REFUSAL, 'the server already said why; the page must not invent one');
	// AND THE RUN TRAVELS WITH IT. The page renders a server refusal in the
	// block that keeps newlines and a client-side one inline, and it tells the
	// two apart by whether there is a run attached.
	assert.ok(state.run, 'a server refusal carries its run');
});

test('a run that swept is finished and carries its table', () => {
	const state = sweepOutcome({ in_flight: false, report: REPORT, refusal: null });

	assert.equal(state.phase, 'done');
	assert.equal(state.why, '');
	assert.equal(state.run?.report, REPORT, 'the nine-rung table is what the page draws');
});

test('a run still going is running, whatever else the slot holds', () => {
	// IN FLIGHT IS TESTED FIRST, AND THIS IS THE CASE THAT NEEDS IT. The slot
	// keeps the previous run until the next one replaces it, so a payload can
	// carry a fresh `in_flight: true` beside a stale refusal. Reading the
	// refusal first would report a run as failed before it had done anything.
	const state = sweepOutcome({ in_flight: true, report: null, refusal: REFUSAL });

	assert.equal(state.phase, 'running');
	assert.equal(state.why, '');
});

test('no run at all is idle rather than an absence the caller must decide', () => {
	for (const nothing of [null, undefined]) {
		const state = sweepOutcome(nothing);
		assert.equal(state.phase, 'idle');
		assert.equal(state.run, null);
		assert.equal(state.why, '');
	}
});

test('every payload the route can send lands in exactly one phase', () => {
	// THE POINT OF THE FUNCTION IS THAT THE CALLER HAS NO FALL-THROUGH LEFT.
	// An ended run with NEITHER field set cannot come off `settle`, which always
	// sets one — but the page must not silently treat it as a success if the
	// server ever regressed, so the case is pinned rather than left open.
	const payloads = [
		null,
		{ in_flight: true },
		{ in_flight: false, report: REPORT, refusal: null },
		{ in_flight: false, report: null, refusal: REFUSAL },
		{ in_flight: false, report: null, refusal: null }
	];
	const phases = payloads.map((p) => sweepOutcome(p).phase);

	assert.deepEqual(phases, ['idle', 'running', 'done', 'failed', 'done']);
	for (const phase of phases) {
		assert.ok(
			['idle', 'running', 'done', 'failed'].includes(phase),
			`${phase} is not a state this page renders`
		);
	}
});

test('an empty refusal string is not a refusal', () => {
	// `json_string("")` is a legal payload and an empty sentence tells the
	// operator nothing. Treating it as a failure would paint a red block with
	// no reason in it, which is worse than the green one it replaced.
	const state = sweepOutcome({ in_flight: false, report: REPORT, refusal: '' });
	assert.equal(state.phase, 'done');
});
