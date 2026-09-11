// A 200 THAT NEVER REACHED THE API IS NOT A RECEIPT.
//
// `readReceipt` parsed any 200 as one, and with no `.badge` in the document the
// fallback `good: ok` painted a green "OK" verdict with a blank reason over a
// request no server ever saw.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { notAReceipt, RECEIPT_HEADER, RECEIPT_SPOT } from '../src/lib/receipt.js';

const SHELL = '<!doctype html><html><head><title>brutex</title></head><body><div id="app"></div></body></html>';

test('a 200 with no receipt marker is refused, whatever it carries', () => {
  // THE REPRODUCED CASE: `/pull/spot` missing from web/vite.config.js. The dev
  // server answers its own index.html, 200, text/html — and without the header
  // `pull_spot` puts on every answer it gives (D-0124).
  const verdict = notAReceipt({
    html: SHELL,
    status: 200,
    ctype: 'text/html',
    marker: null,
    hasVerdict: false
  });
  assert.ok(verdict, 'nothing between the browser and the handler invents that header');
  assert.equal(verdict.good, false);
  assert.match(verdict.reason, new RegExp(`${RECEIPT_HEADER}: ${RECEIPT_SPOT}`));
  assert.match(verdict.reason, /did not come from it/);
  // AND A DIFFERENT MARKER IS NAMED RATHER THAN GLOSSED.
  const other = notAReceipt({
    html: SHELL,
    status: 200,
    ctype: 'text/html',
    marker: 'pull-fno',
    hasVerdict: true
  });
  // `assert.ok` FIRST, as every other case in this file does. It is not a type
  // ceremony: `notAReceipt` returns `null` when the answer IS a receipt, so
  // reaching `.reason` without this would read a field off nothing on the very
  // regression the test exists to catch — the refusal quietly not happening.
  assert.ok(other, 'a different marker is not this route\'s receipt');
  assert.match(other.reason, /it carried pull-fno/);
});

test('a marked answer with no verdict in it is still refused', () => {
  const verdict = notAReceipt({
    html: SHELL,
    status: 200,
    ctype: 'text/html',
    marker: RECEIPT_SPOT,
    hasVerdict: false
  });
  assert.ok(verdict, 'a page with no verdict in it is not a receipt');
  assert.equal(verdict.good, false, 'and it is never green');
  assert.match(verdict.verdict, /NOT A RECEIPT/);
  assert.match(verdict.reason, /carrying no verdict/);
  assert.match(verdict.reason, /nothing in it is a measurement/);
  assert.deepEqual(verdict.facts, []);
  assert.equal(verdict.raw, SHELL, 'the body is kept, so the operator can look at it');
});

test('a JSON or plain-text answer is refused before it is parsed at all', () => {
  for (const ctype of ['application/json', 'text/plain; charset=utf-8', '', null]) {
    const verdict = notAReceipt({
      html: '{"ok":true}',
      status: 200,
      ctype,
      marker: RECEIPT_SPOT,
      hasVerdict: true
    });
    assert.ok(verdict, `${ctype} is not a receipt`);
    assert.equal(verdict.good, false);
    assert.match(verdict.reason, /not HTML/);
    assert.match(verdict.reason, /proxy list in web\/vite\.config\.js/);
  }
});

test('the API is not behind this route is said in the answer, not inferred', () => {
  const verdict = notAReceipt({
    html: '',
    status: 200,
    ctype: 'application/json',
    marker: RECEIPT_SPOT,
    hasVerdict: true
  });
  assert.ok(verdict, 'the route is not behind this server, so nothing it sent is a receipt');
  assert.match(verdict.reason, /The API is not behind this route/);
  assert.match(verdict.reason, /Nothing was asked of any vendor/);
});

test("the server's own receipt passes both tests untouched", () => {
  assert.equal(
    notAReceipt({
      html: '<div class="badge good">STORED</div>',
      status: 200,
      ctype: 'text/html; charset=utf-8',
      marker: RECEIPT_SPOT,
      hasVerdict: true
    }),
    null
  );
  // A REFUSAL IS STILL A RECEIPT. 400 with a verdict in it is the server
  // answering, and the page must render the server's own words.
  assert.equal(
    notAReceipt({
      html: '<div class="badge bad">REFUSED</div>',
      status: 400,
      ctype: 'text/html; charset=utf-8',
      marker: RECEIPT_SPOT,
      hasVerdict: true
    }),
    null
  );
});
