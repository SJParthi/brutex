/**
 * Exact decoding for the six-word condition mask on the wire.
 *
 * A partial decode is more dangerous than a refusal: it names a smaller,
 * plausible combination as though that were what won.  The Rust API therefore
 * has one accepted browser representation here — exactly six canonical
 * unsigned decimal strings, each inside `u64` — and every other shape is one
 * visible non-decodable result.
 */

const WORDS = 6;
const U64_MAX = 18_446_744_073_709_551_615n;
const CANONICAL_UNSIGNED_DECIMAL = /^(0|[1-9][0-9]*)$/;

/**
 * @param {unknown} input
 * @returns {{ok: boolean, positions: number[], why: string}}
 */
export function decodeMaskWords(input) {
  if (!Array.isArray(input)) {
    return { ok: false, positions: [], why: 'mask_words is not an array.' };
  }
  if (input.length !== WORDS) {
    return {
      ok: false,
      positions: [],
      why: `mask_words carries ${input.length} words; this format requires exactly ${WORDS}.`
    };
  }

  /** @type {bigint[]} */
  const values = [];
  for (let at = 0; at < WORDS; at += 1) {
    const word = input[at];
    if (typeof word !== 'string' || !CANONICAL_UNSIGNED_DECIMAL.test(word)) {
      return {
        ok: false,
        positions: [],
        why: `mask_words[${at}] is not a canonical unsigned decimal string.`
      };
    }
    const value = BigInt(word);
    if (value > U64_MAX) {
      return {
        ok: false,
        positions: [],
        why: `mask_words[${at}] is greater than the largest unsigned 64-bit integer.`
      };
    }
    values.push(value);
  }

  /** @type {number[]} */
  const positions = [];
  values.forEach((word, index) => {
    let value = word;
    for (let bit = 0; bit < 64 && value !== 0n; bit += 1) {
      if ((value & 1n) === 1n) positions.push(index * 64 + bit);
      value >>= 1n;
    }
  });
  return { ok: true, positions, why: '' };
}
