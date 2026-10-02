// Gate W5's program, moved out of .github/workflows/ci.yml by D-1100: an
// inline `node -e` there was JavaScript in a tracked file outside web/.
// Run by path: node web/ci/css-comments.mjs FILE...
import fs from 'node:fs';
import process from 'node:process';

const files = process.argv.slice(2);
if (files.length === 0) {
  console.log("Gate W5 was handed no file: a gate that reads nothing is not a gate.");
  process.exit(1);
}
let bad = 0;
for (const f of files) {
  const src = fs.readFileSync(f, "utf8");
  // A `.css` file IS the stylesheet; a `.svelte` file carries one.
  // `theme.css` is 2,052 lines and the first version of this gate
  // globbed past it, along with `routes/+page.svelte`, whose path
  // has no directory between `routes/` and the filename. A gate
  // that names its inputs by hand misses the ones nobody thought
  // of, so this walks the tree instead.
  //
  // THE SPLIT THIS REPLACES READ THE WRONG BLOCK ON A REAL FILE.
  // `split("<style>")[1]` takes whatever follows the FIRST of that
  // eight-character string, and on `routes/+page.svelte` the second
  // occurrence is PROSE -- six lines into the style block a comment
  // reads "A literal in a page <style> is a second theme the toggle
  // cannot reach". So the split handed 387 characters to a scanner
  // owed 35,472, and the fragment ends inside a comment, so the
  // gate reported an unterminated comment that is not there.
  //
  // The opening tag is matched at COLUMN ZERO and the closing tag
  // is required to start a line, which is where a Svelte component
  // puts its one top-level style element and where prose about the
  // tag never sits. Attributes are allowed: `<style lang="scss">`
  // is the same element.
  let css = null;
  let base = 0;
  if (f.endsWith(".css")) {
    css = src;
  } else {
    const open = /^<style(?:[^>\n]*)?>/m.exec(src);
    if (open) {
      base = open.index + open[0].length;
      const rest = src.slice(base);
      const shut = rest.indexOf("\n</style>");
      if (shut !== -1) css = rest.slice(0, shut);
    }
    // A FILE THAT CARRIES THE TAG AND NO READABLE BLOCK IS LOUD.
    // Falling quietly through is how the version of this gate that
    // was handed ZERO filenames printed its success line every run.
    if (css === null && src.includes("<style")) {
      console.log(`${f}: carries a <style tag but no top-level <style> ... </style> block this gate can read`);
      bad++;
    }
  }
  if (css === null) continue;
  const opens = (css.match(/\/\*/g) || []).length;
  const closes = (css.match(/\*\//g) || []).length;
  // 1-based line number in the ORIGINAL file, so a finding can be
  // opened rather than searched for.
  const at = (k) => src.slice(0, base + k).split("\n").length;
  let i = 0;
  while (true) {
    const a = css.indexOf("/*", i);
    if (a === -1) break;
    const b = css.indexOf("*/", a + 2);
    const body = css.slice(a, b === -1 ? undefined : b).split("\n");
    if (b === -1) {
      console.log(`${f}:${at(a)}: UNTERMINATED comment: ${body[0].slice(0, 70)}`);
      bad++;
      break;
    }
    // ANY line of the body, not just the first: the `*/` is deleted
    // wherever the prose happened to end, which on a three-line
    // comment is the third line.
    const k = body.findIndex((l) => l.trimEnd().endsWith("{"));
    if (k !== -1) {
      console.log(`${f}:${at(a) + k}: comment swallows the rule under it: ${body[k].trim().slice(0, 70)}`);
      bad++;
    }
    i = b + 2;
  }
  if (opens !== closes) {
    console.log(`${f}: note, not a failure: ${opens} "/*" against ${closes} "*/" -- a nested opener has no meaning to CSS and is counted anyway`);
  }
}
if (bad) {
  console.log("");
  console.log("A comment that runs past its rule leaves that styling INERT while the");
  console.log("build stays green: svelte-check cannot see it and Gate W4 cannot count");
  console.log("it, because to every tool involved the text is prose.");
  process.exit(1);
}
console.log("no comment swallows a rule");
