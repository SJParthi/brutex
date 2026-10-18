// AH-04 (g-ui-dom-vs-json-5, D-5882): which /markets rows the store HOLDS.
//
// `/instruments.json`'s `bars` counts a row's SPOT series only, and a GDFL row
// for an option underlying whose spot is not filed under its own name says so
// with `spot: false, options: true` beside `bars: 0`. /markets read neither
// flag: FINNIFTY, MIDCPNIFTY, NIFTYNXT50 and fourteen renamed or removed share
// underlyings read "0 = measured zero", "not stored", "it can be pulled — it
// just has not been", while they held 57,731 option contracts. Pure, so
// `web/tests/markets-options-only.test.js` drives every state.

/**
 * A row's spot count when it is a whole number of bars, else `null`.
 *
 * @param {any} row
 */
export function barsOf(row) {
  return Number.isInteger(row?.bars) && row.bars >= 0 ? row.bars : null;
}

/**
 * FIVE ANSWERS FOR ONE COLUMN, AND THEY NEVER LOOK ALIKE.
 *
 *   count    — spot bars on disk, counted.
 *   options  — no spot bar under this name, and the server says the feed
 *              holds this underlying's OPTION contracts (`options: true`).
 *              Held. Never "never pulled", never offered "Pull it".
 *   zero     — measured: no spot bar and no option contract.
 *   unread   — `bars` is absent or not a whole number: unknown, never 0, even
 *              when `options` is true.
 *   nomaster — the master on hand is not this feed's; nothing is measured.
 *
 * O(1) per row: two field reads.
 *
 * @param {any} row
 * @param {{ isThisFeed: boolean, wait: boolean, why: string }} master
 * @returns {{ kind: 'nomaster', wait: boolean, why: string }
 *          | { kind: 'unread', why: string }
 *          | { kind: 'zero' }
 *          | { kind: 'options' }
 *          | { kind: 'count', n: number }}
 */
export function barStateOf(row, master) {
  if (!master.isThisFeed) return { kind: 'nomaster', wait: master.wait, why: master.why };
  const n = barsOf(row);
  if (n === null) {
    return {
      kind: 'unread',
      why: `this row's \`bars\` field is ${JSON.stringify(row?.bars)}${row?.options === true ? ' (the server says option contracts are held; the spot count is what is unread)' : ''}`
    };
  }
  if (n === 0) return row?.options === true ? { kind: 'options' } : { kind: 'zero' };
  return { kind: 'count', n };
}

/**
 * Whether a state is HELD: spot bars counted, or option contracts held.
 *
 * @param {{ kind: string }} st
 */
export function isHeld(st) {
  return st.kind === 'count' || st.kind === 'options';
}

/**
 * THE PARTITION, AND IT SUMS: held (of which options only), not stored,
 * unread. A `nomaster` row is in no bucket — the footer says the partition is
 * not counted instead. O(rows), once per master, never per keystroke.
 *
 * @param {any[]} rows
 * @param {(row: any) => { kind: string }} stateOf
 */
export function heldTally(rows, stateOf) {
  let held = 0;
  let optionsOnly = 0;
  let zero = 0;
  let unread = 0;
  for (const row of rows) {
    const st = stateOf(row);
    if (st.kind === 'count') held += 1;
    else if (st.kind === 'options') {
      held += 1;
      optionsOnly += 1;
    } else if (st.kind === 'zero') zero += 1;
    else if (st.kind === 'unread') unread += 1;
  }
  return { held, optionsOnly, zero, unread };
}

/**
 * What `/aliases.json?symbol=<symbol>` says about where an options-only
 * row's spot lives, in a sentence, or `null` while it is unread.
 *
 * `matched: 'index'` names the INDEX key that holds the spot (FINNIFTY is
 * filed as NIFTYFINSERVICE); `symbol_change` names the rename and the newest
 * symbol, whose CASH key holds the share's later bars; `not_aliased` gives
 * the table's own reason; `none` says the table names no other key. Never a
 * guessed key.
 *
 * @param {any} answer the parsed `/aliases.json?symbol=` body, or an Error
 * @param {string} symbol
 * @returns {{ spotKey: string | null, text: string } | null}
 */
export function spotHome(answer, symbol) {
  if (answer == null) return null;
  if (answer instanceof Error) {
    return { spotKey: null, text: `Where ${symbol}'s spot is filed could not be read: ${answer.message}.` };
  }
  const key = typeof answer.key === 'string' ? answer.key : null;
  if (answer.matched === 'index' && key) {
    return {
      spotKey: key,
      text: `The alias table files ${symbol}'s spot under ${answer.segment ?? 'INDEX'} ${answer.store_key ?? key} (${key}), not under this name.`
    };
  }
  if (answer.matched === 'symbol_change') {
    const changes = Array.isArray(answer.changes) ? answer.changes : [];
    const formerly = Array.isArray(answer.formerly) ? answer.formerly : [];
    const fwd = changes[0];
    if (fwd && typeof fwd.new === 'string') {
      const newest = typeof fwd.current === 'string' ? fwd.current : fwd.new;
      return {
        spotKey: typeof fwd.key === 'string' ? fwd.key : null,
        text: `${symbol} was renamed ${fwd.new} from ${fwd.effective ?? 'a date the table states'}; the newest symbol in its chain is ${newest}${typeof fwd.key === 'string' ? `, filed as ${fwd.key}` : ''}.`
      };
    }
    const back = formerly[0];
    if (back && typeof back.old === 'string') {
      return { spotKey: key, text: `${symbol} was formerly ${back.old} (renamed ${back.effective ?? 'on a date the table states'}).` };
    }
  }
  if (answer.matched === 'not_aliased') {
    const why = typeof answer.not_aliased?.why === 'string' ? answer.not_aliased.why : 'the table keeps it on its own key';
    return { spotKey: null, text: `The alias table does not alias ${symbol}: ${why}.` };
  }
  return {
    spotKey: null,
    text: `The alias table names no other key for ${symbol}, so where its spot is filed, if anywhere, is not known here.`
  };
}
