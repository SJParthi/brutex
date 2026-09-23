/**
 * One latest-request-wins gate, scoped to a single independent data stream.
 *
 * A ticket binds both the request generation and the identity it was started
 * for. A response may publish only while both still match. Invalidating is
 * synchronous, so closing a panel wins even when the network promise settles
 * in the same task immediately afterwards.
 */
export function createRequestGate() {
  /** @type {{ identity: string|undefined }|undefined} */
  let current;

  return {
    /** @param {string|undefined} identity */
    begin(identity) {
      current = { identity };
      return current;
    },

    /** @param {{ identity: string|undefined }} ticket @param {string|undefined} openIdentity */
    admits(ticket, openIdentity) {
      return ticket === current && ticket.identity === openIdentity;
    },

    invalidate() {
      current = undefined;
    }
  };
}
