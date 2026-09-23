/**
 * Pure presentation folds for the recorded trade stream.
 *
 * The Rust side owns every stored number.  This module only turns its integer
 * period keys and ordered worst-fill results into labels and bounded chart
 * rows.  Keeping the folds outside the Svelte component makes the edge cases
 * executable without a browser.
 */

export const TIME_GRAINS = Object.freeze([
  Object.freeze({ key: 'hour', label: 'Hours' }),
  Object.freeze({ key: 'weekday', label: 'Weekdays' }),
  Object.freeze({ key: 'day', label: 'Daily' }),
  Object.freeze({ key: 'week', label: 'Weekly' }),
  Object.freeze({ key: 'month', label: 'Monthly' }),
  Object.freeze({ key: 'quarter', label: 'Quarterly' }),
  Object.freeze({ key: 'half', label: 'Half-years' }),
  Object.freeze({ key: 'year', label: 'Years' })
]);

const WEEKDAY_NAMES = Object.freeze(['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']);
const MONTH_NAMES = Object.freeze([
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec'
]);

const TRADE_INTEGER_FIELDS = Object.freeze([
  'seq',
  'signal_bar',
  'entry_bar',
  'exit_bar',
  'best',
  'worst',
  'bars_held',
  'entry_micros',
  'exit_micros',
  'adverse_ppm',
  'adverse_paisa',
  'favourable_ppm',
  'favourable_paisa'
]);
const BUCKET_INTEGER_FIELDS = Object.freeze([
  'key',
  'trades',
  'wins',
  'worst_wins',
  'best_paisa',
  'worst_paisa',
  'largest_win',
  'largest_loss'
]);

/** @param {number} left @param {number} right */
const integerOrder = (left, right) => (left === right ? 0 : left < right ? -1 : 1);

/**
 * Add two already-validated integers without accepting a rounded result.
 *
 * @param {number} left
 * @param {number} right
 * @returns {number | null}
 */
const checkedAdd = (left, right) => {
  const sum = left + right;
  return Number.isSafeInteger(sum) ? sum : null;
};

/**
 * Subtract two already-validated integers without accepting a rounded result.
 *
 * @param {number} left
 * @param {number} right
 * @returns {number | null}
 */
const checkedSubtract = (left, right) => {
  const difference = left - right;
  return Number.isSafeInteger(difference) ? difference : null;
};

/**
 * Maximum drawdown of a cumulative realised-P&L curve.
 *
 * Equity starts at zero before the first trade. Seeding the running peak with
 * the first point erases an initial loss: `[-100, -50]` would incorrectly have
 * no drawdown even though capital first fell by 100 paisa.
 *
 * @param {unknown} input
 * @returns {number | null}
 */
export function equityMaxDrawdown(input) {
  if (!Array.isArray(input) || input.length === 0) return null;
  let peak = 0;
  let maximum = 0;
  for (const value of input) {
    if (!Number.isSafeInteger(value)) return null;
    if (value > peak) peak = value;
    const drawdown = checkedSubtract(peak, value);
    if (drawdown === null) return null;
    maximum = Math.max(maximum, drawdown);
  }
  return maximum;
}

/** @param {number} left @param {number} right @returns {number | null} */
const checkedMultiply = (left, right) => {
  const product = left * right;
  return Number.isSafeInteger(product) ? product : null;
};

const DAY_MICROS = 86_400_000_000;
const IST_OFFSET_MICROS = 19_800_000_000;

/** @param {number} micros @param {string} grain @returns {number|null} */
const periodKeyOf = (micros, grain) => {
  // Epoch microseconds are the timezone-neutral wire representation. Every
  // trading calendar bucket is nevertheless an exchange-calendar bucket, so
  // shift once to IST before deriving the civil day, week or hour.
  const istMicros = checkedAdd(micros, IST_OFFSET_MICROS);
  if (istMicros === null) return null;
  const days = Math.floor(istMicros / DAY_MICROS);
  const millis = checkedMultiply(days, 86_400_000);
  if (millis === null) return null;
  const date = new Date(millis);
  if (Number.isNaN(date.getTime())) return null;
  const year = date.getUTCFullYear();
  const month = date.getUTCMonth() + 1;
  if (grain === 'day') return days;
  if (grain === 'week') {
    const mondayShifted = checkedAdd(days, 3);
    return mondayShifted === null ? null : Math.floor(mondayShifted / 7);
  }
  if (grain === 'month') return year * 12 + month - 1;
  if (grain === 'quarter') return year * 4 + Math.floor((month - 1) / 3);
  if (grain === 'half') return year * 2 + Math.floor((month - 1) / 6);
  if (grain === 'year') return year;
  if (grain === 'weekday') return ((days + 3) % 7 + 7) % 7;
  if (grain === 'hour') {
    const insideDay = istMicros - days * DAY_MICROS;
    return Math.floor(insideDay / 3_600_000_000);
  }
  return null;
};

