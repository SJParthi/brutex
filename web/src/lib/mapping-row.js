/**
 * ONE ROW OF /mapping, AND THE NAMES AN AMBIGUOUS ABBREVIATION COLLIDED WITH
 * (WP-61; DB-G27).
 *
 * The page promised "seeing the nine is the difference between a refusal and
 * a decision waiting to be made", and the server sent only a count, so the row
 * read "3 published names accept this abbreviation" with no names. `/indexmap
 * .json` now carries `candidate_names` beside `candidates` (WP-20), on an
 * ambiguous row and on a renamed row's `unaided` reading (what the rule alone
 * would have said before the alias table settled it, e.g. NIFTYFINSERVICE ->
 * three FINANCIAL SERVICES indices). Pure, so `web/tests/mapping-row.test.js`
 * drives it under `node --test`.
 */

/**
 * The collided names of `row`: its own when ambiguous, else its `unaided`
 * reading's when that was ambiguous, else none. `count` is the server's count,
 * so a list shorter than it (an older API) is said rather than hidden.
 *
 * @param {any} row
 * @returns {{ names: string[], count: number, unaided: boolean } | null}
 */
export function candidatesOf(row) {
  if (!row || typeof row !== 'object') return null;
  const own = row.why === 'ambiguous' ? row : null;
  const unaided =
    !own && row.unaided && typeof row.unaided === 'object' && row.unaided.why === 'ambiguous'
      ? row.unaided
      : null;
  const from = own ?? unaided;
  if (!from) return null;
  const names = Array.isArray(from.candidate_names)
    ? from.candidate_names.filter((/** @type {unknown} */ name) => typeof name === 'string')
    : [];
  const count = Number.isSafeInteger(from.candidates) ? from.candidates : names.length;
  return { names, count, unaided: unaided !== null };
}

/**
 * The row's "Why" sentence.
 *
 * @param {any} row a tagged row: the server's fields plus `kind`
 * @returns {string}
 */
export function noteOf(row) {
  const collided = candidatesOf(row);
  if (row.kind === 'ambiguous') {
    const listed = collided && collided.names.length > 0;
    const missing = collided && collided.names.length < collided.count;
    return `${collided?.count ?? row.candidates} published names accept this abbreviation${
      listed ? ', listed below' : ''
    }. Picking one would be a guess.${
      missing
        ? ` The server named ${collided.names.length} of the ${collided.count}: this API predates the names, so the rest are not shown, not absent.`
        : ''
    }`;
  }
  if (row.kind === 'absent') {
    return 'No published name accepts it. Either the vendor reordered the words, dropped a number the name carries, or this is not an NSE index at all.';
  }
  if (row.kind === 'abbreviation') return 'Shortens exactly one published name.';
  if (row.kind === 'aliased') {
    const source = row.alias && typeof row.alias.source === 'string' ? ` Source: ${row.alias.source}.` : '';
    const unaided = collided
      ? ` The abbreviation rule alone matches ${collided.count} published names (below); the alias table, not the rule, chose this one.`
      : '';
    return `This engine renamed it. The exchange publishes the vendor's own name for it, and the store keys on ${row.symbol} instead.${unaided}${source}`;
  }
  return 'The symbol is the published name.';
}

/**
 * Where the join's list of symbols came from, as the census card's caption:
 * a feed's master, or (GDFL, which ships none) the index series its store
 * holds (WP-20 builds `indexmap` from the census when a feed has no master).
 *
 * @param {any} join the `/indexmap.json` body
 * @returns {{ label: string, sentence: string }}
 */
export function listedFrom(join) {
  if (join?.listed_from === 'census') {
    return {
      label: 'Stored by feed',
      sentence:
        "Index series this feed's store holds. It ships no instrument master, so the list is the census's: stored, not mastered."
    };
  }
  return { label: 'Listed by feed', sentence: "Index symbols in this feed's master." };
}
