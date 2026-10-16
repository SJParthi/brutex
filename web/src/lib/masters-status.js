// The state /mapping holds after `/masters/status.json` refused or failed.
//
// CE-82, D-1788. The page returned on a non-200 without reading the body, so
// the 503's `refusal` ("neither BRUTEX_MASTERS nor HOME is set ...") was
// never shown, the four-file table disappeared with no reason, and
// `restartNeeded` kept whatever an earlier read had said. A refused read
// clears the list, makes the restart flag UNKNOWN (null, never a stale
// true or a guessed false), and carries the sentence to render.

/**
 * @param {string} why the refusal sentence, from `refusalFrom`
 * @returns {{ onDisk: any[], restartNeeded: null, statusWhy: string }}
 */
export function mastersStatusRefused(why) {
  const text = typeof why === 'string' && why.trim() !== ''
    ? why.trim()
    : '/masters/status.json failed and named no reason';
  return { onDisk: [], restartNeeded: null, statusWhy: text };
}