/** @param {Record<string, number>} bucket @param {any} row @returns {boolean} */
const takePeriodRow = (bucket, row) => {
  const trades = checkedAdd(bucket.trades, 1);
  const wins = checkedAdd(bucket.wins, row.best > 0 ? 1 : 0);
  const worstWins = checkedAdd(bucket.worst_wins, row.worst > 0 ? 1 : 0);
  const best = checkedAdd(bucket.best_paisa, row.best);
  const worst = checkedAdd(bucket.worst_paisa, row.worst);
  if (trades === null || wins === null || worstWins === null || best === null || worst === null) {
    return false;
  }
  bucket.trades = trades;
  bucket.wins = wins;
  bucket.worst_wins = worstWins;
  bucket.best_paisa = best;
  bucket.worst_paisa = worst;
  if (trades === 1) {
    bucket.largest_win = row.worst;
    bucket.largest_loss = row.worst;
  } else {
    bucket.largest_win = Math.max(bucket.largest_win, row.worst);
    bucket.largest_loss = Math.min(bucket.largest_loss, row.worst);
  }
  return true;
};

/** @param {string} why */
const invalidPayload = (why) => ({
  ok: false,
  rows: [],
  periods: null,
  policy: null,
  direction: null,
  empty: false,
  why
});

/**
 * Validate the complete `/trades.json` answer before one number reaches a fold.
 *
 * JSON turns Rust `u64`/`i64` values into JavaScript `Number`s.  A finite
 * number is not enough: integers beyond 2^53 are rounded before any chart sees
 * them, and two individually exact endpoints can still produce an inexact sum
 * or difference.  This door checks every integer field and every additive
 * intermediate used by the equity, streak and time-period folds.  One failure
 * refuses the whole payload; it never leaves a plausible-looking prefix.
 *
 * A successful empty trade file is kept distinct from malformed input. Every
 * successful response carries all eight period arrays. A store with no
 * committed run may carry null policy/direction beside an explicit refusal;
 * every committed result, including a zero-row one, must carry the canonical
 * chosen-grid policy and its selected direction.
 *
 * @param {unknown} input
 * @param {any} expectedRun
 * @returns {{ok: boolean, rows: any[], periods: any, policy: string|null, direction: string|null, empty: boolean, why: string}}
 */
