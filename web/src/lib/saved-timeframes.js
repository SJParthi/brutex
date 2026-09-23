import { CAMPAIGN_RUNGS } from './boolean-campaign.js';

/** Exact saved intraday scope, with omitted fields limited to legacy all-eight
 * responses. Physical indices remain stable when a subset skips a timeframe.
 * @param {any} body
 * @returns {string[]} */
export function savedTimeframes(body) {
  const value = Object.hasOwn(body, 'timeframes') ? body.timeframes : CAMPAIGN_RUNGS;
  if (!Array.isArray(value) || value.length === 0 || value.length > CAMPAIGN_RUNGS.length) {
    throw new Error('Saved research must name a nonempty intraday timeframe selection.');
  }
  let previous = -1;
  for (const rung of value) {
    const index = typeof rung === 'string' ? CAMPAIGN_RUNGS.indexOf(rung) : -1;
    if (index <= previous) {
      throw new Error('Saved timeframes must be unique, supported and in their original order.');
    }
    previous = index;
  }
  return [...value];
}

/** Saved arithmetic versions 1/2 retain the historical all-eight scope; version
 * 3 binds a proper subset. Version 4 binds the added index policy and therefore
 * explicitly records either all eight or a subset without changing statistics.
 * @param {any} body */
export function savedProjectionTimeframes(body) {
  if (![1, 2, 3, 4].includes(body?.projection_version)) throw new Error('Saved search arithmetic version is missing or unsupported.');
  const timeframes = savedTimeframes(body), explicit = Object.hasOwn(body, 'timeframes');
  if (body.projection_version === 4 ? !explicit : body.projection_version === 3
    ? !explicit || timeframes.length === CAMPAIGN_RUNGS.length : timeframes.length !== CAMPAIGN_RUNGS.length) {
    throw new Error('Saved search version does not bind its exact timeframe selection.');
  }
  return timeframes;
}
