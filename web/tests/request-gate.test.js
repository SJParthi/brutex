import test from 'node:test';
import assert from 'node:assert/strict';

import { createRequestGate } from '../src/lib/request-gate.js';

/** @returns {{promise: Promise<string>, resolve: (value: string) => void, reject: (reason?: unknown) => void}} */
function deferred() {
  /** @type {(value: string) => void} */
  let resolve = () => {};
  /** @type {(reason?: unknown) => void} */
  let reject = () => {};
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function harness() {
  const gate = createRequestGate();
  /** @type {string | undefined} */
  let openIdentity;
  let state = 'idle';

  return {
    /** @param {string} identity */
    open(identity) {
      openIdentity = identity;
    },
    close() {
      openIdentity = undefined;
      gate.invalidate();
      state = 'idle';
    },
    /** @param {string} identity @param {Promise<string>} promise */
    async load(identity, promise) {
      const ticket = gate.begin(identity);
      try {
        const value = await promise;
        if (gate.admits(ticket, openIdentity)) state = value;
      } catch {
        if (gate.admits(ticket, openIdentity)) state = 'failed';
      }
    },
    state() {
      return state;
    }
  };
}

test('a late response for run A cannot overwrite the newer run B', async () => {
  const h = harness();
  const a = deferred();
  const b = deferred();
  h.open('A');
  const loadingA = h.load('A', a.promise);
  h.open('B');
  const loadingB = h.load('B', b.promise);

  b.resolve('rows B');
  await loadingB;
  a.resolve('rows A');
  await loadingA;

  assert.equal(h.state(), 'rows B');
});

test('a response arriving after the drill-down closes leaves it idle', async () => {
  const h = harness();
  const a = deferred();
  h.open('A');
  const loading = h.load('A', a.promise);
  h.close();
  a.resolve('rows A');
  await loading;
  assert.equal(h.state(), 'idle');
});

test('a stale failure cannot replace the current run success', async () => {
  const h = harness();
  const a = deferred();
  const b = deferred();
  h.open('A');
  const loadingA = h.load('A', a.promise);
  h.open('B');
  const loadingB = h.load('B', b.promise);

  b.resolve('rows B');
  await loadingB;
  a.reject(new Error('late A failure'));
  await loadingA;

  assert.equal(h.state(), 'rows B');
});
