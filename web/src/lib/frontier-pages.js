/**
 * Assemble the complete committed frontier before either ranking surface sees it.
 * These transport ceilings mirror api::detail: at most 4,096 rows, 256 per page.
 * Fetching every row costs O(rows); the fixed cap bounds memory and requests.
 * Any changed receipt facts, missing page, or non-pagination refusal aborts the
 * assembly. The caller still validates every row's financial and mask facts.
 */
export const MAX_FRONTIER_ROWS = 4_096;
export const FRONTIER_PAGE_ROWS = 256;

/** @param {unknown} value */
const count = (value) => Number.isSafeInteger(value) && Number(value) >= 0;

/** @param {any} left @param {any} right */
function sameRules(left, right) {
  if (left === null || right === null) return left === right;
  if (!left || !right || typeof left !== 'object' || typeof right !== 'object' ||
      Array.isArray(left) || Array.isArray(right)) return false;
  const keys = Object.keys(left);
  return keys.length === Object.keys(right).length &&
    keys.every((key) => Object.hasOwn(right, key) && left[key] === right[key]);
}

/**
 * @param {string} identity
 * @param {(url: string) => Promise<{ok: boolean, status: number, json: () => Promise<any>}>} request
 * @returns {Promise<any>}
 */
export async function fetchCompleteFrontier(identity, request) {
  if (!/^[0-9a-f]{64}$/.test(identity)) {
    throw new Error('Frontier pages require one canonical run identity.');
  }
  /** @type {any[]} */
  const rows = [];
  /** @type {any} */
  let first = null;
  let admitted = 0;
  const pages = Math.ceil(MAX_FRONTIER_ROWS / FRONTIER_PAGE_ROWS);
  for (let page = 0; page < pages; page += 1) {
    const response = await request(
      `/frontier.json?identity=${encodeURIComponent(identity)}&page=${page}&limit=${FRONTIER_PAGE_ROWS}`
    );
    if (!response.ok || (response.status !== 200 && response.status !== 206)) {
      throw new Error(`/frontier.json answered ${response.status} on page ${page}.`);
    }
    const body = await response.json();
    const refuse = (/** @type {string} */ why) => {
      throw new Error(`Frontier page ${page} refused: ${why}`);
    };
    if (!body || typeof body !== 'object' || Array.isArray(body)) {
      refuse('the answer is not an object.');
    }
    if (body.identity !== identity) refuse('the run identity changed.');
    if (body.page !== page || body.limit !== FRONTIER_PAGE_ROWS || body.page_complete !== true) {
      refuse('page coordinates, limit, or page_complete do not match the request.');
    }
    if (!count(body.total_count) || body.total_count > MAX_FRONTIER_ROWS) {
      refuse(`total_count must be an exact count from 0 through ${MAX_FRONTIER_ROWS}.`);
    }
    if (!count(body.total_admitted) || body.total_admitted > body.total_count) {
      refuse('total_admitted is not a valid result count.');
    }
    if (first && (body.total_count !== first.total_count ||
        body.total_admitted !== first.total_admitted || !sameRules(first.rules, body.rules))) {
      refuse('result counts or recorded rules changed between pages.');
    }
    const offset = page * FRONTIER_PAGE_ROWS;
    const expectedCount = Math.min(FRONTIER_PAGE_ROWS, body.total_count - offset);
    if (!Array.isArray(body.rows) || !count(body.count) || expectedCount < 0 ||
        body.count !== expectedCount || body.rows.length !== expectedCount) {
      refuse('count or row length does not match the expected page window.');
    }
    if (body.rows.some((/** @type {any} */ row, /** @type {number} */ index) =>
      !row || typeof row !== 'object' || row.rank !== offset + index + 1)) {
      refuse('row ranks are missing, repeated, or out of order.');
    }
    const pageAdmitted = body.rows.filter((/** @type {any} */ row) => row.meets?.all === true).length;
    if (!count(body.admitted) || body.admitted !== pageAdmitted) {
      refuse('admitted does not match this page’s verdicts.');
    }
    const end = offset + expectedCount;
    const wholePage = page === 0 && end === body.total_count;
    if (body.complete !== wholePage || response.status !== (wholePage ? 200 : 206)) {
      refuse('completion or HTTP status contradicts the page window.');
    }
    const expectedNext = end < body.total_count ? page + 1 : null;
    if (body.next_page !== expectedNext) {
      refuse('next_page is missing, cyclic, skipped, or inconsistent with total_count.');
    }
    const paginationNotice = `partial page only: page ${page} returns rows ${offset}..${end} of ${body.total_count}. Fetch every page and reconcile \`total_count\`; this response is not a complete frontier`;
    const explicitAbsence = wholePage && body.total_count === 0 && body.rules === null &&
      body.total_admitted === 0 && typeof body.refusal === 'string' && body.refusal.length > 0;
    if (!explicitAbsence && body.refusal !== (wholePage ? null : paginationNotice)) {
      refuse(typeof body.refusal === 'string' ? body.refusal : 'refusal metadata is missing or invalid.');
    }
    if (!first) first = body;
    rows.push(...body.rows);
    admitted += body.admitted;
    if (expectedNext === null) {
      if (rows.length !== first.total_count || admitted !== first.total_admitted) {
        refuse('assembled rows or admissions do not match the committed totals.');
      }
      return {
        ...first,
        rows,
        count: rows.length,
        admitted,
        page: 0,
        complete: true,
        page_complete: true,
        next_page: null,
        refusal: explicitAbsence ? first.refusal : null
      };
    }
  }
  throw new Error('Frontier pagination exceeded its fixed request bound; no rows were published.');
}
