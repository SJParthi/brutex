import test from 'node:test';
import assert from 'node:assert/strict';
import {
  checkExplanation, decisionExplanation, exitExplanation, formatPaisa,
  formatPpmPercent, formatPpmRatio, formatSavedCount, periodComparison,
  signalExplanation, timeframeExplanation, SETTING_EXPLANATION,
  RETURN_EXPLANATION, UNKNOWN_EXPLANATION, VWAP_EXPLANATION, resultReason
} from '../saved-backtest/explanations.js';

test('integer paisa stays exact at cent, negative and i64 boundaries', () => {
  assert.equal(formatPaisa('0'), '₹0.00');
  assert.equal(formatPaisa('1'), '₹0.01');
  assert.equal(formatPaisa('-1'), '−₹0.01');
  assert.equal(formatPaisa('-159340'), '−₹1,593.40');
  assert.equal(formatPaisa('9007199254740993'), '₹9,00,71,99,25,47,409.93');
  assert.equal(formatPaisa('-9223372036854775808'), '−₹92,23,37,20,36,85,47,758.08');
  assert.equal(formatPaisa('9223372036854775808'), 'Invalid saved value');
});

test('missing and malformed values never become zero or a blank valid result', () => {
  for (const format of [formatPaisa, formatSavedCount, formatPpmPercent, formatPpmRatio]) {
    for (const missing of [null, undefined]) assert.equal(format(missing), 'Not recorded');
    for (const invalid of ['', 'NaN', '1e3', '01', '-0', '1.5', 0, false, {}]) {
      assert.equal(format(invalid), 'Invalid saved value');
    }
  }
  assert.equal(formatSavedCount('0'), '0');
  assert.equal(formatSavedCount('18446744073709551615'), '1,84,46,74,40,73,70,95,51,615');
  assert.equal(formatSavedCount('-1'), 'Invalid saved value');
});

test('exit percentages and policy ratios use their own exact ppm scales', () => {
  assert.equal(formatPpmPercent('13244'), '1.3244%');
  assert.equal(formatPpmPercent('693'), '0.0693%');
  assert.equal(formatPpmPercent('50000'), '5%');
  assert.equal(formatPpmPercent('1'), '0.0001%');
  assert.equal(formatPpmRatio('3000000'), '3×');
  assert.equal(formatPpmRatio('1500001'), '1.500001×');
  assert.equal(formatPpmPercent('-13244'), '−1.3244%');
  assert.equal(formatPpmRatio('-1500001'), '−1.500001×');
});

test('rejected, refused, unmeasured and admitted retain their different meanings', () => {
  assert.match(decisionExplanation('rejected').detail, /evidence was measured/);
  assert.match(decisionExplanation('refused').detail, /refused upstream/);
  assert.match(decisionExplanation('unmeasured').detail, /not a zero result/);
  assert.match(decisionExplanation('admitted').detail, /not permission to trade/);
  assert.equal(decisionExplanation(undefined).label, 'Decision not recorded');
  assert.match(SETTING_EXPLANATION, /not its rank/);
  assert.match(RETURN_EXPLANATION, /one unit per trade, before costs/);
});

test('the signal timeframe is not confused with execution duration', () => {
  assert.equal(timeframeExplanation('60min'), '60-minute candles for the entry rule. Saved execution uses separate 1-minute bars.');
  assert.equal(timeframeExplanation('1day'), 'Signal timeframe not recorded.');
  assert.equal(timeframeExplanation(undefined), 'Signal timeframe not recorded.');
});

test('signal prose uses supplied Rust names rather than a copied numeric vocabulary', () => {
  const names = [{ i: 52, name: 'close_above_vwap' }, { i: 53, name: 'close_below_vwap' }];
  assert.equal(signalExplanation('52', names), 'Close above the session VWAP');
  assert.equal(signalExplanation('53', names), 'Close below the session VWAP');
  assert.equal(signalExplanation('!(53)', names), 'Close at or above the session VWAP, when VWAP is available');
  assert.equal(signalExplanation('!52', names), 'Close at or below the session VWAP, when VWAP is available');
  assert.match(signalExplanation('!(53)'), /name not loaded/);
  assert.equal(signalExplanation('!(999)', [{ i: 999, name: 'close_below_vwap' }]), 'Close at or above the session VWAP, when VWAP is available');
  assert.match(signalExplanation('(52 | 53)', names), /close above vwap OR close below vwap/);
  assert.match(UNKNOWN_EXPLANATION, /NOT keeps Unknown as Unknown/);
  assert.match(UNKNOWN_EXPLANATION, /Spot-index VWAP remains unavailable/);
  assert.match(VWAP_EXPLANATION, /two positive-volume bars/);
  assert.match(VWAP_EXPLANATION, /including the current bar/);
});

