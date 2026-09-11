/** Validate a bounded native proof, not an ETA or a measured population.
 * This descriptor applies to BITS=all only. The current fixed mask has 384
 * positions, so its decimal lower bound is bounded by 116 digits.
 * @param {any} value */
export function validateBooleanWorkModel(value) {
  const object = value !== null && typeof value === 'object' && !Array.isArray(value);
  if (!object || value.version !== 1) throw new Error('The server search-sizing descriptor is unavailable or unsupported.');
  if (value.state === 'refused') {
    if (Object.keys(value).length !== 3 || typeof value.refusal !== 'string' || value.refusal.trim().length === 0 || value.refusal.length > 4096)
      throw new Error('The search-sizing refusal is malformed.');
    return value;
  }
  const names = ['version', 'state', 'scope', 'live_conditions', 'max_instructions', 'conjunction_program_lower_bound',
    'lower_bound_exceeds_cumulative_counter', 'timeframe_execution', 'total_programs', 'eta_seconds', 'refusal'];
  if (Object.keys(value).length !== names.length || !names.every(name => Object.hasOwn(value, name)) ||
      value.state !== 'lower_bound_only' || value.scope !== 'full_live_alphabet_per_timeframe' ||
      !Number.isInteger(value.live_conditions) || value.live_conditions < 1 || value.live_conditions > 384 ||
      !Number.isSafeInteger(value.max_instructions) || value.max_instructions < 2 * value.live_conditions - 1 ||
      typeof value.conjunction_program_lower_bound !== 'string' || value.conjunction_program_lower_bound.length > 116 ||
      value.conjunction_program_lower_bound !== String((1n << BigInt(value.live_conditions)) - 1n) ||
      value.lower_bound_exceeds_cumulative_counter !== (value.live_conditions > 64) ||
      value.timeframe_execution !== 'sequential' || value.total_programs !== null || value.eta_seconds !== null || value.refusal !== null) {
    throw new Error('The search-sizing descriptor contradicts its exact lower bound or claims an unmeasured total or ETA.');
  }
  return value;
}
