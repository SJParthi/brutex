// EVERY REQUEST HAS A CEILING, AND THIS IS WHAT KEEPS IT THAT WAY.
//
// `$lib/ask.js` exists because `AbortSignal.timeout` appeared zero times in
// `web/src`: a Rust process that accepted a connection and then wedged left the
// page reading "Reading…" forever, indistinguishable from slow. Twelve call
// sites were converted.
//
// Three of them were then reverted — not maliciously, just by a rewrite of the
// surrounding function in `ingest/+page.svelte` that spelled the call `fetch`
// again. Nothing objected: the build was clean, the tests were green, and no
// gate looked. The ceiling silently stopped existing on the pull, the folder
// probe and the status poll.
//
// A convention nothing checks is a convention that lasts until the next
// rewrite. This checks it.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const src = join(dirname(fileURLToPath(import.meta.url)), '..', 'src');

/**
 * Every file under `web/src`, at any depth.
 *
 * @param {string} dir
 * @returns {string[]}
 */
function walk(dir) {
  /** @type {string[]} */
  const out = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
}

test('no call site reaches the network without a ceiling', () => {
  /** @type {string[]} */
  const bare = [];
  for (const file of walk(src)) {
    if (!/\.(js|svelte)$/.test(file)) continue;
    if (file.endsWith(join('lib', 'ask.js'))) continue; // the one legal `fetch`
    const text = readFileSync(file, 'utf8');
    text.split('\n').forEach((line, i) => {
      // `globalThis.fetch` in a comment is prose; a call is `fetch(`.
      if (/(^|[^.\w])fetch\s*\(/.test(line) && !line.trim().startsWith('//') && !line.trim().startsWith('*')) {
        bare.push(`${file.slice(src.length + 1)}:${i + 1}: ${line.trim().slice(0, 70)}`);
      }
    });
  }
  assert.deepEqual(
    bare,
    [],
    'these reach the network with no timeout — import `ask` from `$lib/ask.js` ' +
      'instead, aliasing it if the scope already binds that name:\n  ' +
      bare.join('\n  ')
  );
});

test('ask is imported under a name nothing local can shadow', () => {
  // `ingest/+page.svelte` declares `const ask` inside `readFolder`. A
  // module-scope `import { ask }` is shadowed by it, so `await ask(url)` in
  // that helper called the helper — an unbounded recursion that built clean and
  // type-checked clean. Any file that both imports `ask` unaliased AND binds
  // the name locally is that bug waiting to happen.
  for (const file of walk(src)) {
    if (!/\.(js|svelte)$/.test(file)) continue;
    const text = readFileSync(file, 'utf8');
    if (!/import\s*\{[^}]*\bask\b(?!\s+as)[^}]*\}\s*from\s*'\$lib\/ask\.js'/.test(text)) continue;
    const shadows = /(?:const|let|var|function)\s+ask\b/.test(text);
    assert.ok(
      !shadows,
      `${file.slice(src.length + 1)} imports \`ask\` unaliased and also binds \`ask\` locally. ` +
        'The local binding wins inside its scope, so a call meant for the wrapper ' +
        'reaches the local one instead. Import it as another name.'
    );
  }
});
