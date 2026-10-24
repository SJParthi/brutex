/**
 * THE REASONS DICTIONARY, AS A PAGE READS IT (READ-02, R4-05).
 *
 * `/reasons.json` serves `pull::reasons::TABLE`: one row per reason code a
 * page shows, with its plain sentence, what a person does about it and the
 * button or step that does it. A page puts `code: sentence; control` on
 * hover beside the code and adds no paragraph. This file holds NO sentence
 * of its own:
 * every word is the server's, so a code the server's table names is titled
 * the same on every page, and a code it does not name is said to be unnamed,
 * never guessed.
 *
 * ONE FETCH PER PAGE. `loadReasons` asks once; every later call, from any
 * component, gets the same promise. Before the answer lands nothing claims a
 * sentence: `titleFor` answers `null` (the caller shows the code itself),
 * and no page says "loading" (the table is compiled into the server and
 * answered from memory, so the wait is one round trip).
 *
 * THE CONTRACT R4-01 AND R4-04 READ. `titleFor(code)` is synchronous and
 * returns the sentence, or `null` for a code with no row or before the table
 * loaded — never a placeholder sentence (`web/src/lib/ingest-months.js`
 * `reasonWords` relies on it). `actionFor(code)` returns the control's label
 * (`press Repair`, `Retry refused`, `store step S-36`, `operator: <what>`,
 * `no action`) or `null`.
 *
 * A TITLE LONGER THAN THE TABLE'S OWN LONGEST is cut at a word with an
 * ellipsis (`titleText`); `/reasons.json` keeps it whole. The bound is the
 * server's `title_cut`, generated from the table, never a number typed here.
 */

/** The route the table is served on. */
export const REASONS_ROUTE = '/reasons.json';

/** The body shape this page reads. Another word is a server newer than the page, and is refused by name. */
export const REASONS_SCHEMA = 'reasons-v1';

/**
 * @typedef {{ code: string, sentence: string, action: string, control: string, control_kind: string, step: string | null, title: string }} ReasonRow
 * @typedef {{ rows: Map<string, ReasonRow>, cut: number, build: string | null, error: string | null }} ReasonTable
 */

/**
 * A title cut at the last whole word that fits `max` characters, with an
 * ellipsis; the text itself when it fits or `max` is not a positive count.
 *
 * @param {string} text
 * @param {number} max
 */
export function cutTitle(text, max) {
  const chars = Array.from(text);
  if (!Number.isSafeInteger(max) || max <= 0 || chars.length <= max) return text;
  const head = chars.slice(0, max - 1).join('');
  const space = head.lastIndexOf(' ');
  // A first word longer than the bound is cut inside it: never an empty title.
  const kept = space > 0 ? head.slice(0, space) : head;
  return `${kept.trimEnd()}…`;
}

/**
 * A `/reasons.json` body as a table, or the reason it is not one. Pure: a
 * page and a test read a body the same way.
 *
 * @param {any} body
 * @returns {ReasonTable}
 */
export function readReasons(body) {
  /** @type {Map<string, ReasonRow>} */
  const rows = new Map();
  if (!body || typeof body !== 'object') {
    return { rows, cut: 0, build: null, error: 'the reasons table answered no JSON object' };
  }
  if (body.schema !== REASONS_SCHEMA) {
    return {
      rows,
      cut: 0,
      build: typeof body.build === 'string' ? body.build : null,
      error: `unknown reasons schema ${JSON.stringify(body.schema ?? null)}, page older than server`
    };
  }
  for (const row of Array.isArray(body.rows) ? body.rows : []) {
    if (typeof row?.code !== 'string' || typeof row?.sentence !== 'string' || typeof row?.title !== 'string') continue;
    rows.set(row.code, {
      code: row.code,
      sentence: row.sentence,
      action: typeof row.action === 'string' ? row.action : '',
      control: typeof row.control === 'string' ? row.control : '',
      control_kind: typeof row.control_kind === 'string' ? row.control_kind : '',
      step: typeof row.step === 'string' ? row.step : null,
      title: row.title
    });
  }
  const count = Number.isSafeInteger(body.count) ? body.count : null;
  return {
    rows,
    cut: Number.isSafeInteger(body.title_cut) ? body.title_cut : 0,
    build: typeof body.build === 'string' ? body.build : null,
    // A table that kept fewer rows than it says it holds is said, never read
    // as whole.
    error: count !== null && count !== rows.size ? `the reasons table says ${count} rows and holds ${rows.size}` : null
  };
}

/** @type {ReasonTable | null} */
let current = null;
/** @type {Promise<ReasonTable> | null} */
let asked = null;

/**
 * Fetches `/reasons.json` once per page. Every later call returns the same
 * promise; a refused or unreadable answer is a table with no rows and its
 * reason, never a throw and never a second request.
 *
 * @param {(url: string, init?: any) => Promise<Response>} request
 * @returns {Promise<ReasonTable>}
 */
export function loadReasons(request) {
  if (asked) return asked;
  asked = (async () => {
    try {
      const answer = await request(REASONS_ROUTE);
      if (!answer.ok) {
        current = { rows: new Map(), cut: 0, build: null, error: `${REASONS_ROUTE}: HTTP ${answer.status}` };
      } else {
        current = readReasons(await answer.json());
      }
    } catch (why) {
      current = { rows: new Map(), cut: 0, build: null, error: `${REASONS_ROUTE}: ${why instanceof Error ? why.message : String(why)}` };
    }
    return current;
  })();
  return asked;
}

/** Forgets the loaded table, so the next `loadReasons` asks again: a page that reloads, or a test. */
export function forgetReasons() {
  current = null;
  asked = null;
}

/** The loaded table, or `null` before `loadReasons` has answered. */
export function loadedReasons() {
  return current;
}

/**
 * The row of `code` in `table` (the loaded one by default), or `null`.
 *
 * @param {string} code
 * @param {ReasonTable | null} [table]
 */
export function rowFor(code, table = current) {
  return table?.rows.get(code) ?? null;
}

/**
 * The sentence of `code`, or `null` for a code with no row or before the
 * table loaded. One `Map` lookup.
 *
 * @param {string} code
 * @param {ReasonTable | null} [table]
 */
export function titleFor(code, table = current) {
  return rowFor(code, table)?.sentence ?? null;
}

/**
 * The label of the control that acts on `code`, or `null`.
 *
 * @param {string} code
 * @param {ReasonTable | null} [table]
 */
export function actionFor(code, table = current) {
  return rowFor(code, table)?.control ?? null;
}

/**
 * What a person does about `code`, in words, or `null`.
 *
 * @param {string} code
 * @param {ReasonTable | null} [table]
 */
export function actionWordsFor(code, table = current) {
  return rowFor(code, table)?.action ?? null;
}

/**
 * The hover text of `code`: the served `code: sentence; control`, cut at a
 * word past the table's own `title_cut`; `null` for a code with no row.
 *
 * @param {string} code
 * @param {ReasonTable | null} [table]
 */
export function titleText(code, table = current) {
  const row = rowFor(code, table);
  return row === null ? null : cutTitle(row.title, table?.cut ?? 0);
}
