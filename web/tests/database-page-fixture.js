import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { decorateRow, instrumentDescription } from '../src/lib/db/decoration.js';
import { MAX_PREFIX } from '../src/lib/db/search.js';

/**
 * EVERY FILE THE /db PAGE IS MADE OF, shell first.
 *
 * WP-00 split `routes/db/+page.svelte` into a shell plus one component or
 * module per later work package, moving code only. A test that asserts what
 * "the page" says reads all of these — the text it looks for is still on the
 * page, it is simply in the file its owner now holds.
 */
export const DB_PAGE_FILES = [
  '../src/routes/db/+page.svelte',
  '../src/routes/db/DbCensusBanner.svelte',
  '../src/routes/db/DbControls.svelte',
  '../src/routes/db/DbSearch.svelte',
  '../src/routes/db/DbMoneyness.svelte',
  '../src/routes/db/DbSummary.svelte',
  '../src/routes/db/DbBasisBadge.svelte',
  '../src/routes/db/DbCoverageGrid.svelte',
  '../src/routes/db/DbCalendar.svelte',
  '../src/routes/db/DbBarsGrid.svelte',
  '../src/routes/db/DbOptionCells.svelte',
  '../src/routes/db/DbExtremes.svelte',
  '../src/routes/db/DbDayFlags.svelte',
  '../src/routes/db/DbProvenance.svelte',
  '../src/routes/db/DbChart.svelte',
  '../src/routes/db/db.css',
  '../src/lib/db/selection.svelte.js',
  '../src/lib/db/selection-model.js',
  '../src/lib/db/notes-controls.js',
  '../src/lib/db/search.js',
  '../src/lib/db/contract-parse.js',
  '../src/lib/db/bars-plan.js',
  '../src/lib/db/notes-bars.js',
  '../src/lib/db/export.js',
  '../src/lib/db/chain.js',
  '../src/lib/db/notes-options.js',
  '../src/lib/db/decoration.js',
  '../src/lib/db/notes-coverage.js',
  '../src/lib/db/notes-labels.js'
];

/** @param {string} path relative to this file */
export const dbSource = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

/** The whole page's source, every file in `DB_PAGE_FILES` order. */
export const databaseSource = () => DB_PAGE_FILES.map(dbSource).join('\n');

/**
 * The first node in `root` (searched depth first) that satisfies `match`.
 * @param {any} root @param {(node: any) => boolean} match @returns {any}
 */
function find(root, match) {
  if (!root || typeof root !== 'object') return null;
  if (typeof root.type === 'string' && match(root)) return root;
  for (const [key, value] of Object.entries(root)) {
    if (key === 'parent' || key === 'loc' || key === 'metadata') continue;
    if (Array.isArray(value)) {
      for (const item of value) { const hit = find(item, match); if (hit) return hit; }
    } else if (value && typeof value === 'object') {
      const hit = find(value, match);
      if (hit) return hit;
    }
  }
  return null;
}

/**
 * Exercise the actual page's pure decoration, text selection and seeding; no
 * substitute implementation.
 *
 * DECORATION is the real module the page imports. THE TEXT SELECTION is the
 * `textMatched` derived inside `searchState` (DbSearch.svelte), and THE SEEDS
 * are the effects inside `selectionSeeds` (selection.svelte.js): each is cut
 * out of the file that holds it and run over plain inputs, exactly as this
 * fixture ran them when they were in the page script. A name another factory
 * publishes is read as `db.<name>` there, so the inputs arrive on a `db`.
 *
 * @param {string} [source] the page's whole source; `databaseSource()` by default
 */
export function databasePage(source = databaseSource()) {
  const search = dbSource('../src/routes/db/DbSearch.svelte');
  const matched = find(parse(search).module?.content, (node) => node.type === 'VariableDeclarator' &&
    node.id?.type === 'Identifier' && node.id.name === 'textMatched');
  const init = matched?.init;
  if (init?.type !== 'CallExpression' || init.arguments[0]?.type !== 'ArrowFunctionExpression') throw new Error('Database text selection seam changed.');
  const expression = search.slice(init.arguments[0].start, init.arguments[0].end);
  const pick = new Function('index', 'db', 'MAX_PREFIX', `return (${expression})();`);
  /** @param {any} index @param {string} typed @param {any} universed @param {any} universeNames */
  const select = (index, typed, universed, universeNames) => pick(index, { typed, universed, universeNames }, MAX_PREFIX);

  // A plain module has no Svelte parse of its own; wrapped as a module script it
  // parses with the same compiler `source-digest.test.js` uses for a config file.
  const wrapped = '<script module>' + dbSource('../src/lib/db/selection.svelte.js') + '</script>';
  const seeds = find(parse(wrapped).module?.content, (node) => node.type === 'FunctionDeclaration' && node.id?.name === 'selectionSeeds');
  if (!seeds) throw new Error('Database seed seam changed.');
  const effects = seeds.body.body.filter((/** @type {any} */ node) =>
    node.type === 'ExpressionStatement' && node.expression.type === 'CallExpression' &&
    node.expression.callee.type === 'Identifier' && node.expression.callee.name === '$effect');
  /** @param {string} name @param {string[]} inputs */
  const seed = (name, inputs) => {
    const statements = effects.map((/** @type {any} */ node) => wrapped.slice(node.expression.arguments[0].start, node.expression.arguments[0].end));
    const body = statements.find((/** @type {string} */ text) => text.includes(`if (db.${name} === '' && `));
    if (!body) throw new Error(`Database ${name} seed changed.`);
    const run = new Function('db', `(${body})(); return db.${name};`);
    return (/** @type {any[]} */ ...values) => run(Object.fromEntries([name, ...inputs].map((key, i) => [key, values[i]])));
  };
  // `any`, as the cut-out functions this fixture used to build were: callers
  // were written against that, and this fixture's job is the seam, not a retype.
  return { decorateRow: /** @type {any} */ (decorateRow), instrumentDescription: /** @type {any} */ (instrumentDescription), select,
    seedSegment: seed('kind', ['deco', 'filter', 'segmentRows']),
    seedInstrument: seed('filter', ['deco', 'instrumentOffered']),
    seedTimeframe: seed('timeframe', ['filter', 'kind', 'tfAll']), source };
}

/** Generated shape fixture; these rows are not market evidence.
 * @param {number} index */
export function generatedMonth(index) {
  const names = ['1min','2min','3min','5min','10min','15min','30min','60min','1day'];
  return { instrument: 'NSE-CASH-FIXTURE' + String(Math.floor(index / 81)).padStart(4, '0'),
    month: '2025-' + String(index % 9 + 1).padStart(2, '0'), timeframe: names[Math.floor(index / 9) % names.length],
    rows: 7000 + index % 100, first_ts: 1735703100000000, last_ts: 1738307940000000,
    chg_bps: index % 2 ? null : 0, chg_why: index % 2 ? 'corporate_action_unverified' : null,
    prev_chg_bps: null, prev_chg_why: 'previous_not_stored' };
}