export function validateTradePayload(input, expectedRun = undefined) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return invalidPayload('/trades.json did not return an object.');
  }
  const body = /** @type {Record<string, any>} */ (input);
  if (typeof body.identity !== 'string' || !/^[0-9a-f]{64}$/.test(body.identity)) {
    return invalidPayload('/trades.json did not carry one canonical run identity.');
  }
  if (expectedRun !== undefined && body.identity !== expectedRun?.identity) {
    return invalidPayload(
      `/trades.json answered for run ${body.identity}, not requested run ${String(expectedRun?.identity)}.`
    );
  }
  if (!Array.isArray(body.trades)) {
    return invalidPayload('/trades.json `trades` is not an array.');
  }
  if (!Number.isSafeInteger(body.count) || body.count < 0) {
    return invalidPayload('/trades.json `count` is not an exactly representable non-negative integer.');
  }
  if (body.count !== body.trades.length) {
    return invalidPayload(
      `/trades.json count says ${String(body.count)} but the response carries ${body.trades.length} trade rows.`
    );
  }

  const committedMetadata =
    body.policy === 'chosen-grid-v1' && (body.direction === 'long' || body.direction === 'short');
  const honestAbsence =
    body.policy === null &&
    body.direction === null &&
    body.trades.length === 0 &&
    typeof body.refusal === 'string' &&
    body.refusal.length > 0;
  if (!committedMetadata && !honestAbsence) {
    return invalidPayload(
      '/trades.json must carry policy `chosen-grid-v1` and direction `long` or `short`; only an explicitly refused uncommitted empty store may carry both as null.'
    );
  }
  if (committedMetadata && body.refusal !== null) {
    return invalidPayload('/trades.json is partial or refused; no trade prefix is published.');
  }

  const rows = body.trades;
  for (let at = 0; at < rows.length; at += 1) {
    const row = rows[at];
    if (!row || typeof row !== 'object' || Array.isArray(row)) {
      return invalidPayload(`/trades.json trades[${at}] is not an object.`);
    }
    if (row.direction !== body.direction) {
      return invalidPayload(
        `/trades.json trades[${at}].direction does not equal the committed selected direction.`
      );
    }
    for (const field of TRADE_INTEGER_FIELDS) {
      if (!Number.isSafeInteger(row[field])) {
        return invalidPayload(
          `/trades.json trades[${at}].${field} is not an exactly representable integer.`
        );
      }
    }
    for (const field of [
      'seq',
      'signal_bar',
      'entry_bar',
      'exit_bar',
      'bars_held',
      'adverse_ppm',
      'adverse_paisa',
      'favourable_ppm',
      'favourable_paisa'
    ]) {
      if (row[field] < 0) {
        return invalidPayload(`/trades.json trades[${at}].${field} cannot be negative.`);
      }
    }
    if (row.seq !== at) {
      return invalidPayload(
        `/trades.json trade sequence ${String(row.seq)} is not the canonical wire sequence ${String(at)}.`
      );
    }
    const sourceDistance = checkedSubtract(row.entry_bar, row.signal_bar);
    if ((sourceDistance !== 0 && sourceDistance !== 1) || row.entry_bar > row.exit_bar) {
      return invalidPayload(
        `/trades.json trades[${at}] is not a fill-sourced signal or an exact next-bar native signal in signal <= entry <= exit order.`
      );
    }
    const held = checkedSubtract(row.exit_bar, row.entry_bar);
    if (held === null || held !== row.bars_held) {
      return invalidPayload(
        `/trades.json trades[${at}].bars_held does not exactly equal exit_bar - entry_bar.`
      );
    }
    if (row.entry_micros > row.exit_micros) {
      return invalidPayload(`/trades.json trades[${at}] exits before it enters.`);
    }
    if (at > 0) {
      const previous = rows[at - 1];
      if (row.entry_bar <= previous.exit_bar || row.entry_micros <= previous.exit_micros) {
        return invalidPayload(
          `/trades.json trades[${at}] enters before the prior one-position-at-a-time trade has finished.`
        );
      }
    }
    if (row.worst > row.best) {
      return invalidPayload(
        `/trades.json trades[${at}] has a worst-fill result greater than its best-fill result.`
      );
    }
  }

  // Rehearse every integer accumulation the report performs.  The report may
  // divide these exact totals later, but it may never first round the integer
  // they came from.
  const ordered = rows;
  let cumulative = 0;
  let bestNet = 0;
  let worstTrade = 0;
  let maxDrawdown = 0;
  let grossProfit = 0;
  let grossLoss = 0;
  let bars = 0;
  let barsWin = 0;
  let barsLoss = 0;
  let streakAmount = 0;
  /** @type {'win'|'loss'|null} */ let streakKind = null;
  /** @type {number[]} */ const curve = [];
  for (const row of ordered) {
    cumulative = checkedAdd(cumulative, row.worst) ?? Number.NaN;
    bestNet = checkedAdd(bestNet, row.best) ?? Number.NaN;
    worstTrade = Math.min(worstTrade, row.worst);
    bars = checkedAdd(bars, row.bars_held) ?? Number.NaN;
    if (row.worst > 0) {
      grossProfit = checkedAdd(grossProfit, row.worst) ?? Number.NaN;
      barsWin = checkedAdd(barsWin, row.bars_held) ?? Number.NaN;
    } else if (row.worst < 0) {
      grossLoss = checkedAdd(grossLoss, -row.worst) ?? Number.NaN;
      barsLoss = checkedAdd(barsLoss, row.bars_held) ?? Number.NaN;
    }
    const nextKind = row.worst > 0 ? 'win' : 'loss';
    if (nextKind !== streakKind) {
      streakKind = nextKind;
      streakAmount = 0;
    }
    streakAmount = checkedAdd(streakAmount, row.worst) ?? Number.NaN;
    if (
      !Number.isSafeInteger(cumulative) ||
      !Number.isSafeInteger(bestNet) ||
      !Number.isSafeInteger(grossProfit) ||
      !Number.isSafeInteger(grossLoss) ||
      !Number.isSafeInteger(bars) ||
      !Number.isSafeInteger(barsWin) ||
      !Number.isSafeInteger(barsLoss) ||
      !Number.isSafeInteger(streakAmount)
    ) {
      return invalidPayload(
        '/trades.json contains values whose equity, streak, duration, or gross-result accumulation is not exactly representable.'
      );
    }
    curve.push(cumulative);
  }
  if (checkedSubtract(grossProfit, grossLoss) === null) {
    return invalidPayload('/trades.json gross profit minus gross loss is not exactly representable.');
  }

  if (curve.length > 0) {
    let peak = 0;
    let anchor = 0;
    let direction = 0;
    for (let at = 0; at < curve.length; at += 1) {
      const value = curve[at];
      if (value > peak) peak = value;
      const drawdown = checkedSubtract(peak, value);
      if (drawdown === null) {
        return invalidPayload('/trades.json equity drawdown is outside the browser exact-integer range.');
      }
      maxDrawdown = Math.max(maxDrawdown, drawdown);
      const previous = at === 0 ? 0 : curve[at - 1];
      const up = value > previous;
      const down = value < previous;
      if (direction === 0) {
        direction = up ? 1 : down ? -1 : 0;
      } else if ((direction === 1 && down) || (direction === -1 && up)) {
        if (checkedSubtract(previous, anchor) === null) {
          return invalidPayload('/trades.json equity swing is outside the browser exact-integer range.');
        }
        anchor = previous;
        direction = up ? 1 : -1;
      }
    }
    if (checkedSubtract(curve[curve.length - 1], anchor) === null) {
      return invalidPayload('/trades.json open equity swing is outside the browser exact-integer range.');
    }
  }

  if (expectedRun !== undefined) {
    for (const field of ['trades', 'pessimistic', 'optimistic', 'worst_trade', 'max_drawdown']) {
      if (!Number.isSafeInteger(expectedRun?.[field])) {
        return invalidPayload(`/backtest.json run.${field} is not an exactly representable integer.`);
      }
    }
    if (
      expectedRun.trades !== rows.length ||
      expectedRun.pessimistic !== cumulative ||
      expectedRun.optimistic !== bestNet ||
      expectedRun.worst_trade !== worstTrade ||
      expectedRun.max_drawdown !== maxDrawdown
    ) {
      return invalidPayload(
        '/trades.json chosen rows do not reconcile exactly with the committed run count, both totals, worst trade, and drawdown.'
      );
    }
  }

  if (body.periods === undefined || body.periods === null) {
    return invalidPayload('/trades.json omitted the eight period bucket arrays.');
  }
  if (typeof body.periods !== 'object' || Array.isArray(body.periods)) {
    return invalidPayload('/trades.json `periods` is not an object.');
  }
  const periodNames = Object.keys(body.periods).sort();
  const expectedNames = TIME_GRAINS.map(({ key }) => key).sort();
  if (
    periodNames.length !== expectedNames.length ||
    periodNames.some((name, index) => name !== expectedNames[index])
  ) {
    return invalidPayload('/trades.json `periods` does not contain exactly the eight versioned grains.');
  }

  /** @type {Record<string, Map<number, Record<string, number>>>} */
  const expectedPeriods = Object.fromEntries(
    TIME_GRAINS.map(({ key }) => [key, new Map()])
  );
  for (const row of rows) {
    if (row.entry_micros === 0) continue;
    for (const { key: grain } of TIME_GRAINS) {
      const key = periodKeyOf(row.entry_micros, grain);
      if (key === null || !Number.isSafeInteger(key)) {
        return invalidPayload(
          `/trades.json trades[${String(row.seq)}].entry_micros cannot be bucketed exactly for ${grain}.`
        );
      }
      const expected = expectedPeriods[grain];
      const held = expected.get(key) ?? {
        key,
        trades: 0,
        wins: 0,
        worst_wins: 0,
        best_paisa: 0,
        worst_paisa: 0,
        largest_win: 0,
        largest_loss: 0
      };
      if (!takePeriodRow(held, row)) {
        return invalidPayload(
          `/trades.json rows cannot be folded exactly into periods.${grain} key ${String(key)}.`
        );
      }
      expected.set(key, held);
    }
  }
  for (const { key: grain } of TIME_GRAINS) {
    const buckets = body.periods[grain];
    if (!Array.isArray(buckets)) {
      return invalidPayload(`/trades.json periods.${grain} is missing or is not an array.`);
    }
    const expected = [...expectedPeriods[grain].values()].sort((left, right) =>
      integerOrder(left.key, right.key)
    );
    if (buckets.length !== expected.length) {
      return invalidPayload(
        `/trades.json periods.${grain} carries ${String(buckets.length)} buckets, not the ${String(expected.length)} implied by the trade timestamps.`
      );
    }
    for (let at = 0; at < buckets.length; at += 1) {
      const bucket = buckets[at];
      if (!bucket || typeof bucket !== 'object' || Array.isArray(bucket)) {
        return invalidPayload(`/trades.json periods.${grain}[${at}] is not an object.`);
      }
      for (const field of BUCKET_INTEGER_FIELDS) {
        if (!Number.isSafeInteger(bucket[field])) {
          return invalidPayload(
            `/trades.json periods.${grain}[${at}].${field} is not an exactly representable integer.`
          );
        }
      }
      if (bucket.trades <= 0 || bucket.wins < 0 || bucket.worst_wins < 0) {
        return invalidPayload(`/trades.json periods.${grain}[${at}] has an invalid trade or win count.`);
      }
      if (bucket.worst_wins > bucket.wins || bucket.wins > bucket.trades) {
        return invalidPayload(
          `/trades.json periods.${grain}[${at}] does not satisfy worst_wins <= wins <= trades.`
        );
      }
      if (bucket.worst_paisa > bucket.best_paisa || bucket.largest_loss > bucket.largest_win) {
        return invalidPayload(`/trades.json periods.${grain}[${at}] contradicts its fill ordering.`);
      }
      if (grain === 'hour' && (bucket.key < 0 || bucket.key > 23)) {
        return invalidPayload(`/trades.json periods.hour[${at}].key is outside IST hours 0..23.`);
      }
      if (grain === 'weekday' && (bucket.key < 0 || bucket.key > 6)) {
        return invalidPayload(`/trades.json periods.weekday[${at}].key is outside weekdays 0..6.`);
      }
      const weekEnd = grain === 'week' ? checkedMultiply(bucket.key, 7) : null;
      const civilDays =
        grain === 'week'
          ? weekEnd === null
            ? null
            : checkedSubtract(weekEnd, 3)
          : bucket.key;
      if ((grain === 'day' || grain === 'week') && civilDays !== null) {
        const millis = checkedMultiply(civilDays, 86_400_000);
        const date = millis === null ? null : new Date(millis);
        if (millis === null || date === null || Number.isNaN(date.getTime())) {
          return invalidPayload(`/trades.json periods.${grain}[${at}].key cannot be labelled exactly.`);
        }
      } else if ((grain === 'day' || grain === 'week') && civilDays === null) {
        return invalidPayload(`/trades.json periods.${grain}[${at}].key cannot be labelled exactly.`);
      }
      if (grain === 'month' || grain === 'quarter' || grain === 'half') {
        const width = grain === 'month' ? 12 : grain === 'quarter' ? 4 : 2;
        const part = slot(bucket.key, width);
        if (checkedSubtract(bucket.key, part) === null) {
          return invalidPayload(`/trades.json periods.${grain}[${at}].key cannot be decoded exactly.`);
        }
      }
      for (const field of BUCKET_INTEGER_FIELDS) {
        if (bucket[field] !== expected[at][field]) {
          return invalidPayload(
            `/trades.json periods.${grain}[${at}].${field} is ${String(bucket[field])}, not ${String(expected[at][field])} from the exact trade timestamp bucket.`
          );
        }
      }
      if (at > 0 && bucket.key <= buckets[at - 1].key) {
        return invalidPayload(
          `/trades.json periods.${grain} is not in canonical increasing key order.`
        );
      }
    }
  }

  return {
    ok: true,
    rows,
    periods: body.periods,
    policy: body.policy,
    direction: body.direction,
    empty: rows.length === 0,
    why: ''
  };
}

