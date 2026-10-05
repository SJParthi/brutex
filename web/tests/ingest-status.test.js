// CE-79, D-1787: /ingest dropped every refusal from /ingest/status.json and
// kept a stale flight and halt list after a failed read.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { emptyPilot, failedPilot, foldIngestStatus, pilotNotice } from '../src/lib/ingest-status.js';

const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');

const surveyed = {
  surveyed: true, state: 'running', blocked_by: 'the sweep is pulling NIFTY 2026-01',
  in_flight: { instrument: 'NSE-INDEX-NIFTY', month: '2026-01', index: 1, of: 9 },
  waiting_on: [{ feed: 'dhan', vendor: 'Dhan', halted: 'PERMANENT REFUSAL' }]
};

test('a surveyed answer carries the flight, the halts, surveyed and blocked_by', () => {
  const pilot = foldIngestStatus(200, surveyed, 42);
  assert.equal(pilot.at, 42);
  assert.equal(pilot.surveyed, true);
  assert.equal(pilot.blockedBy, 'the sweep is pulling NIFTY 2026-01');
  assert.equal(pilot.feeds.length, 1);
  assert.equal(pilot.inFlight?.month, '2026-01');
  assert.equal(pilot.error, null);
  assert.equal(pilotNotice(pilot), null);
});

test('surveyed:false is said in the server words, never folded to "nothing halted"', () => {
  const pilot = foldIngestStatus(200, { surveyed: false, state: 'waiting', blocked_by: 'not known here yet. No round has finished', in_flight: null, waiting_on: [] }, 1);
  assert.equal(pilot.surveyed, false);
  const notice = pilotNotice(pilot);
  assert.equal(notice?.tone, 'warn');
  assert.match(notice?.text ?? '', /^not known here yet\. No round has finished/);
  assert.match(notice?.head ?? '', /not nothing outstanding/);
});

test('the 503 poisoned-lock body is named and the previous answer does not survive', () => {
  const before = foldIngestStatus(200, surveyed, 1);
  assert.equal(before.feeds.length, 1);
  const after = foldIngestStatus(503, { surveyed: false, error: "the autopilot's status lock is poisoned ... Restart the server." }, 2);
  assert.equal(after.inFlight, null);
  assert.deepEqual(after.feeds, []);
  assert.equal(after.surveyed, null);
  assert.equal(after.error, "/ingest/status.json answered HTTP 503: the autopilot's status lock is poisoned ... Restart the server.");
  assert.equal(pilotNotice(after)?.tone, 'bad');
  assert.equal(foldIngestStatus(500, undefined, 3).error, '/ingest/status.json answered HTTP 500 and named no reason');
});

test('a 200 without `surveyed` or without a JSON object is a failure, not an empty survey', () => {
  assert.match(foldIngestStatus(200, { waiting_on: [] }, 1).error ?? '', /`surveyed` is missing/);
  assert.match(foldIngestStatus(200, [], 1).error ?? '', /not a JSON object/);
  assert.match(foldIngestStatus(200, undefined, 1).error ?? '', /not a JSON object/);
  assert.equal(failedPilot('x').feeds.length, 0);
  assert.equal(emptyPilot().surveyed, null);
});

test('the page folds through the module, renders the notice and keeps no stale spread on failure', () => {
  assert.match(page, /pilot = foldIngestStatus\(r\.status, body, Date\.now\(\)\)/);
  assert.match(page, /pilot = failedPilot\(/);
  assert.doesNotMatch(page, /pilot = \{ \.\.\.pilot, busy: false, error: String\(why\) \}/);
  assert.match(page, /\{#if pilotSays\}[\s\S]{0,200}role="alert"[\s\S]{0,300}\{pilotSays\.text\}/);
});
