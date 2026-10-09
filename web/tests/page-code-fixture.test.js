// THE HELPER THAT REPLACED RAW-TEXT MATCHING, TESTED ITSELF (P19-05, D-2564).
//
// `codeOf` is only worth using if a comment can no longer satisfy a match and
// code that merely LOOKS like a comment still can. Both halves are pinned here
// on a synthetic component, so a regression in the helper fails by name rather
// than silently weakening every page test built on it.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { codeOf, declarations, effects, templateExpressions } from './page-code-fixture.js';

const COMPONENT = `<script>
  // keepAlive() only in a line comment
  /* return () => { abort(); }; */
  /** jsdoc names hiddenDoc() */
  const kept = 'a // string, not a comment';
  const pattern = /\\/\\*/;
  const url = \`/* template literal */\`;
  let open = $state(false);
  function shown(x) { return x + 1; }
  $effect(() => {
    if (open) shown(1);
    return () => { open = false; };
  });
</script>

<!-- <p>{commentedOut(r.exp)}</p> -->
<p title={open ? 'yes' : 'no'}>{shown(2)}</p>
{#if open}<span>{kept}</span>{/if}
`;

test('codeOf blanks script and markup comments and keeps everything that runs', () => {
  const code = codeOf(COMPONENT);
  assert.equal(code.length, COMPONENT.length, 'offsets survive');
  assert.equal(code.split('\n').length, COMPONENT.split('\n').length, 'line numbers survive');
  for (const gone of ['keepAlive()', 'abort();', 'hiddenDoc()', 'commentedOut(r.exp)', '<!--', '-->']) {
    assert.equal(code.includes(gone), false, `${gone} sat in a comment and must not match`);
  }
  for (const kept of [
    "'a // string, not a comment'",
    'const pattern = /\\/\\*/;',
    '`/* template literal */`',
    'function shown(x) { return x + 1; }',
    "{open ? 'yes' : 'no'}",
    '{shown(2)}',
    'return () => { open = false; };'
  ]) {
    assert.equal(code.includes(kept), true, `${kept} is code and must still match`);
  }
});

test('codeOf on a component with no comments is the identity', () => {
  const plain = '<script>\n  let a = 1;\n</script>\n<p>{a}</p>\n';
  assert.equal(codeOf(plain), plain);
  assert.equal(codeOf('<p>plain</p>'), '<p>plain</p>', 'no script at all');
});

test('the extraction helpers return only live declarations, effects and markup expressions', () => {
  assert.match(declarations(COMPONENT, ['shown']), /^function shown\(x\)/);
  assert.match(declarations(COMPONENT, ['kept', 'open']), /const kept[\s\S]*let open/);
  assert.throws(() => declarations(COMPONENT, ['keepAlive']), /declares no top-level keepAlive/);
  assert.throws(() => declarations(COMPONENT, ['shown', 'missing']), /missing/);
  const found = effects(COMPONENT);
  assert.equal(found.length, 1);
  assert.match(found[0], /^\(\) => \{/);
  const expressions = templateExpressions(COMPONENT);
  for (const live of ["open ? 'yes' : 'no'", 'shown(2)', 'kept']) {
    assert.ok(expressions.includes(live), `${live} is a rendered expression: ${JSON.stringify(expressions)}`);
  }
  assert.equal(expressions.some((text) => text.includes('commentedOut')), false, 'a {…} inside <!-- --> is not markup');
});