/** @param {number} value @param {number} divisor */
const slot = (value, divisor) => ((value % divisor) + divisor) % divisor;

/** @param {number} minutes */
const hhmm = (minutes) =>
  `${String(Math.floor(minutes / 60)).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}`;

/**
 * Label one whole exchange-calendar hour. Rust has already shifted the stored
 * epoch timestamp to IST before writing the key, so applying the offset again
 * here would move every bucket by another five and a half hours.
 *
 * @param {number} istHour
 */
export function istHourLabel(istHour) {
  if (!Number.isSafeInteger(istHour)) return `IST hour ${String(istHour)}`;
  const start = slot(istHour * 60, 1_440);
  return `${hhmm(start)}–${hhmm(slot(start + 60, 1_440))}`;
}

/** @param {number} days */
function civilDay(days) {
  if (!Number.isSafeInteger(days)) return `day ${String(days)}`;
  const millis = days * 86_400_000;
  if (!Number.isSafeInteger(millis)) return `day ${String(days)}`;
  const date = new Date(millis);
  return Number.isNaN(date.getTime()) ? `day ${String(days)}` : date.toISOString().slice(0, 10);
}

/**
 * Decode the exact integer keys written by `cli::trades::Period`.
 *
 * @param {string} grain
 * @param {number} key
 */
