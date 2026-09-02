/**
 * Group a combination's condition names by the indicator family they belong to.
 *
 * # The problem this solves
 *
 * A live sweep's leading row reads, in full:
 *
 *   close_above_pdl close_below_supertrend close_above_pivot_s2_band
 *   close_above_pivot_s3_band close_above_pivot_s4_band
 *   close_below_pivot_r5_band close_above_pivot_s5_band
 *
 * Seven names in one flat line, and a reader has to hold all seven at once to
 * see that four of them are the same indicator at four levels. Grouped, the
 * same row reads `pivot: above s2, above s3, above s4, above s5, below r5  ·
 * supertrend: below  ·  pdl: above` — the shape of the setup is visible without
 * reading every token.
 *
 * # Why grouping and NOT implication-collapsing
 *
 * The obvious next step is to print only `above s2`, because the pivot supports
 * are ordered S5 < S4 < S3 < S2 and being above S2 implies being above the
 * rest. **That collapse is deliberately not done here.** These are `_band`
 * conditions — a tolerance window around a level rather than a bare inequality,
 * a distinction `crates/indicators/src/daily.rs:440` draws explicitly when it
 * notes that the `close_above_pdh` family is named WITHOUT `_band` and is
 * `Kind::Plain`. Whether one band's window can reach the next is a fact about
 * the tolerance table, and until that is measured, dropping a condition would
 * discard a real distinction and misreport what the engine actually tested.
 *
 * So this is presentation only: every condition in the mask is still shown, in
 * the order the mask holds it, and nothing is inferred.
 *
 * # Cost
 *
 * One pass with a `Map`, linear in the number of set bits — at most the mask's
 * popcount, and a combination is a handful of conditions.
 */

/** Matches `close_{above,below,near}_{rest}`. */
const RELATION = /^close_(above|below|near)_(.+)$/;

/**
 * Split one condition name into the family it belongs to and what it says.
 *
 * The vocabulary names most conditions `close_{above,below,near}_{family}` with
 * an optional `_band`, plus a long tail that follows no such shape (`pat_*`,
 * `inside_*`, bare indicator words). Anything unrecognised keeps its whole name
 * as its own family — the honest fallback, since it then renders exactly as it
 * did before rather than being forced into a group it may not belong to.
 *
 * @param {string} name
 * @returns {{family: string, detail: string}}
 */
export function splitCondition(name) {
  if (typeof name !== 'string' || name.length === 0) {
    return { family: '?', detail: '' };
  }
  const relation = name.match(RELATION);
  if (!relation) {
    return { family: name, detail: '' };
  }
  const side = relation[1];
  // `_band` is a rendering detail of the level, not a family of its own: a
  // reader comparing `pivot_s2_band` with `pivot_s3_band` is comparing two
  // levels of one indicator.
  const level = relation[2].replace(/_band$/, '');
  // `pivot_s2` -> family `pivot`, level `s2`. `ema20` has no underscore, so it
  // is its own family and the side is the whole detail.
  const parts = level.split('_');
  if (parts.length >= 2) {
    return { family: parts[0], detail: `${side} ${parts.slice(1).join(' ')}` };
  }
  return { family: level, detail: side };
}

/**
 * Group condition names by family, preserving first-seen order throughout.
 *
 * Order is preserved rather than sorted so two rows differing by one condition
 * still line up visually; sorting would move unrelated tokens and make a
 * one-condition difference look like a different setup.
 *
 * @param {string[]} names
 * @returns {{family: string, details: string[]}[]}
 */
export function groupConditions(names) {
  if (!Array.isArray(names)) return [];
  /** @type {Map<string, string[]>} */
  const groups = new Map();
  for (const name of names) {
    const { family, detail } = splitCondition(name);
    const held = groups.get(family);
    if (held) {
      held.push(detail);
    } else {
      groups.set(family, [detail]);
    }
  }
  return [...groups.entries()].map(([family, details]) => ({ family, details }));
}

/**
 * How many conditions the mask holds, and how many families they fall into.
 *
 * Both numbers are shown to the reader because their DIFFERENCE is the finding:
 * seven conditions across three families is a narrower setup than seven across
 * seven, and the flat list made those two look identical.
 *
 * @param {string[]} names
 * @returns {{conditions: number, families: number}}
 */
export function conditionShape(names) {
  const groups = groupConditions(names);
  return {
    conditions: Array.isArray(names) ? names.length : 0,
    families: groups.length
  };
}

/**
 * The pivot ladder, low to high. Rank is position; only ORDER is used.
 *
 * Written out rather than parsed from the names because the ordering is a fact
 * about the levels — S5 is the furthest support below the pivot and R5 the
 * furthest resistance above it — and a name is not evidence of a position.
 */
const PIVOT_LADDER = ['s5', 's4', 's3', 's2', 's1', 'bc', 'tc', 'r1', 'r2', 'r3', 'r4', 'r5'];

/** Ladders whose members are ordered, keyed by the family name. */
const LADDERS = new Map([['pivot', PIVOT_LADDER]]);

/**
 * Which conditions in a combination are IMPLIED by a tighter one beside them.
 *
 * # The rule, and it is the operator's own
 *
 * On an ordered ladder, two conditions in the SAME direction are not two facts.
 * `above s2` and `above s3` both say price is above a support, and since
 * s3 < s2 the first implies the second — only the tightest carries information.
 * Two conditions in OPPOSITE directions are a band and both matter: `above s2`
 * with `below s1` says price sits between them, which is a real, narrow setup.
 *
 * So within one family and one direction:
 *   `above`  — the highest-ranked level is informative, every lower one implied
 *   `below`  — the lowest-ranked level is informative, every higher one implied
 *
 * # What this does NOT do
 *
 * It does not drop the implied conditions, and it does not tell the engine
 * anything. It marks them, so a reader can see that a seven-condition row is
 * four conditions and three restatements. Dropping them would be a claim that
 * the implication is EXACT, and these are `_band` conditions — tolerance
 * windows around a level, not bare inequalities
 * (`crates/indicators/src/daily.rs:440`). Whether one band's window can reach
 * the next is a fact about the tolerance table that has not been measured here.
 *
 * @param {string[]} names
 * @returns {Set<string>} the names implied by a tighter condition in the same set
 */
export function impliedConditions(names) {
  /** @type {Set<string>} */
  const implied = new Set();
  if (!Array.isArray(names)) return implied;
  /** @type {Map<string, {name: string, rank: number}[]>} */
  const ladders = new Map();
  for (const name of names) {
    const { family, detail } = splitCondition(name);
    const ladder = LADDERS.get(family);
    if (!ladder) continue;
    const [side, level] = detail.split(' ');
    const rank = ladder.indexOf(level);
    if (rank < 0 || (side !== 'above' && side !== 'below')) continue;
    const key = `${family} ${side}`;
    const held = ladders.get(key);
    if (held) held.push({ name, rank });
    else ladders.set(key, [{ name, rank }]);
  }
  for (const [key, members] of ladders) {
    if (members.length < 2) continue;
    // `above` keeps the highest rank; `below` keeps the lowest. Everything else
    // in that direction restates it.
    const above = key.endsWith('above');
    let tightest = members[0];
    for (const member of members) {
      if (above ? member.rank > tightest.rank : member.rank < tightest.rank) {
        tightest = member;
      }
    }
    for (const member of members) {
      if (member.name !== tightest.name) implied.add(member.name);
    }
  }
  return implied;
}
