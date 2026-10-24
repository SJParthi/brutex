// R4-28 repair 5: which expired-derivative segments /ingest asks each feed for,
// by the `fno` word /feeds.json states for it. Zerodha states `by_token` since
// D-7186; the page knew four words and declined it as "states no
// expired-derivative history", which is false for it. Every fixture is
// invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { FNO_MODES, fnoDeclinedWhy, fnoPartialWhy, fnoSegmentsOf } from '../src/lib/fno-modes.js';

const ROUTE = readFileSync(new URL('../../crates/api/src/feeds_route.rs', import.meta.url), 'utf8');
const PAGE = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');

/** The quoted words of the server's `let fno = match ...` block. */
function serverWords() {
  const start = ROUTE.indexOf('let fno = match feed.descriptor().transport {');
  assert.ok(start >= 0, 'the server still states the word in one match');
  const end = ROUTE.indexOf('};', start);
  const block = ROUTE.slice(start, end)
    .split('\n')
    .filter((line) => !line.trim().startsWith('//'))
    .join('\n');
  return [...block.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]).sort();
}

test('the_page_knows_every_word_the_server_states_and_no_other', () => {
  assert.deepEqual(serverWords(), [...FNO_MODES].sort());
});

test('a_token_feed_is_asked_for_futures_and_named_partial_never_declined', () => {
  assert.deepEqual(fnoSegmentsOf('by_token'), ['futures']);
  assert.equal(fnoDeclinedWhy('by_token'), '', 'not declined');
  const partial = fnoPartialWhy('by_token');
  assert.match(partial, /^serves expired futures only — /);
  assert.match(partial, /refuses expired options for it by name/);
  assert.doesNotMatch(partial, /no expired-derivative history/);
});

test('the_four_words_the_page_knew_answer_as_they_did', () => {
  assert.deepEqual(fnoSegmentsOf('by_name'), ['futures', 'options']);
  assert.equal(fnoPartialWhy('by_name'), '', 'both segments: not partial');
  assert.equal(fnoDeclinedWhy('by_name'), '');
  assert.deepEqual(fnoSegmentsOf('by_strike_offset'), ['options']);
  assert.equal(
    fnoPartialWhy('by_strike_offset'),
    'serves expired options only — its endpoint is v2/charts/rollingoption, which addresses options by ATM±n and publishes no expired-futures series.'
  );
  assert.deepEqual(fnoSegmentsOf('local_folder'), []);
  assert.equal(
    fnoDeclinedWhy('local_folder'),
    'is a folder of files, not an endpoint — there is no expiry list to ask it for.'
  );
  assert.deepEqual(fnoSegmentsOf('none'), []);
  assert.equal(fnoDeclinedWhy('none'), 'states no expired-derivative history.');
  for (const how of ['local_folder', 'none']) assert.equal(fnoPartialWhy(how), '', how);
});

test('a_word_the_table_does_not_know_is_declined_and_named_never_read_as_none', () => {
  for (const how of ['by_magic', '', 'BY_TOKEN', 'toString', '__proto__', 42, null, undefined]) {
    assert.deepEqual(fnoSegmentsOf(how), [], String(how));
    assert.equal(fnoPartialWhy(how), '', String(how));
    const why = fnoDeclinedWhy(how);
    assert.match(why, /states an expired-derivative mode this page does not know/, String(how));
    assert.doesNotMatch(why, /no expired-derivative history/, String(how));
  }
  assert.match(fnoDeclinedWhy(undefined), /\(none stated\)/);
  assert.match(fnoDeclinedWhy('by_magic'), /\("by_magic"\)/);
});

test('the_segments_returned_are_copies_the_caller_cannot_use_to_change_the_table', () => {
  const segments = fnoSegmentsOf('by_name');
  segments.push('anything');
  assert.deepEqual(fnoSegmentsOf('by_name'), ['futures', 'options']);
  assert.ok(Object.isFrozen(FNO_MODES));
});

test('the_ingest_page_asks_the_table_and_keeps_no_word_list_of_its_own', () => {
  assert.match(PAGE, /from '\$lib\/fno-modes\.js'/);
  for (const stale of [
    "how === 'by_name'",
    "how === 'by_strike_offset'",
    "x.how !== 'by_name'",
    "'states no expired-derivative history.'"
  ]) {
    assert.ok(!PAGE.includes(stale), `the page no longer spells ${stale}`);
  }
});