export function periodLabel(grain, key) {
  if (!Number.isSafeInteger(key)) return `${grain} ${String(key)}`;
  if (grain === 'hour') return istHourLabel(key);
  if (grain === 'weekday') return WEEKDAY_NAMES[key] ?? `weekday ${key}`;
  if (grain === 'day') return civilDay(key);
  if (grain === 'week') {
    const weekEnd = checkedMultiply(key, 7);
    const monday = weekEnd === null ? null : checkedSubtract(weekEnd, 3);
    return monday === null ? `week ${String(key)}` : `Mon ${civilDay(monday)}`;
  }
  if (grain === 'year') return String(key);

  const width = grain === 'month' ? 12 : grain === 'quarter' ? 4 : grain === 'half' ? 2 : 0;
  if (width === 0) return `${grain} ${key}`;
  const part = slot(key, width);
  const year = (key - part) / width;
  if (grain === 'month') return `${MONTH_NAMES[part]} ${year}`;
  if (grain === 'quarter') return `Q${part + 1} ${year}`;
  return `H${part + 1} ${year}`;
}

/** @param {any[]} buckets @param {string} grain */
function shape(buckets, grain) {
  return (Array.isArray(buckets) ? buckets : [])
    .map((bucket) => {
      const trades = bucket.trades ?? 0;
      const wins = bucket.worst_wins ?? 0;
      return {
        key: bucket.key,
        label: periodLabel(grain, bucket.key),
        trades,
        wins,
        losses: trades - wins,
        rateBp: trades ? Math.round((wins / trades) * 10_000) : 0,
        worst: bucket.worst_paisa ?? 0,
        best: bucket.best_paisa ?? 0
      };
    })
    .sort((a, b) => integerOrder(a.key, b.key));
}

