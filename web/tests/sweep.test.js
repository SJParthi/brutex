// A SWEEP THAT REFUSED IS NOT A SWEEP THAT FINISHED.
//
// `pollSweep` tested `in_flight` and nothing else, so every ended run — the one
// that swept 1.4 million bars and the one that refused every rung before
// reading a byte — took the same branch and printed the same green
// "Sweep finished" over a ledger that, in the second case, had gained no row.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { sweepOutcome, ledgerBlock } from '../src/lib/sweep.js';

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

	assert.deepEqual(phases, ['idle', 'running', 'done', 'failed', 'unknown']);
	for (const phase of phases) {
		assert.ok(
			['idle', 'running', 'done', 'failed', 'unknown'].includes(phase),
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

test('missing, stale and unreadable lifecycle evidence is unknown, never a completed sweep', () => {
  for (const run of [
    { in_flight: false, status: 'unknown', why: 'log cannot be read' },
    { in_flight: false, report: null, refusal: null },
    { in_flight: /** @type {any} */ ('false'), report: REPORT },
    { in_flight: false, status: 'unknown', report: REPORT, why: 'stale activity' }
  ]) {
    const outcome = sweepOutcome(run);
    assert.equal(outcome.phase, 'unknown');
    assert.ok(outcome.why.length > 0);
  }
});

// ---- the pre-flight check on the ledger ------------------------------------

test('a ledger the build cannot append to blocks the run, and names both versions', () => {
	// THE REPRODUCED CASE, from the operator's own store: runs.bin at version 2
	// against a build writing 3. Nine rungs swept 121 months and recorded
	// nothing, because `Results::append` refuses on the mismatch.
	const block = ledgerBlock({
		version: 2,
		writes_version: 3,
		appendable: false,
		refusal: null,
		path: '/Users/x/.brutex/store/results/runs.bin'
	});

	assert.ok(block, 'this is the whole point: it must be knowable before Run');
	assert.equal(block.version, 2);
	assert.equal(block.writes, 3, 'the banner names both numbers, not just the mismatch');
	// PINNED SEPARATELY BECAUSE THE MATCH CANNOT PIN IT. `ledgerBlock` returns
	// `path: string | null` — the ledger can block without naming a file — and
	// `assert.match` on a null path would fail with a type complaint about its
	// argument rather than with the sentence this test is about. Asserting the
	// path is THERE before asserting what it ends with states the two facts
	// separately, and says which one broke.
	assert.ok(block.path, 'a block that cannot name its file cannot be acted on');
	assert.match(block.path, /runs\.bin$/, 'the operator has to know which file to move');
});

test('a current ledger does not block', () => {
	assert.equal(ledgerBlock({ version: 3, writes_version: 3, appendable: true }), null);
});

test('an absent appendable field makes no claim in either direction', () => {
	// A payload that never said whether it could be appended to is not evidence
	// that it cannot. Painting a red banner over it would be an alarm the
	// operator has no way to act on, and no way to clear.
	assert.equal(ledgerBlock({ version: 3 }), null);
	assert.equal(ledgerBlock({}), null);
	assert.equal(ledgerBlock(null), null);
	assert.equal(ledgerBlock(undefined), null);
});

test('a ledger that was already refused gets one banner, not two', () => {
	// `api::backtest` reports `appendable:false` for an unreadable ledger too,
	// which is correct -- but the page already renders `refusal` on its own.
	// Two banners for one fault read as two faults.
	const block = ledgerBlock({
		version: 0,
		writes_version: 3,
		appendable: false,
		refusal: 'the header is shorter than its magic'
	});
	assert.equal(block, null);
});

test('a blocked ledger still reports what it can when fields are missing', () => {
	// The banner must render rather than throw when the server sent the
	// decision but not the detail -- `null` is a state the page can word
	// around, `undefined.toString()` is a blank screen.
	const block = ledgerBlock({ appendable: false });
	assert.ok(block);
	assert.equal(block.version, null);
	assert.equal(block.writes, null);
	assert.equal(block.path, null);
});