test('the App vocabulary projection and nested expressions keep every operator and missing name visible', () => {
  const names = [{ index: 52, name: 'close_above_vwap' }, { bit: 53, name: 'close_below_vwap' }];
  assert.equal(signalExplanation('52', names), 'Close above the session VWAP');
  assert.equal(signalExplanation('53', names), 'Close below the session VWAP');
  assert.equal(signalExplanation('(52 & !(53 | 999))', names), '(close above vwap AND NOT (close below vwap OR condition 999 (name not loaded)))');
  assert.equal(signalExplanation(null), 'Not recorded');
  assert.equal(signalExplanation(''), 'Not recorded');
});

test('pinned480 and481 exit shapes differ by trailing rules, not an invented ranking', () => {
  // Exact display projections observed on pinned batch1/1min, local coordinates144/145.
  const stop = { stop_ppm: '13244', target_ppm: null, tsl_ppm: null, ttp_arm_ppm: null, ttp_trail_ppm: null };
  const grid = { horizon_bars: '5', execution_seconds: '60' };
  const first = exitExplanation({ levels: { ...stop, tsl_ppm: '22214', ttp_arm_ppm: '15325', ttp_trail_ppm: '693' } }, grid);
  const next = exitExplanation({ levels: stop }, grid);
  assert.deepEqual(first.map(row => row.value), ['1.3244%', 'Off', '2.2214%', 'Activate at 1.5325%; trail by 0.0693%', '5 minutes']);
  assert.deepEqual(next.map(row => row.value), ['1.3244%', 'Off', 'Off', 'Off', '5 minutes']);
  assert.match(first[4].detail, /same-session 15:10 IST deadline/);
  assert.equal(exitExplanation({}, {})[0].value, 'Not recorded');
  assert.equal(exitExplanation({}, {})[4].value, 'Not recorded');
  assert.equal(exitExplanation({}, { horizon_bars: '3', execution_seconds: '15' })[4].value, '45 seconds');
});

test('training and later zero observations stay separate from an unloaded period', () => {
  const observed = periodComparison({ trades: '1395', wins: '278', return_paisa: '-159340' },
    { cell: { trades: '1947', wins: '392', pessimistic: '-186540', optimistic: '2234' }, truth: { hits: '11273', unknown: '62' } });
  assert.equal(observed.training.pessimistic, '−₹1,593.40');
  assert.equal(observed.later.pessimistic, '−₹1,865.40');
  assert.equal(observed.later.optimistic, '₹22.34');
  assert.equal(observed.later.trades, '1,947');
  assert.equal(periodComparison(null, undefined).later.trades, 'Not recorded');
  assert.equal(periodComparison(null, { cell: { trades: '0', pessimistic: '0' } }).later.trades, '0');
  assert.equal(periodComparison(null, { cell: { trades: '0', pessimistic: '0' } }).later.pessimistic, '₹0.00');
});

/** Generated complete display-policy shape; not a market or admission fixture.
 * @param {string} name @param {string|boolean} value */
function policy(name, value) {
  return { digest: 'a'.repeat(64), values: [{ name, value }, ...Array.from({ length: 38 }, (_, index) => ({ name: `fixture_${index}`, value: '1' }))] };
}

test('a measured zero and an unmeasured check never share a value label', () => {
  const zero = checkExplanation({ name: 'trades', state: 'failed' }, [{ name: 'trades', state: 'measured', value: '0' }]);
  const missing = checkExplanation({ name: 'trades', state: 'unmeasured' }, [{ name: 'trades', state: 'unmeasured', value: null }]);
  assert.equal(zero.observed, '0');
  assert.equal(missing.observed, 'Not measured');
  assert.equal(zero.required, 'Policy limit not loaded');
  assert.match(missing.detail, /do not fill this gap/);
  assert.equal(checkExplanation({ name: 'trades', state: 'passed' }, []).observed, 'Not recorded');
});

test('only a complete supplied policy displays thresholds; saved zero ceilings remain zero', () => {
  const check = { name: 'trades', state: 'failed' }, values = [{ name: 'trades', state: 'measured', value: '0' }];
  const complete = policy('min_trades', '200');
  assert.equal(checkExplanation(check, values, complete).required, 'At least 200');
  for (const absent of [null, {}, { values: complete.values }, { ...complete, values: complete.values.slice(1) },
    { ...complete, values: [...complete.values.slice(0, 38), complete.values[0]] }]) {
    assert.equal(checkExplanation(check, values, absent).required, 'Policy limit not loaded');
  }
  assert.equal(checkExplanation({ name: 'ambiguous_fills', state: 'failed' }, [], policy('max_ambiguous_fill_rate_ppm', '0')).required, 'At most 0%');
});