/**
 * Fold rows with the same slot across years.  This is only for the two
 * at-a-glance tiles: weekday (seven slots) and calendar month (twelve slots).
 * The chronological `day`/`month` series remain separate below.
 *
 * @param {number} size
 * @param {any[]} rows
 * @param {(index: number) => number} keyOf
 * @param {(index: number) => string} label
 */
function fill(size, rows, keyOf, label) {
  return Array.from({ length: size }, (unused, index) => {
    const wanted = keyOf(index);
    const hits = rows.filter((row) => slot(row.key, size) === slot(wanted, size));
    const trades = hits.reduce((sum, row) => sum + row.trades, 0);
    const wins = hits.reduce((sum, row) => sum + row.wins, 0);
    return {
      key: index,
      label: label(index),
      trades,
      wins,
      losses: trades - wins,
      rateBp: trades ? Math.round((wins / trades) * 10_000) : 0,
      worst: hits.reduce((sum, row) => sum + row.worst, 0),
      best: hits.reduce((sum, row) => sum + row.best, 0)
    };
  });
}

/** @param {any[]} rows @param {number} floor */
function best(rows, floor) {
  const eligible = rows.filter((row) => row.trades >= floor);
  return eligible.length === 0
    ? null
    : eligible.reduce((winner, row) => (row.rateBp > winner.rateBp ? row : winner));
}

/**
 * Shape all eight durable period grains.  No grain is silently discarded.
 *
 * @param {any} periods
 * @returns {any}
 */
export function timePatternRows(periods) {
  if (!periods || typeof periods !== 'object') return null;

  const grains = Object.fromEntries(
    TIME_GRAINS.map(({ key }) => [key, shape(periods[key], key)])
  );
  if (TIME_GRAINS.every(({ key }) => grains[key].length === 0)) return null;

  const weekdays = fill(
    7,
    grains.weekday,
    (index) => (index + 6) % 7,
    (index) => WEEKDAY_NAMES[(index + 6) % 7]
  );
  const hours = fill(24, grains.hour, (index) => index, istHourLabel);
  const calendarMonths = fill(
    12,
    grains.month,
    (index) => index,
    (index) => MONTH_NAMES[index]
  );
  const total = grains.hour.reduce((sum, row) => sum + row.trades, 0);
  const floor = Math.max(1, Math.round(total / 20));

  return {
    // TradingView's time-pattern plot keeps every hour and weekday in a fixed
    // position. Zero-filled slots are evidence too: their absence must not
    // compress the x axis and make 09:30 appear beside 14:30.
    grains: { ...grains, hour: hours, weekday: weekdays, calendarMonth: calendarMonths },
    bestHour: best(hours, floor),
    bestWeekday: best(weekdays, floor),
    bestCalendarMonth: best(calendarMonths, floor),
    floor,
    total
  };
}

/**
 * Return one clamped, deterministic window from an output-sensitive series.
 * The full recorded series remains available to the caller; only this slice is
 * admitted to the DOM.
 *
 * @param {unknown} input
 * @param {number} requestedPage
 * @param {number} pageSize
 */
export function windowSeries(input, requestedPage, pageSize) {
  const all = Array.isArray(input) ? input : [];
  const width = Number.isSafeInteger(pageSize) && pageSize > 0 ? pageSize : 1;
  const pages = Math.max(1, Math.ceil(all.length / width));
  const wanted = Number.isSafeInteger(requestedPage) ? requestedPage : 0;
  const page = Math.max(0, Math.min(wanted, pages - 1));
  const start = page * width;
  return {
    rows: all.slice(start, start + width),
    page,
    pages,
    start,
    total: all.length
  };
}

/**
 * Produce the complete consecutive-result series in resolution order.
 *
 * A flat worst-fill result is a losing-streak member.  That is deliberately
 * the same rule as Rust's `runner::grid::tally_trade`: only `pess > 0` is a
 * win, and a trade that returned nothing did not carry the winning run.  The
 * distribution donut may still show flats separately; the two views answer
 * different questions.
 *
 * @param {unknown} input
 */
export function streakSeries(input) {
  const ordered = (Array.isArray(input) ? [...input] : []).sort((a, b) =>
    integerOrder(a?.seq ?? 0, b?.seq ?? 0)
  );
  /** @type {Array<{index: number, kind: 'win'|'loss', count: number, amount: number, startSeq: number, endSeq: number}>} */
  const streaks = [];
  /** @type {'win'|'loss'|null} */ let kind = null;
  let count = 0;
  let amount = 0;
  let startSeq = 0;
  let endSeq = 0;

  const finish = () => {
    if (kind === null || count === 0) return;
    streaks.push({ index: streaks.length + 1, kind, count, amount, startSeq, endSeq });
  };

  for (const row of ordered) {
    const value = Number.isFinite(row?.worst) ? row.worst : 0;
    const nextKind = value > 0 ? 'win' : 'loss';
    const sequence = Number.isSafeInteger(row?.seq) ? row.seq : 0;
    if (kind !== nextKind) {
      finish();
      kind = nextKind;
      count = 0;
      amount = 0;
      startSeq = sequence;
    }
    count += 1;
    amount += value;
    endSeq = sequence;
  }
  finish();

  const wins = streaks.filter((streak) => streak.kind === 'win');
  const losses = streaks.filter((streak) => streak.kind === 'loss');
  /** @param {typeof streaks} rows */
  const average = (rows) =>
    rows.length === 0 ? null : rows.reduce((sum, row) => sum + row.count, 0) / rows.length;
  /** @param {typeof streaks} rows */
  const longest = (rows) => rows.reduce((largest, row) => Math.max(largest, row.count), 0);

  return {
    ordered,
    streaks,
    longestWin: longest(wins),
    longestLoss: longest(losses),
    avgWinStreak: average(wins),
    avgLossStreak: average(losses)
  };
}