test('check explanations preserve probability, price, ratio and logical units', () => {
  assert.equal(checkExplanation({ name: 'max_mae', state: 'failed' }, [{ name: 'max_mae_paisa', state: 'measured', value: '501' }], policy('max_mae_paisa', '500')).required, 'At most ₹5.00');
  const ratio = checkExplanation({ name: 'worst_reward_risk', state: 'failed' }, [{ name: 'worst_reward_risk_ppm', state: 'measured', value: '1500000' }], policy('min_worst_reward_risk_ppm', '3000000'));
  assert.equal(ratio.observed, '1.5×'); assert.equal(ratio.required, 'At least 3×');
  const probability = checkExplanation({ name: 'fwer', state: 'failed' }, [{ name: 'fwer_p_value_ppm', state: 'measured', value: '1000000' }], policy('max_fwer_p_value_ppm', '50000'));
  assert.equal(probability.observed, '100%'); assert.equal(probability.required, 'At most 5%');
  assert.match(probability.detail, /not the probability that a trade will win/);
  assert.equal(checkExplanation({ name: 'white_reality_decision', state: 'failed' }, [], policy('require_white_reality_rejection', true)).required, 'Statistical null rejection required');
});

test('policy booleans never become numeric truthiness and large saved count limits remain exact', () => {
  const check = { name: 'white_reality_decision', state: 'passed' };
  assert.equal(checkExplanation(check, [], policy('require_white_reality_rejection', false)).required, 'Null rejection not required by this saved policy');
  assert.equal(checkExplanation(check, [], policy('require_white_reality_rejection', '1')).required, 'Invalid saved value');
  assert.equal(checkExplanation({ name: 'trades', state: 'failed' }, [], policy('min_trades', '18446744073709551615')).required, 'At least 1,84,46,74,40,73,70,95,51,615');
  assert.equal(checkExplanation(null).label, 'Unknown check');
});

test('execution refusals and missing fold authority do not become failed profit measurements', () => {
  const refused = checkExplanation({ name: 'execution_completeness', state: 'refused' }, [{ name: 'execution_complete', state: 'refused', value: null }]);
  assert.equal(refused.observed, 'Evidence refused');
  assert.match(refused.detail, /Read the saved execution refusal/);
  const later = checkExplanation({ name: 'oos_pessimistic_return', state: 'unmeasured' }, [{ name: 'oos_pessimistic_return_paisa', state: 'unmeasured', value: null }]);
  assert.equal(later.observed, 'Not measured');
  assert.equal(later.outcome, 'Not measured');
  assert.equal(checkExplanation({ name: 'future-check', state: 'future' }, []).outcome, 'Outcome not recorded');
});

test('the research reason names later execution refusals even when training is clean', () => {
  const comparison = { status: 'refused', checks: [{ name: 'trades', state: 'failed' }, { name: 'execution_completeness', state: 'refused' }] };
  assert.equal(resultReason(comparison, { training: { execution_refusals: [] }, later: { execution_refusals: ['Missing target'] } }), 'Later: Missing target');
  assert.equal(resultReason(comparison, { training: { execution_refusals: ['Missing stop'] }, later: { execution_refusals: [] } }), 'Training: Missing stop');
  assert.equal(resultReason(comparison, { training: { execution_refusals: ['Missing stop'] }, later: { execution_refusals: ['Missing target'] } }), 'Training: Missing stop. Later: Missing target');
});

test('identical period refusals share a label without conflating different lists', () => {
  const facts = { training: { execution_refusals: ['Missing target'] }, later: { execution_refusals: ['Missing target'] } };
  assert.equal(resultReason({ status: 'refused' }, facts), 'Training and later: Missing target');
  const both = ['Missing stop', 'Missing target'];
  assert.equal(resultReason({}, { training: { execution_refusals: both }, later: { execution_refusals: [...both].reverse() } }), 'Training and later: Missing stop; Missing target');
  assert.deepEqual(both, ['Missing stop', 'Missing target']);
});

test('missing period details prioritize refused then unmeasured checks before measured failures', () => {
  const checks = [{ name: 'trades', state: 'failed' }, { name: 'max_mae', state: 'failed' },
    { name: 'decided_folds', state: 'unmeasured' }, { name: 'execution_completeness', state: 'refused' }];
  const comparison = { status: 'refused', checks, values: [] };
  const expected = 'Evidence refused: Required execution and exit evidence; Not measured: Completed later validation windows; 2 more checks';
  assert.equal(resultReason(comparison, undefined), expected);
  assert.equal(resultReason(comparison, { training: { execution_refusals: [] }, later: { execution_refusals: [] } }), expected);
  assert.equal(checks[0].state, 'failed', 'sorting the explanation must not mutate the saved checks');
  assert.equal(resultReason({ ...comparison, checks: checks.slice(1) }, null), 'Evidence refused: Required execution and exit evidence; Not measured: Completed later validation windows; 1 more check');
});

test('ordinary measured failures, saved passes and missing decisions retain their meanings', () => {
  assert.equal(resultReason({ status: 'rejected', checks: [{ name: 'trades', state: 'failed' }] }, null), 'Failed measured limit: Number of trades');
  const admitted = { status: 'admitted', checks: [{ name: 'trades', state: 'passed' }] };
  assert.equal(resultReason(admitted, null), decisionExplanation('admitted').detail);
  assert.equal(resultReason(undefined, undefined), decisionExplanation(undefined).detail);
  assert.equal(resultReason({ status: 'unmeasured' }, { training: { execution_refusals: [''] } }), decisionExplanation('unmeasured').detail);
});
